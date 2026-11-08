use crate::ui::ScenarioView;
use crate::ui::color;
use benchmark_common::{
    FrameState, SWITCH_CELL_SIZE, SWITCH_COLUMNS, SWITCH_COUNT, SWITCH_FONT_SIZE, SWITCH_OFF_COLOR,
    SWITCH_ON_COLOR, SWITCH_ROWS, SWITCH_TEXT_COLOR, Scenario, SwitchMode, Switches,
};
use gpui::{Context, InteractiveElement, IntoElement, Render, Window, div, prelude::*, px, rgb};
use std::{cell::RefCell, rc::Rc};

struct BenchView {
    switches: std::rc::Rc<std::cell::RefCell<Switches>>,
    switch_cells: Vec<gpui::Entity<SwitchCell>>,
    rendered: bool,
}

impl Render for BenchView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.rendered = true;
        div()
            .size_full()
            .font_family("DejaVu Sans")
            .child(self.switch_grid(cx))
    }
}

impl BenchView {
    fn switch_grid(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        if self.switch_cells.is_empty() {
            self.switch_cells = (0..SWITCH_COUNT)
                .map(|index| {
                    let switches = self.switches.clone();
                    cx.new(move |_| SwitchCell {
                        on: false,
                        index,
                        switches,
                    })
                })
                .collect();
        }
        let mut grid = div().size_full().flex().flex_col().bg(rgb(0xffffff));
        for row_index in 0..SWITCH_ROWS {
            let mut line = div()
                .flex()
                .w(px(SWITCH_CELL_SIZE * SWITCH_COLUMNS as f32))
                .h(px(SWITCH_CELL_SIZE))
                .flex_shrink_0();
            for column in 0..SWITCH_COLUMNS {
                let index = row_index * SWITCH_COLUMNS + column;
                line = line.child(self.switch_cells[index].clone());
            }
            grid = grid.child(line);
        }
        grid.into_any_element()
    }
}

struct SwitchCell {
    on: bool,
    index: usize,
    switches: Rc<RefCell<Switches>>,
}

impl Render for SwitchCell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id(self.index)
            .size(px(SWITCH_CELL_SIZE))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .bg(if self.on {
                rgb(color(SWITCH_ON_COLOR))
            } else {
                rgb(color(SWITCH_OFF_COLOR))
            })
            .text_size(px(SWITCH_FONT_SIZE))
            .text_color(rgb(color(SWITCH_TEXT_COLOR)))
            .on_click(cx.listener(|this, _, _, cx| {
                this.on = !this.on;
                this.switches.borrow_mut().toggle(this.index);
                cx.notify();
            }))
            .child(if self.on { "ON" } else { "OFF" })
    }
}

impl ScenarioView for BenchView {
    const SCENARIO: Scenario = Scenario::Switches;

    fn new() -> Self {
        Self {
            switches: std::rc::Rc::new(std::cell::RefCell::new(Switches::default())),
            switch_cells: Vec::new(),
            rendered: false,
        }
    }

    fn rendered(&self) -> bool {
        self.rendered
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

    fn update(&mut self, state: FrameState, mode: SwitchMode, cx: &mut Context<Self>) -> u64 {
        self.switches
            .borrow_mut()
            .advance_changed(state.update_index, mode, |index, value| {
                self.switch_cells[index].update(cx, |cell, cx| {
                    cell.on = value;
                    cx.notify();
                });
            });
        self.switches.borrow().applied_updates()
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run::<BenchView>()
}
