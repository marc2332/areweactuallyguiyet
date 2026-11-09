use crate::ui::ScenarioView;
use benchmark_common::{
    FrameState, Harness, Scenario, SwitchMode,
    hover::{self, HoverObserver},
};
use gpui::{Context, Entity, IntoElement, Render, Window, div, prelude::*, px, rgb};

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
    rendered: bool,
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
                line = line.child(self.cells[row * hover::COLUMNS + column].clone());
            }
            grid = grid.child(line);
        }
        self.rendered = true;
        grid
    }
}

impl ScenarioView for HoverGrid {
    const SCENARIO: Scenario = Scenario::Hover;

    fn new() -> Self {
        Self {
            rendered: false,
            cells: Vec::new(),
        }
    }

    fn initialize_hover(&mut self, observer: HoverObserver, cx: &mut Context<Self>) {
        self.cells = (0..hover::COUNT)
            .map(|index| {
                let observer = observer.clone();
                cx.new(|_| HoverCell {
                    index,
                    hovered: false,
                    observer,
                })
            })
            .collect();
    }

    fn rendered(&self) -> bool {
        self.rendered
    }

    fn reset(&mut self, _cx: &mut Context<Self>) {}

    fn update(&mut self, _state: FrameState, _mode: SwitchMode, _cx: &mut Context<Self>) -> u64 {
        0
    }
}

pub(crate) fn mark_ready(
    harness: &mut Harness,
    window: &Window,
) -> Result<(), Box<dyn std::error::Error>> {
    use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
    let backend = match window.display_handle().ok().map(|handle| handle.as_raw()) {
        Some(RawDisplayHandle::Xlib(_) | RawDisplayHandle::Xcb(_)) => "x11",
        Some(RawDisplayHandle::Wayland(_)) => "wayland",
        _ => "unspecified",
    };
    harness.mark_hover_ready(
        "initial-render-returned-not-presented",
        backend,
        f64::from(window.scale_factor()),
        window.viewport_size().width.into(),
        window.viewport_size().height.into(),
    )?;
    Ok(())
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run::<HoverGrid>()
}
