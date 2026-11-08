use std::{cell::Cell, rc::Rc};

use floem::{kurbo::Point, prelude::*};

use benchmark_common::Scenario;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let scrolling = create_rw_signal(None);
    let reset = Rc::new(Cell::new(0));
    let tick_reset = Rc::clone(&reset);
    crate::ui::run(
        Scenario::VirtualScroll,
        move |_| {
            let records: im::Vector<usize> = (0..100_000).collect();
            scroll(
                virtual_stack(
                    VirtualDirection::Vertical,
                    VirtualItemSize::Fixed(Box::new(|| 48.0)),
                    move || records.clone(),
                    |index| *index,
                    crate::ui::row,
                )
                .style(|style| style.width_full().flex_col()),
            )
            .scroll_to(move || scrolling.get())
            .style(|style| style.size_full())
            .into_any()
        },
        move |_, frame| {
            if tick_reset.replace(frame.reset_generation) != frame.reset_generation {
                scrolling.set(Some(Point::ZERO));
            }
            if frame.running {
                scrolling.set(Some(Point::new(0.0, f64::from(frame.scroll_offset))));
            }
            Ok(frame.update_index)
        },
    )
}
