use std::{cell::Cell, rc::Rc};

use benchmark_common::Scenario;
use floem::{kurbo::Point, prelude::*};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let scrolling = create_rw_signal(None);
    let reset = Rc::new(Cell::new(0));
    let tick_reset = Rc::clone(&reset);
    crate::ui::run(
        Scenario::Scroll,
        move |_| {
            scroll(v_stack_from_iter((0..2000).map(crate::ui::row)))
                .scroll_to(move || scrolling.get())
                .style(|style| style.size_full())
                .into_any()
        },
        move |runtime, frame| {
            if tick_reset.replace(frame.reset_generation) != frame.reset_generation {
                scrolling.set(Some(Point::ZERO));
            }
            if frame.running {
                scrolling.set(Some(Point::new(0.0, f64::from(frame.scroll_offset))));
            }
            let _ = runtime;
            Ok(frame.update_index)
        },
    )
}
