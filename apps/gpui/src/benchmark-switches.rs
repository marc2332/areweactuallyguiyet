use crate::ui::{Protocol, ScenarioView};
use benchmark_common::{FrameState, Harness, Scenario};
use gpui::{App, Context, IntoElement, Render, Window, div, prelude::*, px, rgb};
use std::{cell::RefCell, rc::Rc};

use benchmark_common::{
    SWITCH_CELL_SIZE, SWITCH_COLUMNS, SWITCH_COUNT, SWITCH_FONT_SIZE, SWITCH_OFF_COLOR,
    SWITCH_ON_COLOR, SWITCH_ROWS, SWITCH_TEXT_COLOR, Switches,
};
use gpui::{Entity, StyleRefinement};
struct DummyToggle {
    index: usize,
    on: bool,
    switches: Rc<RefCell<Switches>>,
}

impl Render for DummyToggle {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let color = if self.on {
            SWITCH_ON_COLOR
        } else {
            SWITCH_OFF_COLOR
        };
        div()
            .w_full()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .bg(rgb(((color.0 as u32) << 16)
                | ((color.1 as u32) << 8)
                | color.2 as u32))
            .text_size(px(SWITCH_FONT_SIZE))
            .text_color(rgb(((SWITCH_TEXT_COLOR.0 as u32) << 16)
                | ((SWITCH_TEXT_COLOR.1 as u32) << 8)
                | SWITCH_TEXT_COLOR.2 as u32))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.on = !this.on;
                    this.switches.borrow_mut().toggle(this.index);
                    cx.notify();
                }),
            )
            .child(if self.on { "ON" } else { "OFF" })
    }
}

struct BenchView {
    protocol: Protocol,
    switches: Rc<RefCell<Switches>>,
    switch_cells: Vec<Entity<DummyToggle>>,
}

impl Render for BenchView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let content = self.switch_grid();
        self.protocol.rendered = true;
        div().size_full().font_family("DejaVu Sans").child(content)
    }
}

impl BenchView {
    fn switch_grid(&self) -> gpui::AnyElement {
        let mut grid = div().size_full().flex().flex_col().bg(rgb(0xffffff));
        for row_index in 0..SWITCH_ROWS {
            let mut line = div()
                .flex()
                .w(px(SWITCH_CELL_SIZE * SWITCH_COLUMNS as f32))
                .h(px(SWITCH_CELL_SIZE))
                .flex_shrink_0();
            for column in 0..SWITCH_COLUMNS {
                let index = row_index * SWITCH_COLUMNS + column;
                line = line.child(
                    div().size(px(SWITCH_CELL_SIZE)).flex_shrink_0().child(
                        gpui::AnyView::from(self.switch_cells[index].clone())
                            .cached(StyleRefinement::default()),
                    ),
                );
            }
            grid = grid.child(line);
        }
        grid.into_any_element()
    }
}

impl ScenarioView for BenchView {
    const SCENARIO: Scenario = Scenario::Switches;
    const NOTIFY_EVERY_TICK: bool = false;
    fn new(cx: &mut App, harness: Rc<RefCell<Harness>>) -> Self {
        let switches = Rc::new(RefCell::new(Switches::default()));
        Self {
            protocol: Protocol {
                harness,
                reset_generation: 0,
                rendered: false,
            },
            switches: switches.clone(),
            switch_cells: (0..SWITCH_COUNT)
                .map(|index| {
                    let switches = switches.clone();
                    cx.new(|_| DummyToggle {
                        index,
                        on: false,
                        switches,
                    })
                })
                .collect(),
        }
    }
    fn protocol(&mut self) -> &mut Protocol {
        &mut self.protocol
    }
    fn reset(&mut self, cx: &mut Context<Self>) {
        *self.switches.borrow_mut() = Switches::default();
        for cell in &self.switch_cells {
            cell.update(cx, |cell, cx| {
                if cell.on {
                    cell.on = false;
                    cx.notify();
                }
            });
        }
    }
    fn advance(&mut self, state: FrameState, cx: &mut Context<Self>) {
        let mode = self.protocol.harness.borrow().switch_mode();
        let cells = &self.switch_cells;
        self.switches
            .borrow_mut()
            .advance_changed(state.update_index, mode, |index, value| {
                cells[index].update(cx, |cell, cx| {
                    cell.on = value;
                    cx.notify();
                });
            });
    }
    fn applied(&self, _state: FrameState) -> u64 {
        self.switches.borrow().applied_updates()
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run::<BenchView>()
}
