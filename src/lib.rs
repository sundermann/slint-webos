//! Native Slint windows on LG webOS TV, rendered through SDL-webOS and OpenGL ES.
//!
//! Install the platform before constructing any Slint components.
//!
//! ~~~no_run
//! use slint::ComponentHandle;
//!
//! slint::slint! {
//!     #[edition = "1.19"]
//!     export component App inherits Window {
//!         Text { text: "Hello, webOS!"; }
//!     }
//! }
//!
//! fn main() -> Result<(), slint::PlatformError> {
//!     slint_webos::init()?;
//!     App::new()?.run()
//! }
//! ~~~
//!
//! Add a direct dependency on Slint using the Cargo version requirement `"1.19"`.
//! Disable default features and enable `std` and `compat-1-2`. Slint's code
//! generator resolves types through the application's Slint dependency.

mod input;
mod platform;
mod surface;
mod window;

pub use slint;

use slint::{PhysicalSize, PlatformError};

/// How the main application window initially occupies the display.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum WindowMode {
    /// Fill the current display using its compositor-selected size.
    Fullscreen,
    /// Request an independently sized window where the compositor supports it.
    Windowed,
}

/// Initial SDL window configuration.
#[derive(Clone, Debug)]
pub struct Settings {
    /// Window title used when the Slint component does not supply a title.
    pub title: String,
    /// Initial size in SDL window coordinates. The compositor controls the fullscreen size.
    pub size: PhysicalSize,
    /// Initial window mode. Defaults to fullscreen on webOS TV.
    pub mode: WindowMode,
    /// Physical pixels per Slint logical pixel. Defaults to 1.0.
    pub scale_factor: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            title: "Slint".into(),
            size: PhysicalSize::new(1280, 720),
            mode: if cfg!(all(target_os = "linux", target_arch = "arm")) {
                WindowMode::Fullscreen
            } else {
                WindowMode::Windowed
            },
            scale_factor: 1.0,
        }
    }
}

impl Settings {
    fn validate(&self) -> Result<(), PlatformError> {
        if !self.scale_factor.is_finite() || self.scale_factor <= 0.0 {
            return Err("window scale factor must be finite and positive".into());
        }
        if self.size.width == 0
            || self.size.height == 0
            || i32::try_from(self.size.width).is_err()
            || i32::try_from(self.size.height).is_err()
        {
            return Err(
                "window dimensions must be positive and fit in a signed 32-bit integer".into(),
            );
        }
        if self.title.contains('\0') {
            return Err("window title contains a NUL byte".into());
        }
        Ok(())
    }
}

/// Install the default SDL platform before constructing a Slint component.
///
/// Call this on the process main thread. The platform supports one live top-level
/// window at a time.
///
/// # Errors
///
/// Returns an error if SDL is unavailable, the calling thread is unsuitable,
/// or a Slint platform has already been installed.
pub fn init() -> Result<(), PlatformError> {
    init_with_settings(Settings::default())
}

/// Install the SDL platform on the main thread with explicit window settings.
///
/// # Errors
///
/// Returns an error for invalid settings or if platform installation fails.
pub fn init_with_settings(settings: Settings) -> Result<(), PlatformError> {
    settings.validate()?;
    platform::install(settings)
}
