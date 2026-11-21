//! Exercise the production backend under a real SDL OpenGL window.
use std::cell::Cell;
use std::error::Error;
use std::rc::Rc;
use std::time::{Duration, Instant};

use sdl2::event::{Event, EventSender};
use sdl2::keyboard::{Keycode, Mod};
use sdl2::mouse::{MouseButton, MouseState};
use sdl2::sys;
use slint::{ComponentHandle, GraphicsAPI, PhysicalSize, RenderingState, Timer};
use slint_webos::Settings;

mod ui {
    #![allow(missing_docs)] // Slint generates public accessors.

    slint::slint! {
        #[edition = "1.19"]

        export component Probe inherits Window {
            title: "Backend integration test";
            background: #192840;

            in-out property <int> counter;
            in-out property <string> text <=> edit.text;
            in-out property <length> marker-pos: 10px;
            animate marker-pos { duration: 250ms; }
            public function edit-text() { edit.focus(); }
            forward-focus: keys;

            keys := FocusScope {
                key-pressed(event) => {
                    if event.text == Key.UpArrow { root.counter += 1; return accept; }
                    return reject;
                }
                Rectangle {
                    x: root.marker-pos; y: 10px; width: 30px; height: 30px;
                    background: #f02036;
                }
                TouchArea {
                    x: 30px; y: 70px; width: 100px; height: 60px;
                    clicked => { root.counter += 10; }
                }
                edit := TextInput { x: 30px; y: 150px; width: 220px; height: 40px; }
                Text { x: 30px; y: 220px; text: "System font"; font-size: 24px; color: white; }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    if std::env::var_os("SLINT_WEBOS_GPU_TEST").is_none() {
        return Ok(());
    }
    slint_webos::init_with_settings(Settings {
        size: PhysicalSize::new(640, 480),
        ..Settings::default()
    })?;
    let app = ui::Probe::new()?;
    assert!(ui::Probe::new().is_err(), "second simultaneous window");

    let frames = Rc::new(Cell::new(0u32));
    let rendered = Rc::clone(&frames);
    app.window()
        .set_rendering_notifier(move |state, api: &GraphicsAPI<'_>| {
            if matches!(state, RenderingState::AfterRendering) {
                assert!(matches!(api, GraphicsAPI::NativeOpenGL { .. }), "GPU path");
                rendered.set(rendered.get() + 1);
            }
        })?;

    let completed = Rc::new(Cell::new(false));
    let result = Rc::clone(&completed);
    app.window().on_close_requested(move || {
        assert!(
            result.get(),
            "Back arrived before rendering and input assertions"
        );
        slint::CloseRequestResponse::HideWindow
    });

    let weak = app.as_weak();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        let sent = Instant::now();
        weak.upgrade_in_event_loop(move |app| {
            assert!(
                sent.elapsed() < Duration::from_millis(500),
                "idle SDL wake-up"
            );
            app.set_marker_pos(160.0);
            ready_tx.send(()).expect("test continues running");
        })
    });

    let phase = Rc::new(Cell::new("animation"));
    let watchdog = Rc::clone(&phase);
    Timer::single_shot(Duration::from_secs(8), move || {
        panic!(
            "backend integration test timed out during {}",
            watchdog.get()
        )
    });
    let weak = app.as_weak();
    Timer::single_shot(Duration::from_millis(850), {
        let completed = Rc::clone(&completed);
        move || {
            ready_rx
                .try_recv()
                .expect("worker callback ran on the main thread");
            assert!(
                frames.get() >= 4,
                "animation must cause multiple GPU frames"
            );
            let app = weak.unwrap();
            let shot = app.window().take_snapshot().expect("GPU snapshot");
            assert_eq!((shot.width(), shot.height()), (640, 480));
            let marker = &shot.as_slice()[20 * shot.width() as usize + 170];
            assert!(
                marker.r > 200 && marker.g < 60,
                "animated rectangle was drawn"
            );
            let text_visible = shot
                .as_slice()
                .chunks_exact(shot.width() as usize)
                .skip(220)
                .take(40)
                .any(|row| {
                    row.iter()
                        .any(|pixel| pixel.r > 200 && pixel.g > 200 && pixel.b > 200)
                });
            assert!(text_visible, "system font rendered visible text");

            let sdl = sdl2::init().expect("backend already initialized SDL");
            let id = sdl
                .keyboard()
                .focused_window_id()
                .expect("window has keyboard focus");
            let sender = sdl.event().expect("SDL events").event_sender();
            phase.set("pointer and remote");
            push_key(&sender, id, Keycode::UP);
            push_click(&sender, id);

            Timer::single_shot(Duration::from_millis(100), move || {
                let app = weak.unwrap();
                assert_eq!(app.get_counter(), 11, "remote arrow and pointer click");
                app.invoke_edit_text();
                assert!(sdl.video().expect("SDL video").text_input().is_active());
                phase.set("text commit");
                push_text(id, "é漢");
                push_key(&sender, id, Keycode::A);
                Timer::single_shot(Duration::from_millis(100), move || {
                    assert_eq!(
                        weak.unwrap().get_text(),
                        "é漢",
                        "SDL text input committed once"
                    );
                    // The compositor can send a repeat for the last key as IME
                    // closes. Without a committed text event it must leave text alone.
                    sdl.video().expect("SDL video").text_input().stop();
                    sender
                        .push_event(Event::KeyDown {
                            timestamp: 0,
                            window_id: id,
                            keycode: Some(Keycode::A),
                            scancode: None,
                            keymod: Mod::NOMOD,
                            repeat: true,
                        })
                        .expect("queue stale key repeat");
                    phase.set("resize");
                    weak.unwrap().window().set_size(PhysicalSize::new(760, 540));
                    Timer::single_shot(Duration::from_millis(200), move || {
                        let app = weak.unwrap();
                        assert_eq!(app.get_text(), "é漢", "stale repeat after IME shutdown");
                        let size = app.window().size();
                        assert_eq!((size.width, size.height), (760, 540), "SDL resize");
                        phase.set("Back");
                        completed.set(true);
                        push_key(&sender, id, Keycode::AC_BACK);
                    });
                });
            });
        }
    });
    app.run()?;
    worker.join().expect("worker did not panic")?;
    assert!(completed.get(), "close completed the integration check");
    drop(app);
    ui::Probe::new()?;
    eprintln!("SDL GPU, idle wake, animation, input, IME, resize and close passed");
    Ok(())
}

fn push_key(sender: &EventSender, id: u32, keycode: Keycode) {
    sender
        .push_event(Event::KeyDown {
            timestamp: 0,
            window_id: id,
            keycode: Some(keycode),
            scancode: None,
            keymod: Mod::NOMOD,
            repeat: false,
        })
        .expect("queue SDL key");
    sender
        .push_event(Event::KeyUp {
            timestamp: 0,
            window_id: id,
            keycode: Some(keycode),
            scancode: None,
            keymod: Mod::NOMOD,
            repeat: false,
        })
        .expect("queue SDL key release");
}

fn push_click(sender: &EventSender, id: u32) {
    for event in [
        Event::MouseMotion {
            timestamp: 0,
            window_id: id,
            which: 0,
            mousestate: MouseState::from_sdl_state(0),
            x: 65,
            y: 95,
            xrel: 0,
            yrel: 0,
        },
        Event::MouseButtonDown {
            timestamp: 0,
            window_id: id,
            which: 0,
            mouse_btn: MouseButton::Left,
            clicks: 1,
            x: 65,
            y: 95,
        },
        Event::MouseButtonUp {
            timestamp: 0,
            window_id: id,
            which: 0,
            mouse_btn: MouseButton::Left,
            clicks: 1,
            x: 65,
            y: 95,
        },
    ] {
        sender.push_event(event).expect("queue SDL pointer event");
    }
}

fn push_text(window_id: u32, text: &str) {
    // rust-sdl2 only translates text events from SDL to Rust.
    let mut chars = [0; 32];
    assert!(
        text.len() < chars.len(),
        "SDL text fits and is NUL terminated"
    );
    for (dst, src) in chars.iter_mut().zip(text.bytes()) {
        *dst = src as _;
    }
    let mut event = sys::SDL_Event {
        text: sys::SDL_TextInputEvent {
            type_: sys::SDL_EventType::SDL_TEXTINPUT as u32,
            timestamp: 0,
            windowID: window_id,
            text: chars,
        },
    };
    // SAFETY: The SDL event union contains an initialized SDL_TextInputEvent
    // with inline UTF-8 data. SDL copies the event into its queue.
    assert_eq!(
        unsafe { sys::SDL_PushEvent(&mut event) },
        1,
        "queue SDL text input"
    );
}
