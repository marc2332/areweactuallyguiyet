use benchmark_common::Scenario;
use zgui::prelude::*;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run(Scenario::Scroll, |runtime| {
        let viewport = NodeRef::new();
        let mut reset_generation = 0;
        runtime.schedule(move |_, frame| {
            if reset_generation != frame.reset_generation {
                reset_generation = frame.reset_generation;
                crate::ui::scroll_to(viewport, 0.0);
            }
            if frame.running {
                crate::ui::scroll_to(viewport, frame.scroll_offset);
            }
            Ok(frame.update_index)
        });
        view! {
            scroll(class = "full", node_ref = viewport) {
                column {
                    for index in || 0..2_000, key = |index: &usize| *index {
                        {crate::ui::row(index)}
                    }
                }
            }
        }
    })
}
