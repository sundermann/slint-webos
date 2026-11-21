use std::ops::Range;

use i_slint_core::input::{InternalKeyEvent, KeyEventType};
use sdl2::keyboard::{Keycode, Mod};
use sdl2::mouse::MouseButton;
use slint::SharedString;
use slint::platform::{Key, PointerEventButton, WindowEvent};

// SDL-webOS assigns scancode 482 in its fork of SDL_scancode.h.
const WEBOS_BACK: i32 = (1 << 30) | 482;

pub(crate) fn key_event(
    code: Keycode,
    modifiers: Mod,
    pressed: bool,
    repeat: bool,
    text_via_ime: bool,
) -> Option<WindowEvent> {
    let mut event = InternalKeyEvent {
        event_type: if pressed {
            KeyEventType::KeyPressed
        } else {
            KeyEventType::KeyReleased
        },
        ..Default::default()
    };
    let state = &mut event.key_event.modifiers;
    state.shift = modifiers.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
    state.control = modifiers.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD);
    state.alt = modifiers.intersects(Mod::LALTMOD | Mod::RALTMOD);
    state.meta = modifiers.intersects(Mod::LGUIMOD | Mod::RGUIMOD);
    // SDL delivers printable text separately while the input method is active.
    let shortcut = (state.control || state.alt || state.meta) && !modifiers.contains(Mod::MODEMOD);
    event.key_event.text = key_text(code, !text_via_ime || shortcut)?;
    event.key_event.repeat = repeat;
    Some(WindowEvent::internal(event))
}

fn key_text(code: Keycode, printable: bool) -> Option<SharedString> {
    use Keycode as K;
    let key = match code {
        K::RETURN | K::KP_ENTER => Key::Return,
        K::BACKSPACE | K::KP_BACKSPACE => Key::Backspace,
        K::DELETE => Key::Delete,
        K::INSERT => Key::Insert,
        K::TAB => Key::Tab,
        K::UP => Key::UpArrow,
        K::DOWN => Key::DownArrow,
        K::LEFT => Key::LeftArrow,
        K::RIGHT => Key::RightArrow,
        K::HOME => Key::Home,
        K::END => Key::End,
        K::PAGEUP => Key::PageUp,
        K::PAGEDOWN => Key::PageDown,
        K::LSHIFT | K::RSHIFT => Key::Shift,
        K::LCTRL | K::RCTRL => Key::Control,
        K::LALT | K::RALT => Key::Alt,
        K::LGUI | K::RGUI => Key::Meta,
        K::F1 => Key::F1,
        K::F2 => Key::F2,
        K::F3 => Key::F3,
        K::F4 => Key::F4,
        K::F5 => Key::F5,
        K::F6 => Key::F6,
        K::F7 => Key::F7,
        K::F8 => Key::F8,
        K::F9 => Key::F9,
        K::F10 => Key::F10,
        K::F11 => Key::F11,
        K::F12 => Key::F12,
        _ if printable => {
            let ch = char::from_u32(u32::try_from(i32::from(code)).ok()?)?;
            return (!ch.is_control()).then(|| ch.to_string().into());
        }
        _ => return None,
    };
    Some(key.into())
}

pub(crate) fn is_back(code: Keycode) -> bool {
    matches!(code, Keycode::AC_BACK | Keycode::ESCAPE) || i32::from(code) == WEBOS_BACK
}

pub(crate) fn preedit(text: String, start: i32, length: i32) -> WindowEvent {
    let position = |index: i32| {
        let max = text.len();
        let index = usize::try_from(index.max(0)).unwrap_or(usize::MAX);
        text.char_indices()
            .nth(index)
            .map_or(max, |(offset, _)| offset) as i32
    };
    let selection: Option<Range<i32>> = if start >= 0 {
        Some(position(start)..position(start.saturating_add(length.max(0))))
    } else {
        None
    };
    WindowEvent::internal(InternalKeyEvent {
        event_type: KeyEventType::UpdateComposition,
        preedit_text: text.into(),
        preedit_selection: selection,
        ..Default::default()
    })
}

pub(crate) fn commit(text: String) -> WindowEvent {
    let mut event = InternalKeyEvent {
        event_type: KeyEventType::CommitComposition,
        ..Default::default()
    };
    event.key_event.text = text.into();
    WindowEvent::internal(event)
}

pub(crate) fn button(button: MouseButton) -> PointerEventButton {
    match button {
        MouseButton::Left => PointerEventButton::Left,
        MouseButton::Middle => PointerEventButton::Middle,
        MouseButton::Right => PointerEventButton::Right,
        MouseButton::X1 => PointerEventButton::Back,
        MouseButton::X2 => PointerEventButton::Forward,
        MouseButton::Unknown => PointerEventButton::Other,
    }
}
