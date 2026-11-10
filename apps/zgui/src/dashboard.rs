use zgui::prelude::*;

const STYLE: &str = ":root { font-family: sans-serif; font-size: 14px; color: #111; background: #fff; } .heading { font-size: 24px; } column, row, scroll { box-sizing: border-box; } .root { width: 100%; height: 100%; min-height: 0; } .toolbar { height: 180px; min-height: 180px; padding: 16px 24px 8px; gap: 25px; } .controls { height: 40px; min-height: 40px; gap: 32px; align-items: center; } .field { width: 360px; height: 40px; padding: 10px; background: #f1f1f1; } .field.selected { background: #dce8fa; } .button { width: 160px; height: 40px; justify-content: center; align-items: center; background: #e8e8e8; } .toggle { width: 160px; height: 40px; margin-left: 16px; justify-content: center; align-items: center; background: #dbe9dc; } .records { flex: 1; min-height: 0; width: 100%; } .record { height: 88px; min-height: 88px; padding: 12px 24px; gap: 16px; align-items: center; } .record:nth-child(even) { background: #f6f6f6; } .record-image { width: 64px; height: 64px; } .copy { gap: 4px; }";

fn main_view() -> impl IntoView {
    let (count, set_count) = signal(0usize);
    let (enabled, set_enabled) = signal(false);
    let (notes, set_notes) = signal(String::from("Project notes"));
    let (selected, set_selected) = signal(false);
    view! {
        column(class = "root") {
            column(class = "toolbar") {
                text(class = "heading") {"Project dashboard"}
                row(class = "controls") {
                    row(
                        class = "field",
                        class:selected = move || selected.get(),
                        tabindex = Focus::Sequential,
                        a11y:role = Role::TextInput,
                        a11y:label = "Project notes",
                        on:click = move |_| set_selected.set(false),
                        on:key_down = move |event| {
                            if event.modifiers.control() {
                                if event.key_without_modifiers.inserted_text().is_some_and(|text| text.eq_ignore_ascii_case("a")) {
                                    set_selected.set(true);
                                }
                                return;
                            }
                            match &event.key {
                                Key::Named(NamedKey::Backspace) => {
                                    if selected.get_untracked() {
                                        set_notes.set(String::new());
                                        set_selected.set(false);
                                    } else {
                                        set_notes.update(|value| { value.pop(); });
                                    }
                                }
                                key => if let Some(inserted) = key.inserted_text() {
                                    if inserted.chars().any(char::is_control) {
                                        return;
                                    }
                                    if selected.get_untracked() {
                                        set_notes.set(inserted.to_owned());
                                        set_selected.set(false);
                                    } else {
                                        set_notes.update(|value| value.push_str(inserted));
                                    }
                                }
                            }
                        }
                    ) {
                        text { {move || notes.get()} }
                    }
                    control(class = "button", on:click = move |_| set_count.update(|n| *n += 1)) { "Refresh" }
                    control(class = "toggle", on:click = move |_| set_enabled.update(|v| *v = !*v)) {
                        {move || if enabled.get() { "Enabled" } else { "Disabled" }}
                    }
                }
                text { {move || format!("Refresh count: {}", count.get())} }
            }
            scroll(class = "records") {
                column {
                    for index in || 0..100, key = |index: &usize| *index {
                        row(class = "record") {
                            image(class = "record-image", src = Some(format!("{}/../../assets/images/dashboard-{}.png", env!("CARGO_MANIFEST_DIR"), index % 10)))
                            column(class = "copy") {
                                text { {format!("Project {}", index + 1)} }
                                text {"A project record with descriptive dashboard information."}
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn main() -> Result<(), zgui::Error> {
    app()
        .with_title("zgui Scroll 2000")
        .with_size(1000.0, 700.0)
        .with_stylesheet(STYLE)
        .run(main_view)
}
