use benchmark_common::{Scenario, hover};
use floem::{event::EventListener, prelude::*, style::Style};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run(
        Scenario::Hover,
        |runtime| {
            let observer = runtime.harness.borrow().hover_observer();
            v_stack_from_iter((0..hover::ROWS).map(|row| {
                let observer = observer.clone();
                h_stack_from_iter((0..hover::COLUMNS).map(move |column| {
                    let index = row * hover::COLUMNS + column;
                    let enter = observer.clone();
                    let leave = observer.clone();
                    empty()
                        .style(|_| cell_style())
                        .on_event_cont(EventListener::PointerEnter, move |_| enter.enter(index))
                        .on_event_cont(EventListener::PointerLeave, move |_| leave.leave(index))
                }))
                .style(|style| {
                    style
                        .width(hover::GRID_SIZE)
                        .height(hover::CELL_SIZE)
                        .flex_shrink(0.0)
                        .gap(0.0)
                })
            }))
            .style(|style| {
                style
                    .width(hover::GRID_SIZE)
                    .height(hover::GRID_SIZE)
                    .flex_shrink(0.0)
                    .gap(0.0)
            })
            .into_any()
        },
        |_, _| Ok(0),
    )
}

fn cell_style() -> Style {
    let normal = hover::NORMAL_COLOR;
    let hovered = hover::HOVERED_COLOR;
    Style::new()
        .width(hover::CELL_SIZE)
        .height(hover::CELL_SIZE)
        .flex_shrink(0.0)
        .background(Color::rgb8(normal.0, normal.1, normal.2))
        .hover(move |style| style.background(Color::rgb8(hovered.0, hovered.1, hovered.2)))
}

#[cfg(test)]
mod tests {
    use crate::selected_hover::cell_style;
    use benchmark_common::hover;
    use floem::{
        peniko::Brush,
        prelude::Color,
        style::{Background, StyleSelector},
    };

    #[test]
    fn hover_cell_style_changes_only_background_without_changing_geometry() {
        let normal = cell_style();
        let hovered = normal.clone().apply_selectors(&[StyleSelector::Hover]);
        let normal_rgb = hover::NORMAL_COLOR;
        let hovered_rgb = hover::HOVERED_COLOR;
        assert_eq!(
            normal.get(Background),
            Some(Brush::Solid(Color::rgb8(
                normal_rgb.0,
                normal_rgb.1,
                normal_rgb.2
            )))
        );
        assert_eq!(
            hovered.get(Background),
            Some(Brush::Solid(Color::rgb8(
                hovered_rgb.0,
                hovered_rgb.1,
                hovered_rgb.2
            )))
        );
        let layout = normal.to_taffy_style();
        assert_eq!(layout, hovered.to_taffy_style());
        assert_eq!(
            layout.size.width,
            floem::taffy::style::Dimension::Length(24.0)
        );
        assert_eq!(
            layout.size.height,
            floem::taffy::style::Dimension::Length(24.0)
        );
        assert_eq!(layout.flex_shrink, 0.0);
        assert_eq!(normal.get(Background), cell_style().get(Background));
    }
}
