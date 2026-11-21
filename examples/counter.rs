//! A single-window counter with remote, pointer, and text input.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use slint::{ComponentHandle, PlatformError, Timer};

mod ui {
    #![allow(missing_docs)] // The Slint macro generates public Rust accessors.

    slint::slint! {
        #[edition = "1.19"]
        import { Button, LineEdit } from "std-widgets.slint";

        export component Counter inherits Window {
            title: "Slint Counter";
            background: #151b27;
            default-font-size: 24px;

            in-out property <int> count;
            in-out property <bool> details-open;
            forward-focus: keys;

            keys := FocusScope {
                key-pressed(event) => {
                    if event.text == Key.Return || event.text == Key.UpArrow {
                        root.count += 1;
                        return accept;
                    }
                    if event.text == Key.DownArrow {
                        root.details-open = !root.details-open;
                        return accept;
                    }
                    return reject;
                }
                VerticalLayout {
                    padding: 64px;
                    spacing: 24px;
                    Text { text: "Slint Counter"; color: white; font-size: 52px; }
                    Text { text: "Count  " + root.count; color: #a7d8ff; font-size: 40px; }
                    HorizontalLayout {
                        spacing: 24px;
                        Button {
                            min-height: 64px;
                            text: "Increment";
                            clicked => { root.count += 1; }
                        }
                        Button {
                            min-height: 64px;
                            text: root.details-open ? "Hide input" : "Show input";
                            clicked => { root.details-open = !root.details-open; }
                        }
                    }
                    Rectangle {
                        clip: true;
                        height: 140px;
                        Rectangle {
                            width: parent.width;
                            height: parent.height;
                            y: root.details-open ? 0px : -self.height;
                            animate y { duration: 250ms; easing: ease-out; }
                            background: #263448;
                            border-radius: 12px;
                            VerticalLayout {
                                padding: 24px;
                                LineEdit {
                                    enabled: root.details-open;
                                    placeholder-text: "Enter text";
                                }
                            }
                        }
                    }
                    Rectangle {}
                    Text {
                        text: "OK / Up: Increment     Down: Toggle input     Back: Exit";
                        color: #c1cce0;
                    }
                }
            }
        }
    }
}

fn main() -> Result<(), PlatformError> {
    // The webOS app manager also passes a native launch event as JSON.
    let smoke = std::env::args_os().skip(1).any(|arg| arg == "--smoke");
    slint_webos::init()?;
    let app = ui::Counter::new()?;
    if !smoke {
        return app.run();
    }

    let weak = app.as_weak();
    Timer::single_shot(Duration::from_millis(400), move || {
        if let Some(app) = weak.upgrade() {
            app.set_count(1);
            app.set_details_open(true);
        }
    });
    let snapshot = Rc::new(RefCell::new(None));
    let capture = Rc::clone(&snapshot);
    let weak = app.as_weak();
    Timer::single_shot(Duration::from_millis(1100), move || {
        if let Some(app) = weak.upgrade() {
            capture.replace(Some(app.window().take_snapshot()));
        }
        let _ = slint::quit_event_loop();
    });
    app.run()?;
    let snapshot = snapshot
        .borrow_mut()
        .take()
        .ok_or("smoke test ended before capturing the window")??;
    println!("snapshot={}x{}", snapshot.width(), snapshot.height());
    Ok(())
}
