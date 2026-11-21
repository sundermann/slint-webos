use std::cell::RefCell;
use std::error::Error;
use std::ffi::{CStr, c_void};
use std::io;
use std::num::NonZeroU32;
use std::rc::Rc;

use sdl2::VideoSubsystem;
use sdl2::video::{GLContext, GLProfile, SwapInterval, Window};
use slint::PlatformError;
use slint::platform::femtovg_renderer::OpenGLInterface;

use crate::{Settings, WindowMode};

type GlResult = Result<(), Box<dyn Error + Send + Sync>>;

pub(crate) struct Surface {
    // The context must be destroyed before its native window.
    context: GLContext,
    pub(crate) window: RefCell<Window>,
}

impl Surface {
    pub(crate) fn new(
        video: &VideoSubsystem,
        settings: &Settings,
    ) -> Result<Rc<Self>, PlatformError> {
        let attributes = video.gl_attr();
        if cfg!(target_os = "macos") {
            attributes.set_context_profile(GLProfile::Core);
            attributes.set_context_version(3, 3);
        } else {
            attributes.set_context_profile(GLProfile::GLES);
            attributes.set_context_version(3, 0);
        }
        attributes.set_red_size(8);
        attributes.set_green_size(8);
        attributes.set_blue_size(8);
        attributes.set_alpha_size(8);
        attributes.set_depth_size(24);
        attributes.set_stencil_size(8);
        attributes.set_double_buffer(true);

        let mut builder = video.window(&settings.title, settings.size.width, settings.size.height);
        builder.hidden().resizable().opengl().allow_highdpi();
        if settings.mode == WindowMode::Fullscreen {
            builder.fullscreen_desktop();
        } else {
            builder.position_centered();
        }
        let window = builder
            .build()
            .map_err(|err| format!("creating SDL window: {err}"))?;
        let context = window
            .gl_create_context()
            .map_err(|err| format!("creating OpenGL context: {err}"))?;
        window.gl_make_current(&context)?;
        if let Err(err) = video.gl_set_swap_interval(SwapInterval::VSync) {
            log::debug!("SDL swap interval unavailable: {err}");
        }

        Ok(Rc::new(Self {
            context,
            window: RefCell::new(window),
        }))
    }

    pub(crate) fn make_current(&self) -> Result<(), String> {
        self.window.borrow().gl_make_current(&self.context)
    }
}

pub(crate) struct Gl(pub(crate) Rc<Surface>);

// SAFETY: Surface owns SDL's GL context and window for the lifetime of the renderer.
// Rc keeps access on the UI thread. SDL supplies the addresses from its loaded GL library.
unsafe impl OpenGLInterface for Gl {
    fn ensure_current(&self) -> GlResult {
        self.0
            .make_current()
            .map_err(|err| io::Error::other(err).into())
    }

    fn swap_buffers(&self) -> GlResult {
        self.0.window.borrow().gl_swap_window();
        Ok(())
    }

    fn resize(&self, _: NonZeroU32, _: NonZeroU32) -> GlResult {
        // SDL resizes the underlying EGL window in response to compositor configure.
        Ok(())
    }

    fn get_proc_address(&self, name: &CStr) -> *const c_void {
        name.to_str().map_or(std::ptr::null(), |name| {
            self.0
                .window
                .borrow()
                .subsystem()
                .gl_get_proc_address(name)
                .cast()
        })
    }
}
