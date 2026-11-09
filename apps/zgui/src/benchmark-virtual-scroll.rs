use benchmark_common::Scenario;
use zgui::prelude::*;
use zgui_ui::virtualize::Virtualize;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run(Scenario::VirtualScroll, |runtime| {
        let viewport = NodeRef::new();
        let rows = RwSignal::new_local(100_000usize);
        let visible = Virtualize::new(viewport, rows.into(), benchmark_common::ROW_HEIGHT, 4);
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
                column(
                    style:padding-top = move || Some(format!("{}px", visible.window().lead)),
                    style:padding-bottom = move || Some(format!("{}px", visible.window().trail))
                ) {
                    for index in move || visible.indices(), key = |index: &usize| *index {
                        {crate::ui::row(index)}
                    }
                }
            }
        }
    })
}
