use benchmark_common::Scenario;
use zgui::prelude::*;
use zgui_ui::prelude::*;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run(Scenario::Startup, |runtime| {
        let query = RwSignal::new_local(String::new());
        let applied = RwSignal::new(0usize);
        runtime.schedule(|_, frame| Ok(frame.update_index));
        view! {
            column(class = "full startup") {
                text {"Benchmark startup"}
                text {"Prepared initial content"}
                Input(value = query, placeholder = "Search records", class = "startup-input")
                control(
                    class = "apply",
                    tabindex = Focus::Sequential,
                    on:click = move |_| applied.update(|value| *value += 1)
                ) {
                    {move || format!("Apply ({})", applied.get())}
                }
                scroll(class = "startup-list") {
                    column {
                        for index in || 0..20, key = |index: &usize| *index {
                            {crate::ui::row(index)}
                        }
                    }
                }
            }
        }
    })
}
