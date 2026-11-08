use benchmark_common::Scenario;
use floem::prelude::*;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run(
        Scenario::Startup,
        |_| {
            let query = create_rw_signal(String::new());
            let applied = create_rw_signal(0usize);
            v_stack((
                label(|| "Benchmark startup".to_owned()),
                label(|| "Prepared initial content".to_owned()),
                text_input(query)
                    .placeholder("Search records")
                    .style(|style| style.width(430.0).height(32.0)),
                button(label(move || format!("Apply ({})", applied.get())))
                    .on_click_stop(move |_| applied.update(|value| *value += 1)),
                scroll(v_stack_from_iter((0..20).map(crate::ui::row)))
                    .style(|style| style.width_full().min_height(0.0).flex_grow(1.0)),
            ))
            .style(|style| style.size_full().padding(16.0).gap(8.0))
            .into_any()
        },
        |_, frame| Ok(frame.update_index),
    )
}
