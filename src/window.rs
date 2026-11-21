use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use i_slint_core::InternalToken;
use i_slint_core::cursor::{MouseCursorHandling, MouseCursorInner};
use i_slint_core::items::BuiltInMouseCursor;
use i_slint_core::window::{
    InputMethodProperties, InputMethodRequest, WindowAdapterInternal, WindowInner,
};
use sdl2::VideoSubsystem;
use sdl2::event::{Event, WindowEvent as SdlWindowEvent};
use sdl2::mouse::{Cursor, MouseWheelDirection, SystemCursor};
use sdl2::rect::Rect;
use sdl2::video::{FullscreenType, Window as SdlWindow};
use slint::platform::femtovg_renderer::FemtoVGRenderer;
use slint::platform::{Key, Renderer, WindowAdapter, WindowEvent, WindowProperties};
use slint::{LogicalPosition, PhysicalSize, PlatformError, Window, WindowSize};

use crate::input;
use crate::surface::{Gl, Surface};
use crate::{Settings, WindowMode};

const SCROLL_STEP: f32 = 40.0;

pub(crate) struct AppWindow {
    renderer: FemtoVGRenderer,
    window: Window,
    cursor: RefCell<Option<(BuiltInMouseCursor, Cursor)>>,
    native: Rc<Surface>,
    title: String,
    id: u32,
    geometry: Cell<Geometry>,
    scale: f32,
    initial_mode: Cell<Option<WindowMode>>,
    redraw: Cell<bool>,
    shown: Cell<bool>,
    suspended: Cell<bool>,
    backgrounded: Cell<bool>,
    focused: Cell<bool>,
    ime_cursor: Cell<Option<Rect>>,
}

impl AppWindow {
    pub(crate) fn new(
        video: &VideoSubsystem,
        settings: &Settings,
    ) -> Result<Rc<Self>, PlatformError> {
        let native = Surface::new(video, settings)?;
        let renderer = FemtoVGRenderer::new(Gl(Rc::clone(&native)))?;
        let geometry = Geometry::read(&native.window.borrow());
        let id = native.window.borrow().id();

        Ok(Rc::new_cyclic(|weak: &Weak<Self>| Self {
            renderer,
            native,
            window: Window::new(weak.clone()),
            title: settings.title.clone(),
            id,
            geometry: Cell::new(geometry),
            scale: settings.scale_factor,
            initial_mode: Cell::new(Some(settings.mode)),
            redraw: Cell::new(true),
            shown: Cell::new(false),
            suspended: Cell::new(false),
            backgrounded: Cell::new(false),
            focused: Cell::new(false),
            cursor: RefCell::new(None),
            ime_cursor: Cell::new(None),
        }))
    }

    pub(crate) fn needs_frame(&self) -> bool {
        let size = self.size();
        self.is_foreground()
            && size.width > 0
            && size.height > 0
            && (self.redraw.get() || self.window.has_active_animations())
    }

    fn is_foreground(&self) -> bool {
        self.shown.get() && !self.suspended.get() && !self.backgrounded.get()
    }

    pub(crate) fn render(&self) -> Result<(), PlatformError> {
        self.redraw.set(false);
        self.renderer.render()
    }

    fn resize(&self) -> Result<(), PlatformError> {
        let geometry = Geometry::read(&self.native.window.borrow());
        let previous = self.geometry.replace(geometry);
        if previous.drawable != geometry.drawable
            && geometry.drawable.width > 0
            && geometry.drawable.height > 0
        {
            self.dispatch(WindowEvent::Resized {
                size: geometry.drawable.to_logical(self.scale),
            })?;
            self.request_redraw();
        }
        Ok(())
    }

    fn position(&self, x: i32, y: i32) -> LogicalPosition {
        let (sx, sy) = self.geometry.get().logical_scale(self.scale);
        LogicalPosition::new(x as f32 * sx, y as f32 * sy)
    }

    fn dispatch(&self, event: WindowEvent) -> Result<(), PlatformError> {
        self.window.dispatch_event_with_result(event).map(|_| ())
    }

    pub(crate) fn handle_event(&self, event: Event) -> Result<(), PlatformError> {
        match event {
            Event::AppWillEnterBackground { .. } => {
                self.backgrounded.set(true);
                self.sync_ime();
                return Ok(());
            }
            Event::AppDidEnterForeground { .. } => {
                self.backgrounded.set(false);
                self.sync_ime();
                self.request_redraw();
                return Ok(());
            }
            _ => {}
        }
        if event.get_window_id() != Some(self.id) || !self.shown.get() {
            return Ok(());
        }
        match event {
            Event::Window { win_event, .. } => self.handle_window_event(win_event),
            Event::MouseMotion { x, y, .. } => self.dispatch(WindowEvent::PointerMoved {
                position: self.position(x, y),
            }),
            Event::MouseButtonDown {
                x, y, mouse_btn, ..
            } => self.dispatch(WindowEvent::PointerPressed {
                position: self.position(x, y),
                button: input::button(mouse_btn),
            }),
            Event::MouseButtonUp {
                x, y, mouse_btn, ..
            } => self.dispatch(WindowEvent::PointerReleased {
                position: self.position(x, y),
                button: input::button(mouse_btn),
            }),
            Event::MouseWheel {
                precise_x,
                precise_y,
                direction,
                mouse_x,
                mouse_y,
                ..
            } => {
                let step = if direction == MouseWheelDirection::Flipped {
                    -SCROLL_STEP
                } else {
                    SCROLL_STEP
                };
                self.dispatch(WindowEvent::PointerScrolled {
                    position: self.position(mouse_x, mouse_y),
                    delta_x: precise_x * step,
                    delta_y: precise_y * step,
                })
            }
            Event::KeyDown {
                keycode: Some(code),
                keymod,
                repeat,
                ..
            }
            | Event::KeyUp {
                keycode: Some(code),
                keymod,
                repeat,
                ..
            } => {
                let pressed = matches!(event, Event::KeyDown { .. });
                if input::is_back(code) {
                    if pressed && !repeat {
                        return self.back();
                    }
                    return Ok(());
                }
                // Keep a stale compositor repeat from inserting text while the native
                // keyboard is closing. Deliberate text and repeats arrive as TextInput.
                let text_via_ime = self.ime_cursor.get().is_some()
                    && (repeat
                        || self
                            .native
                            .window
                            .borrow()
                            .subsystem()
                            .text_input()
                            .is_active());
                if let Some(event) = input::key_event(code, keymod, pressed, repeat, text_via_ime) {
                    self.dispatch(event)?;
                }
                Ok(())
            }
            Event::TextInput { text, .. } if self.ime_cursor.get().is_some() => {
                self.dispatch(input::commit(text))
            }
            Event::TextEditing {
                text,
                start,
                length,
                ..
            } if self.ime_cursor.get().is_some() => {
                self.dispatch(input::preedit(text, start, length))
            }
            _ => Ok(()),
        }
    }

    fn back(&self) -> Result<(), PlatformError> {
        // Slint consumes Escape even when there is no popup to dismiss.
        // Send a close request in that case so app navigation can veto it.
        let popups_open = !WindowInner::from_pub(&self.window)
            .active_popups()
            .is_empty();
        if popups_open {
            self.dispatch(WindowEvent::KeyPressed {
                text: Key::Escape.into(),
            })
        } else {
            self.dispatch(WindowEvent::CloseRequested)
        }
    }

    fn handle_window_event(&self, event: SdlWindowEvent) -> Result<(), PlatformError> {
        match event {
            SdlWindowEvent::Close => self.dispatch(WindowEvent::CloseRequested),
            SdlWindowEvent::Leave => self.dispatch(WindowEvent::PointerExited),
            SdlWindowEvent::FocusGained | SdlWindowEvent::FocusLost => {
                let active = event == SdlWindowEvent::FocusGained;
                self.focused.set(active);
                self.sync_ime();
                self.dispatch(WindowEvent::WindowActiveChanged(active))
            }
            SdlWindowEvent::Shown | SdlWindowEvent::Restored | SdlWindowEvent::Maximized => {
                self.suspended.set(false);
                self.sync_ime();
                self.request_redraw();
                self.resize()
            }
            SdlWindowEvent::Hidden | SdlWindowEvent::Minimized => {
                self.suspended.set(true);
                self.sync_ime();
                Ok(())
            }
            SdlWindowEvent::SizeChanged(..)
            | SdlWindowEvent::Resized(..)
            | SdlWindowEvent::DisplayChanged(..) => self.resize(),
            SdlWindowEvent::Exposed => {
                self.request_redraw();
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn update_ime(&self, properties: &InputMethodProperties) {
        let (sx, sy) = self.geometry.get().logical_scale(self.scale);
        self.ime_cursor.set(Some(Rect::new(
            (properties.cursor_rect_origin.x / sx).round() as i32,
            (properties.cursor_rect_origin.y / sy).round() as i32,
            (properties.cursor_rect_size.width / sx).ceil() as u32,
            (properties.cursor_rect_size.height / sy).ceil() as u32,
        )));
        self.sync_ime();
    }

    fn sync_ime(&self) {
        let native = self.native.window.borrow();
        let ime = native.subsystem().text_input();
        if let Some(rect) = self
            .ime_cursor
            .get()
            .filter(|_| self.focused.get() && self.is_foreground())
        {
            ime.set_rect(rect);
            if !ime.is_active() {
                ime.start();
            }
        } else if ime.is_active() {
            ime.stop();
        }
    }
}

impl WindowAdapter for AppWindow {
    fn window(&self) -> &Window {
        &self.window
    }

    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }

    fn size(&self) -> PhysicalSize {
        self.geometry.get().drawable
    }

    fn request_redraw(&self) {
        self.redraw.set(true);
    }

    fn set_visible(&self, visible: bool) -> Result<(), PlatformError> {
        if visible {
            self.native.window.borrow_mut().show();
            self.shown.set(true);
            self.suspended.set(false);
            self.focused
                .set(self.native.window.borrow().has_input_focus());
            if self.initial_mode.take() == Some(WindowMode::Fullscreen) {
                self.window.set_fullscreen(true);
            }
            self.dispatch(WindowEvent::ScaleFactorChanged {
                scale_factor: self.scale,
            })?;
            self.resize()?;
            self.sync_ime();
            self.request_redraw();
        } else {
            self.shown.set(false);
            self.sync_ime();
            self.native.window.borrow_mut().hide();
        }
        Ok(())
    }

    fn set_size(&self, size: WindowSize) {
        let physical = size.to_physical(self.scale);
        let (width, height) = self.geometry.get().physical_to_native(physical);
        let result = {
            let mut native = self.native.window.borrow_mut();
            if native.fullscreen_state() != FullscreenType::Off || native.size() == (width, height)
            {
                return;
            }
            native.set_size(width, height)
        };
        if let Err(err) = result {
            log::warn!("resizing SDL window: {err}");
        }
        // SDL delivers SizeChanged when the compositor applies the new size.
    }

    fn update_window_properties(&self, properties: WindowProperties<'_>) {
        let title = properties.title();
        let title = if title.is_empty() {
            &self.title
        } else {
            title.as_str()
        };
        let mut native = self.native.window.borrow_mut();
        if native.title() != title
            && let Err(err) = native.set_title(title)
        {
            log::warn!("setting SDL window title: {err}");
        }
        if self.initial_mode.get() == Some(WindowMode::Fullscreen) {
            return;
        }
        let fullscreen = if properties.is_fullscreen() {
            FullscreenType::Desktop
        } else {
            FullscreenType::Off
        };
        if native.fullscreen_state() != fullscreen
            && let Err(err) = native.set_fullscreen(fullscreen)
        {
            log::warn!("changing SDL fullscreen mode: {err}");
        }
    }

    fn internal(&self, _: InternalToken) -> Option<&dyn WindowAdapterInternal> {
        Some(self)
    }
}

impl WindowAdapterInternal for AppWindow {
    fn input_method_request(&self, request: InputMethodRequest) {
        match request {
            InputMethodRequest::Enable(properties) | InputMethodRequest::Update(properties) => {
                self.update_ime(&properties);
            }
            InputMethodRequest::Disable => {
                self.ime_cursor.set(None);
                self.sync_ime();
            }
            _ => {}
        }
    }

    fn set_mouse_cursor(&self, shape: MouseCursorInner) -> MouseCursorHandling {
        use BuiltInMouseCursor as C;
        let native = self.native.window.borrow();
        let mouse = native.subsystem().sdl().mouse();
        let shape = match shape {
            MouseCursorInner::BuiltIn(shape) => shape,
            MouseCursorInner::CustomMouseCursor {
                image,
                hotspot_x,
                hotspot_y,
            } => {
                mouse.show_cursor(false);
                return MouseCursorHandling::Render {
                    image,
                    hotspot_x,
                    hotspot_y,
                };
            }
            _ => C::Default,
        };
        mouse.show_cursor(shape != C::None);
        if shape == C::None
            || self
                .cursor
                .borrow()
                .as_ref()
                .is_some_and(|(active, _)| *active == shape)
        {
            return MouseCursorHandling::Platform;
        }
        let kind = match shape {
            C::Pointer => SystemCursor::Hand,
            C::Text => SystemCursor::IBeam,
            C::Wait | C::Progress => SystemCursor::Wait,
            C::Crosshair => SystemCursor::Crosshair,
            C::Move => SystemCursor::SizeAll,
            C::NotAllowed => SystemCursor::No,
            _ => SystemCursor::Arrow,
        };
        if let Ok(cursor) = Cursor::from_system(kind) {
            cursor.set();
            self.cursor.replace(Some((shape, cursor)));
        }
        MouseCursorHandling::Platform
    }
}

#[derive(Clone, Copy)]
struct Geometry {
    drawable: PhysicalSize,
    native: (u32, u32),
}

impl Geometry {
    fn read(window: &SdlWindow) -> Self {
        let (width, height) = window.drawable_size();
        Self {
            drawable: PhysicalSize::new(width, height),
            native: window.size(),
        }
    }

    fn logical_scale(self, scale: f32) -> (f32, f32) {
        (
            self.drawable.width.max(1) as f32 / self.native.0.max(1) as f32 / scale,
            self.drawable.height.max(1) as f32 / self.native.1.max(1) as f32 / scale,
        )
    }

    fn physical_to_native(self, physical: PhysicalSize) -> (u32, u32) {
        let (x, y) = self.logical_scale(1.0);
        let dimension = |size: u32, scale: f32| {
            ((size as f32 / scale).round() as u32).clamp(1, i32::MAX as u32)
        };
        (dimension(physical.width, x), dimension(physical.height, y))
    }
}
