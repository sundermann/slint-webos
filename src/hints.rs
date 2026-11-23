//! Typed SDL-webOS hints and their names in SDL's public header.
//!
//! Pass [`Hint`] values to [`crate::init_with_settings_and_hints`]. Use
//! [`Hint::Custom`] for additional SDL hints. The string constants give the
//! SDL names and values for apps calling SDL directly.

use std::borrow::Cow;
use std::fmt::Write as _;
use std::num::NonZeroU32;
use std::time::Duration;

use slint::PlatformError;

/// Allows the app to receive the remote's Back key.
pub const WEBOS_ACCESS_POLICY_KEYS_BACK: &str = "SDL_WEBOS_ACCESS_POLICY_KEYS_BACK";

/// Allows the app to handle Exit, which is a long press of Back on a Magic Remote.
pub const WEBOS_ACCESS_POLICY_KEYS_EXIT: &str = "SDL_WEBOS_ACCESS_POLICY_KEYS_EXIT";

/// Allows the app to handle the Home key.
pub const WEBOS_ACCESS_POLICY_KEYS_HOME: &str = "SDL_WEBOS_ACCESS_POLICY_KEYS_HOME";

/// Allows the app to handle the TV Guide key.
pub const WEBOS_ACCESS_POLICY_KEYS_GUIDE: &str = "SDL_WEBOS_ACCESS_POLICY_KEYS_GUIDE";

/// Allows the app to handle the Meta key on webOS 8 and later.
pub const WEBOS_ACCESS_POLICY_KEYS_META: &str = "SDL_WEBOS_ACCESS_POLICY_KEYS_META";

/// Controls whether webOS can show its home launcher over the app.
pub const WEBOS_ACCESS_POLICY_RIBBON: &str = "SDL_WEBOS_ACCESS_POLICY_RIBBON";

/// Controls whether system key actions can launch another app over this app.
pub const WEBOS_ACCESS_POLICY_KEYACTION_LAUNCH_APP: &str =
    "SDL_WEBOS_ACCESS_POLICY_KEYACTION_LAUNCH_APP";

/// Sets the idle timeout before hiding the cursor, in milliseconds.
pub const WEBOS_CURSOR_SLEEP_TIME: &str = "SDL_WEBOS_CURSOR_SLEEP_TIME";

/// Sets the cursor update rate in frames per second.
pub const WEBOS_CURSOR_FREQUENCY: &str = "SDL_WEBOS_CURSOR_FREQUENCY";

/// Controls whether cursor calibration is disabled.
pub const WEBOS_CURSOR_CALIBRATION_DISABLE: &str = "SDL_WEBOS_CURSOR_CALIBRATION_DISABLE";

/// Controls whether SDL registers the app automatically through Luna Service.
pub const WEBOS_REGISTER_APP: &str = "SDL_WEBOS_REGISTER_APP";

/// Sets the native webOS window class.
pub const WEBOS_WINDOW_CLASS: &str = "SDL_WEBOS_WINDOW_CLASS";

/// The SDL hint value for [`WindowClass::Normal`].
pub const WEBOS_WINDOW_CLASS_NORMAL: &str = "0";

/// The SDL hint value for [`WindowClass::Hidden`].
pub const WEBOS_WINDOW_CLASS_HIDDEN: &str = "1";

/// Sets the native webOS window type when SDL creates the window.
pub const WEBOS_WINDOW_TYPE: &str = "SDL_WEBOS_WINDOW_TYPE";

/// The SDL hint value for [`WindowType::Card`].
pub const WEBOS_WINDOW_TYPE_CARD: &str = "_WEBOS_WINDOW_TYPE_CARD";

/// The SDL hint value for [`WindowType::Overlay`].
pub const WEBOS_WINDOW_TYPE_OVERLAY: &str = "_WEBOS_WINDOW_TYPE_OVERLAY";

/// The SDL hint value for [`WindowType::Popup`].
pub const WEBOS_WINDOW_TYPE_POPUP: &str = "_WEBOS_WINDOW_TYPE_POPUP";

/// The SDL hint value for [`WindowType::Floating`].
pub const WEBOS_WINDOW_TYPE_FLOATING: &str = "_WEBOS_WINDOW_TYPE_FLOATING";

/// The SDL hint value for [`WindowType::Home`].
pub const WEBOS_WINDOW_TYPE_HOME: &str = "_WEBOS_WINDOW_TYPE_HOME";

/// The SDL hint value for [`WindowType::Keyboard`].
pub const WEBOS_WINDOW_TYPE_KEYBOARD: &str = "_WEBOS_WINDOW_TYPE_KEYBOARD";

/// The SDL hint value for [`WindowType::Restricted`].
pub const WEBOS_WINDOW_TYPE_RESTRICTED: &str = "_WEBOS_WINDOW_TYPE_RESTRICTED";

/// The SDL hint value for [`WindowType::ScreenSaver`].
pub const WEBOS_WINDOW_TYPE_SCREENSAVER: &str = "_WEBOS_WINDOW_TYPE_SCREENSAVER";

/// The SDL hint value for [`WindowType::SystemUi`].
pub const WEBOS_WINDOW_TYPE_SYSTEM_UI: &str = "_WEBOS_WINDOW_TYPE_SYSTEM_UI";

/// Controls the `cloudgame_active` window property for gamepad input.
pub const WEBOS_CLOUDGAME_ACTIVE: &str = "SDL_WEBOS_CLOUDGAME_ACTIVE";

/// Excludes Bluetooth devices from HIDAPI enumeration by vendor and product ID.
pub const WEBOS_HIDAPI_IGNORE_BLUETOOTH_DEVICES: &str = "SDL_WEBOS_HIDAPI_IGNORE_BLUETOOTH_DEVICES";

/// An SDL hint to apply during platform initialization.
///
/// The built-in variants accept values of the appropriate type.
///
/// ```compile_fail
/// use slint_webos::hints::Hint;
///
/// let hint = Hint::AccessPolicyKeysBack("maybe");
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Hint<'a> {
    /// Controls whether the app receives the remote's Back key.
    AccessPolicyKeysBack(bool),
    /// Controls whether the app receives Exit, a long press of Back on a Magic Remote.
    AccessPolicyKeysExit(bool),
    /// Controls whether the app receives the Home key.
    AccessPolicyKeysHome(bool),
    /// Controls whether the app receives the TV Guide key.
    AccessPolicyKeysGuide(bool),
    /// Controls whether the app receives the Meta key on webOS 8 and later.
    AccessPolicyKeysMeta(bool),
    /// Controls whether webOS can display its home launcher over the app.
    AccessPolicyRibbon(bool),
    /// Controls whether system key actions can launch another app over this app.
    AccessPolicyKeyActionLaunchApp(bool),
    /// Sets the cursor idle timeout in whole milliseconds up to `i32::MAX`.
    CursorSleepTime(Duration),
    /// Sets the cursor update rate in frames per second up to `i32::MAX`.
    CursorFrequency(NonZeroU32),
    /// Controls whether cursor calibration is disabled.
    CursorCalibrationDisable(bool),
    /// Controls whether SDL registers the app automatically through Luna Service.
    RegisterApp(bool),
    /// Sets the native webOS window class.
    WindowClass(WindowClass),
    /// Sets the native webOS window type when SDL creates the window.
    WindowType(WindowType),
    /// Controls the `cloudgame_active` window property for gamepad input.
    CloudgameActive(bool),
    /// Excludes listed Bluetooth devices from HIDAPI enumeration.
    HidapiIgnoreBluetoothDevices(&'a [BluetoothDeviceId]),
    /// Passes a raw hint value to SDL unchanged, including for built-in keys.
    Custom {
        /// The SDL hint name without NUL bytes.
        name: &'a str,
        /// The hint value without NUL bytes.
        value: &'a str,
    },
}

/// A native webOS window class used by the compositor's Recents view.
///
/// Luna Surface Manager defines both classes. For these values, SDL-webOS
/// sends `"1"` as the class property. The value `"0"` lets the compositor
/// supply its default.
/// [Luna Surface Manager](https://github.com/webosose/luna-surfacemanager/blob/b2fef6f73232244f6f13102aa66d2d3f5942835e/modules/weboscompositor/webossurfaceitem.h#L119-L128)
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum WindowClass {
    /// Uses the compositor's default card behavior.
    Normal,
    /// Hides the window from Recents.
    Hidden,
}

impl WindowClass {
    /// Returns the value for the `SDL_WEBOS_WINDOW_CLASS` hint.
    #[must_use]
    pub const fn as_sdl_value(self) -> &'static str {
        match self {
            Self::Normal => WEBOS_WINDOW_CLASS_NORMAL,
            Self::Hidden => WEBOS_WINDOW_CLASS_HIDDEN,
        }
    }
}

/// A native webOS window type recognized by a compositor surface model.
///
/// Availability depends on the TV firmware and its window policy. Use
/// [`Hint::Custom`] for additional firmware-specific values.
/// [webOS window type reference](https://www.webosose.org/docs/guides/core-topics/graphics-input/graphics-input-overview/#window-types)
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum WindowType {
    /// The standard full-screen application window.
    Card,
    /// A window over live TV or the current input.
    Overlay,
    /// A popup above full-screen and overlay windows.
    Popup,
    /// A surface shown by the TV's floating window view.
    Floating,
    /// A surface shown by the TV's Home view.
    Home,
    /// A surface shown by the compositor's on-screen keyboard view.
    Keyboard,
    /// A full-screen window with compositor restrictions on overlays.
    Restricted,
    /// A surface shown by the TV's screen saver view.
    ScreenSaver,
    /// A system UI surface handled by the compositor's app-specific system UI model.
    SystemUi,
}

impl WindowType {
    /// Returns the value for the `SDL_WEBOS_WINDOW_TYPE` hint.
    #[must_use]
    pub const fn as_sdl_value(self) -> &'static str {
        match self {
            Self::Card => WEBOS_WINDOW_TYPE_CARD,
            Self::Overlay => WEBOS_WINDOW_TYPE_OVERLAY,
            Self::Popup => WEBOS_WINDOW_TYPE_POPUP,
            Self::Floating => WEBOS_WINDOW_TYPE_FLOATING,
            Self::Home => WEBOS_WINDOW_TYPE_HOME,
            Self::Keyboard => WEBOS_WINDOW_TYPE_KEYBOARD,
            Self::Restricted => WEBOS_WINDOW_TYPE_RESTRICTED,
            Self::ScreenSaver => WEBOS_WINDOW_TYPE_SCREENSAVER,
            Self::SystemUi => WEBOS_WINDOW_TYPE_SYSTEM_UI,
        }
    }
}

/// A Bluetooth HID vendor and product ID in the ignore list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BluetoothDeviceId {
    /// The 16-bit vendor ID.
    pub vendor_id: u16,
    /// The 16-bit product ID. Zero matches every product from this vendor.
    pub product_id: u16,
}

impl<'a> Hint<'a> {
    pub(crate) fn encode(self) -> Result<(&'a str, Cow<'a, str>), PlatformError> {
        let pair = match self {
            Self::AccessPolicyKeysBack(value) => (WEBOS_ACCESS_POLICY_KEYS_BACK, boolean(value)),
            Self::AccessPolicyKeysExit(value) => (WEBOS_ACCESS_POLICY_KEYS_EXIT, boolean(value)),
            Self::AccessPolicyKeysHome(value) => (WEBOS_ACCESS_POLICY_KEYS_HOME, boolean(value)),
            Self::AccessPolicyKeysGuide(value) => (WEBOS_ACCESS_POLICY_KEYS_GUIDE, boolean(value)),
            Self::AccessPolicyKeysMeta(value) => (WEBOS_ACCESS_POLICY_KEYS_META, boolean(value)),
            Self::AccessPolicyRibbon(value) => (WEBOS_ACCESS_POLICY_RIBBON, boolean(value)),
            Self::AccessPolicyKeyActionLaunchApp(value) => {
                (WEBOS_ACCESS_POLICY_KEYACTION_LAUNCH_APP, boolean(value))
            }
            Self::CursorSleepTime(value) => {
                // SDL-webOS parses this with strtol, which uses a 32-bit long on the TV.
                let millis = i32::try_from(value.as_millis()).map_err(
                    |_| "SDL hint SDL_WEBOS_CURSOR_SLEEP_TIME exceeds i32::MAX milliseconds",
                )?;
                if !value.subsec_nanos().is_multiple_of(1_000_000) {
                    return Err(
                        "SDL hint SDL_WEBOS_CURSOR_SLEEP_TIME requires whole milliseconds".into(),
                    );
                }
                (WEBOS_CURSOR_SLEEP_TIME, Cow::Owned(millis.to_string()))
            }
            Self::CursorFrequency(value) => {
                let fps = i32::try_from(value.get())
                    .map_err(|_| "SDL hint SDL_WEBOS_CURSOR_FREQUENCY exceeds i32::MAX FPS")?;
                (WEBOS_CURSOR_FREQUENCY, Cow::Owned(fps.to_string()))
            }
            Self::CursorCalibrationDisable(value) => {
                (WEBOS_CURSOR_CALIBRATION_DISABLE, boolean(value))
            }
            Self::RegisterApp(value) => (WEBOS_REGISTER_APP, boolean(value)),
            Self::WindowClass(value) => (WEBOS_WINDOW_CLASS, Cow::Borrowed(value.as_sdl_value())),
            Self::WindowType(value) => (WEBOS_WINDOW_TYPE, Cow::Borrowed(value.as_sdl_value())),
            Self::CloudgameActive(value) => (WEBOS_CLOUDGAME_ACTIVE, boolean(value)),
            Self::HidapiIgnoreBluetoothDevices(devices) => {
                let mut value = String::new();
                for (index, device) in devices.iter().enumerate() {
                    if index != 0 {
                        value.push(',');
                    }
                    write!(
                        value,
                        "0x{:04x}/0x{:04x}",
                        device.vendor_id, device.product_id
                    )
                    .expect("writing to a String is infallible");
                }
                (WEBOS_HIDAPI_IGNORE_BLUETOOTH_DEVICES, Cow::Owned(value))
            }
            Self::Custom { name, value } => {
                // SDL's C strings cannot represent embedded NUL bytes.
                if name.contains('\0') || value.contains('\0') {
                    return Err(format!("SDL hint {name:?} contains a NUL byte").into());
                }
                (name, Cow::Borrowed(value))
            }
        };
        Ok(pair)
    }
}

fn boolean(value: bool) -> Cow<'static, str> {
    Cow::Borrowed(if value { "true" } else { "false" })
}

#[cfg(test)]
mod tests {
    use super::{Hint, WindowClass, WindowType};

    #[test]
    fn window_hints_encode_compositor_values() {
        for (class, expected) in [(WindowClass::Normal, "0"), (WindowClass::Hidden, "1")] {
            let (name, value) = Hint::WindowClass(class)
                .encode()
                .expect("typed window class");
            assert_eq!((name, value.as_ref()), ("SDL_WEBOS_WINDOW_CLASS", expected));
        }

        for (window_type, expected) in [
            (WindowType::Card, "_WEBOS_WINDOW_TYPE_CARD"),
            (WindowType::Overlay, "_WEBOS_WINDOW_TYPE_OVERLAY"),
            (WindowType::Popup, "_WEBOS_WINDOW_TYPE_POPUP"),
            (WindowType::Floating, "_WEBOS_WINDOW_TYPE_FLOATING"),
            (WindowType::Home, "_WEBOS_WINDOW_TYPE_HOME"),
            (WindowType::Keyboard, "_WEBOS_WINDOW_TYPE_KEYBOARD"),
            (WindowType::Restricted, "_WEBOS_WINDOW_TYPE_RESTRICTED"),
            (WindowType::ScreenSaver, "_WEBOS_WINDOW_TYPE_SCREENSAVER"),
            (WindowType::SystemUi, "_WEBOS_WINDOW_TYPE_SYSTEM_UI"),
        ] {
            let (name, value) = Hint::WindowType(window_type)
                .encode()
                .expect("typed window type");
            assert_eq!((name, value.as_ref()), ("SDL_WEBOS_WINDOW_TYPE", expected));
        }
    }
}
