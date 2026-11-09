use benchmark_common::{Scenario, hover};
use zgui::prelude::*;

pub(crate) fn style() -> String {
    let (normal_red, normal_green, normal_blue) = hover::NORMAL_COLOR;
    let (hovered_red, hovered_green, hovered_blue) = hover::HOVERED_COLOR;
    format!(
        ".hover-grid {{ width: {grid}px; height: {grid}px; gap: 0; padding: 0; }}
         .hover-row {{ width: {grid}px; height: {cell}px; flex-shrink: 0; gap: 0; }}
         .hover-cell {{ width: {cell}px; height: {cell}px; flex-shrink: 0; padding: 0; border: none; border-radius: 0; background: rgb({normal_red}, {normal_green}, {normal_blue}); }}
         .hover-cell:hover {{ background: rgb({hovered_red}, {hovered_green}, {hovered_blue}); }}",
        grid = hover::GRID_SIZE,
        cell = hover::CELL_SIZE,
    )
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run(Scenario::Hover, |runtime| {
        let observer = runtime.harness.borrow().hover_observer();
        runtime.schedule(|_, frame| Ok(frame.update_index));
        let rows: Vec<_> = (0..hover::ROWS)
            .map(|row_index| {
                let cells: Vec<_> = (0..hover::COLUMNS)
                    .map(|column| {
                        let index = row_index * hover::COLUMNS + column;
                        let enter_observer = observer.clone();
                        let leave_observer = observer.clone();
                        view! {
                            column(
                                class = "hover-cell",
                                on:pointer_enter = move |_| enter_observer.enter(index),
                                on:pointer_leave = move |_| leave_observer.leave(index)
                            ) {}
                        }
                    })
                    .collect();
                view! { row(class = "hover-row") {{cells}} }
            })
            .collect();
        view! { column(class = "hover-grid") {{rows}} }
    })
}
