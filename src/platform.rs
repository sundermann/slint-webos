use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::mem::MaybeUninit;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sdl2::event::{Event, EventSender};
use sdl2::{EventPump, EventSubsystem, Sdl, VideoSubsystem, sys};
use slint::platform::{self, Clipboard, EventLoopProxy, Platform, WindowAdapter};
use slint::{EventLoopError, PlatformError};

use crate::window::AppWindow;
use crate::{Settings, hints};

const BATCH_SIZE: usize = 64;
// Video drivers may accept the vsync request and still return immediately.
const FRAME_INTERVAL: Duration = Duration::from_nanos(16_666_667);

pub(crate) fn install(
    settings: Settings,
    extra_hints: &[(&str, Cow<'_, str>)],
) -> Result<(), PlatformError> {
    if !on_main_thread() {
        return Err("initialize slint-webos on the process main thread".into());
    }
    if i_slint_core::context::SlintContext::current().is_some() {
        return Err(PlatformError::SetPlatformError(
            platform::SetPlatformError::AlreadySet,
        ));
    }

    if cfg!(all(target_os = "linux", target_arch = "arm")) {
        sdl2::hint::set("SDL_VIDEODRIVER", "wayland");
        sdl2::hint::set(hints::WEBOS_ACCESS_POLICY_KEYS_BACK, "true");
        sdl2::hint::set(hints::WEBOS_CLOUDGAME_ACTIVE, "false");
    }
    for (name, value) in extra_hints {
        sdl2::hint::set(name, value);
    }

    let sdl = sdl2::init().map_err(|err| format!("initializing SDL: {err}"))?;
    let video = sdl
        .video()
        .map_err(|err| format!("initializing SDL video: {err}"))?;
    if cfg!(all(target_os = "linux", target_arch = "arm"))
        && video.current_video_driver() != "wayland"
    {
        return Err("SDL-webOS requires the Wayland video driver".into());
    }
    let pump = sdl.event_pump()?;
    let events = sdl.event()?;
    // SAFETY: SDL reserves a unique ID. This event carries no pointers or borrowed data.
    let event_type = unsafe { sys::SDL_RegisterEvents(1) };
    if event_type == u32::MAX {
        return Err(format!("registering SDL wake event: {}", sdl2::get_error()).into());
    }
    let queue = Arc::new(Queue {
        messages: Mutex::new(Some(VecDeque::new())),
        wake: events.event_sender(),
        event_type,
    });

    platform::set_platform(Box::new(SdlPlatform {
        window: RefCell::new(Weak::new()),
        running: Cell::new(false),
        settings,
        pump: RefCell::new(pump),
        video: video.clone(),
        queue,
        _events: events,
        _sdl: sdl,
    }))
    .map_err(|err| PlatformError::from(format!("installing Slint platform: {err}")))?;
    video.text_input().stop();
    Ok(())
}

fn on_main_thread() -> bool {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: gettid takes no arguments and getpid has no preconditions.
        unsafe { libc::syscall(libc::SYS_gettid) == libc::c_long::from(libc::getpid()) }
    }
    #[cfg(target_os = "macos")]
    {
        // SAFETY: pthread_main_np has no preconditions.
        unsafe { libc::pthread_main_np() != 0 }
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        true
    }
}

enum Message {
    Invoke(Box<dyn FnOnce() + Send>),
    Quit,
}

struct Queue {
    // None means the SDL event subsystem has been shut down.
    messages: Mutex<Option<VecDeque<Message>>>,
    wake: EventSender,
    event_type: u32,
}

impl Queue {
    fn push(&self, message: Message) -> Result<(), EventLoopError> {
        let mut guard = self
            .messages
            .lock()
            .expect("SDL event queue mutex poisoned");
        let messages = guard.as_mut().ok_or(EventLoopError::EventLoopTerminated)?;
        if messages.is_empty() {
            // Signal before publishing so a failed wake cannot leave the callback queued.
            // The event loop takes this mutex before it reads the queue.
            self.wake
                .push_event(Event::User {
                    timestamp: 0,
                    window_id: 0,
                    type_: self.event_type,
                    code: 0,
                    data1: std::ptr::null_mut(),
                    data2: std::ptr::null_mut(),
                })
                .map_err(|err| {
                    log::warn!("could not wake SDL event loop: {err}");
                    EventLoopError::EventLoopTerminated
                })?;
        }
        messages.push_back(message);
        Ok(())
    }

    fn pop(&self) -> Option<Message> {
        self.messages
            .lock()
            .expect("SDL event queue mutex poisoned")
            .as_mut()
            .and_then(VecDeque::pop_front)
    }

    fn has_messages(&self) -> bool {
        self.messages
            .lock()
            .expect("SDL event queue mutex poisoned")
            .as_ref()
            .is_some_and(|messages| !messages.is_empty())
    }
}

struct Proxy(Arc<Queue>);

impl EventLoopProxy for Proxy {
    fn quit_event_loop(&self) -> Result<(), EventLoopError> {
        self.0.push(Message::Quit)
    }

    fn invoke_from_event_loop(
        &self,
        callback: Box<dyn FnOnce() + Send>,
    ) -> Result<(), EventLoopError> {
        self.0.push(Message::Invoke(callback))
    }
}

struct SdlPlatform {
    window: RefCell<Weak<AppWindow>>,
    running: Cell<bool>,
    settings: Settings,
    pump: RefCell<EventPump>,
    video: VideoSubsystem,
    queue: Arc<Queue>,
    _events: EventSubsystem,
    _sdl: Sdl,
}

impl Drop for SdlPlatform {
    fn drop(&mut self) {
        let pending = self
            .queue
            .messages
            .lock()
            .expect("SDL event queue mutex poisoned")
            .take();
        drop(pending);
    }
}

impl Platform for SdlPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        if self.window.borrow().upgrade().is_some() {
            return Err("slint-webos supports one live top-level window".into());
        }
        let window = AppWindow::new(&self.video, &self.settings)?;
        self.window.replace(Rc::downgrade(&window));
        Ok(window)
    }

    fn new_event_loop_proxy(&self) -> Option<Box<dyn EventLoopProxy>> {
        Some(Box::new(Proxy(Arc::clone(&self.queue))))
    }

    fn set_clipboard_text(&self, text: &str, clipboard: Clipboard) {
        if clipboard == Clipboard::DefaultClipboard
            && let Err(err) = self.video.clipboard().set_clipboard_text(text)
        {
            log::warn!("setting SDL clipboard: {err}");
        }
    }

    fn clipboard_text(&self, clipboard: Clipboard) -> Option<String> {
        if clipboard == Clipboard::DefaultClipboard {
            self.video.clipboard().clipboard_text().ok()
        } else {
            None
        }
    }

    fn run_event_loop(&self) -> Result<(), PlatformError> {
        if self.running.replace(true) {
            return Err("the Slint event loop is already running".into());
        }
        let _running = Running(&self.running);
        let mut received = None;
        let mut next_frame = Instant::now();

        loop {
            // Input and callbacks may start animations. Give them the current tick
            // after a long sleep so the first update cannot skip the animation.
            platform::update_timers_and_animations();
            for _ in 0..BATCH_SIZE {
                match self.queue.pop() {
                    Some(Message::Invoke(callback)) => callback(),
                    Some(Message::Quit) => return Ok(()),
                    None => break,
                }
            }

            let mut events = 0;
            while events < BATCH_SIZE {
                let event = received
                    .take()
                    .or_else(|| self.pump.borrow_mut().poll_event());
                let Some(event) = event else { break };
                events += 1;
                match event {
                    Event::Quit { .. } | Event::AppTerminating { .. } => return Ok(()),
                    event => {
                        let window = self.window.borrow().upgrade();
                        if let Some(window) = window {
                            window.handle_event(event)?;
                        }
                    }
                }
            }

            let window = self.window.borrow().upgrade();
            let frame_wait = if let Some(window) = window {
                let now = Instant::now();
                if now >= next_frame && window.needs_frame() {
                    window.render()?;
                    next_frame = now + FRAME_INTERVAL;
                }
                window
                    .needs_frame()
                    .then(|| next_frame.saturating_duration_since(Instant::now()))
            } else {
                None
            };

            // Native polling may already have consumed a wake event while this
            // iteration was processing native events or painting.
            if events == BATCH_SIZE || self.queue.has_messages() {
                continue;
            }
            let timeout = match (frame_wait, platform::duration_until_next_timer_update()) {
                (Some(frame), Some(timer)) => Some(frame.min(timer)),
                (frame, timer) => frame.or(timer),
            };
            received = self.wait(timeout)?;
        }
    }
}

impl SdlPlatform {
    fn wait(&self, timeout: Option<Duration>) -> Result<Option<Event>, PlatformError> {
        // Borrow the SDL pump to keep all native event access on its owning thread.
        let _pump = self.pump.borrow_mut();
        let millis = timeout.map_or(-1, |duration| {
            duration
                .as_nanos()
                .div_ceil(1_000_000)
                .min(i32::MAX as u128) as i32
        });
        let mut event = MaybeUninit::uninit();
        sdl2::clear_error();
        // SAFETY: SDL writes an SDL_Event when returning 1. The pump is held on the UI thread.
        let status = unsafe { sys::SDL_WaitEventTimeout(event.as_mut_ptr(), millis) };
        if status == 1 {
            // SAFETY: SDL reported a complete event.
            Ok(Some(Event::from_ll(unsafe { event.assume_init() })))
        } else if timeout.is_some() {
            // SDL reports transient backend conditions and elapsed deadlines with the same code.
            Ok(None)
        } else {
            Err(format!("waiting for SDL events: {}", sdl2::get_error()).into())
        }
    }
}

struct Running<'a>(&'a Cell<bool>);

impl Drop for Running<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
