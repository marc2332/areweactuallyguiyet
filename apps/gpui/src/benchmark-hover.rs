use crate::ui::{Protocol, ScenarioView};
use benchmark_common::{
    FrameState, Harness, Scenario,
    hover::{self, HoverObserver},
};
use gpui::{
    App, Context, Entity, IntoElement, Render, StyleRefinement, Window, div, prelude::*, px, rgb,
};
use std::{cell::RefCell, rc::Rc};

struct HoverCell {
    index: usize,
    hovered: bool,
    observer: HoverObserver,
}

fn color(value: (u8, u8, u8)) -> gpui::Rgba {
    rgb((u32::from(value.0) << 16) | (u32::from(value.1) << 8) | u32::from(value.2))
}

impl Render for HoverCell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id(self.index)
            .size(px(hover::CELL_SIZE))
            .flex_shrink_0()
            .bg(color(if self.hovered {
                hover::HOVERED_COLOR
            } else {
                hover::NORMAL_COLOR
            }))
            .on_hover(cx.listener(|cell, hovered, _, cx| {
                if *hovered {
                    cell.observer.enter(cell.index);
                } else {
                    cell.observer.leave(cell.index);
                }
                if cell.hovered != *hovered {
                    cell.hovered = *hovered;
                    cx.notify();
                }
            }))
    }
}

struct HoverGrid {
    protocol: Protocol,
    cells: Vec<Entity<HoverCell>>,
}

impl Render for HoverGrid {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut grid = div()
            .size_full()
            .flex()
            .flex_col()
            .bg(color(hover::BACKGROUND));
        for row in 0..hover::ROWS {
            let mut line = div()
                .flex()
                .w(px(hover::GRID_SIZE))
                .h(px(hover::CELL_SIZE))
                .flex_shrink_0();
            for column in 0..hover::COLUMNS {
                line = line.child(
                    div().size(px(hover::CELL_SIZE)).flex_shrink_0().child(
                        gpui::AnyView::from(self.cells[row * hover::COLUMNS + column].clone())
                            .cached(StyleRefinement::default()),
                    ),
                );
            }
            grid = grid.child(line);
        }
        self.protocol.rendered = true;
        grid
    }
}

impl ScenarioView for HoverGrid {
    const SCENARIO: Scenario = Scenario::Hover;
    const NOTIFY_EVERY_TICK: bool = false;

    fn new(cx: &mut App, harness: Rc<RefCell<Harness>>) -> Self {
        let observer = harness.borrow().hover_observer();
        Self {
            protocol: Protocol {
                harness,
                reset_generation: 0,
                rendered: false,
            },
            cells: (0..hover::COUNT)
                .map(|index| {
                    let observer = observer.clone();
                    cx.new(|_| HoverCell {
                        index,
                        hovered: false,
                        observer,
                    })
                })
                .collect(),
        }
    }

    fn protocol(&mut self) -> &mut Protocol {
        &mut self.protocol
    }

    fn reset(&mut self, _cx: &mut Context<Self>) {}

    fn advance(&mut self, _state: FrameState, _cx: &mut Context<Self>) {}
}

pub(crate) fn mark_ready(
    harness: &mut Harness,
    window: &Window,
) -> Result<(), Box<dyn std::error::Error>> {
    // GPUI 0.2.2's X11 handle accessors panic, so backend identity is unavailable safely.
    harness.mark_hover_ready(
        "initial-render-returned-not-presented",
        "unspecified",
        f64::from(window.scale_factor()),
        window.viewport_size().width.into(),
        window.viewport_size().height.into(),
    )?;
    Ok(())
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run::<HoverGrid>()
}
