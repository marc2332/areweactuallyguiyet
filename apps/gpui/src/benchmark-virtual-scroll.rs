use crate::ui::{Protocol, ScenarioView};
use benchmark_common::{FrameState, Harness, Scenario};
use gpui::{App, Context, IntoElement, Render, Window, div, prelude::*, px, rgb};
use std::{cell::RefCell, rc::Rc};

use crate::ui::row;
use gpui::{UniformListScrollHandle, point, uniform_list};

struct BenchView {
    protocol: Protocol,
    virtual_scroll: UniformListScrollHandle,
}

impl Render for BenchView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = self.virtual_rows(cx);
        self.protocol.rendered = true;
        div().size_full().font_family("DejaVu Sans").child(content)
    }
}

impl BenchView {
    fn virtual_rows(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let count = 100_000;
        let list = uniform_list(
            "records",
            count,
            cx.processor(|_this, range: std::ops::Range<usize>, _window, _cx| {
                range.map(row).collect::<Vec<_>>()
            }),
        )
        .size_full()
        .track_scroll(self.virtual_scroll.clone());
        div()
            .id("virtual-records")
            .size_full()
            .bg(rgb(0xffffff))
            .child(list)
            .into_any_element()
    }
}

impl ScenarioView for BenchView {
    const SCENARIO: Scenario = Scenario::VirtualScroll;
    fn new(_cx: &mut App, harness: Rc<RefCell<Harness>>) -> Self {
        Self {
            protocol: Protocol {
                harness,
                reset_generation: 0,
                rendered: false,
            },
            virtual_scroll: UniformListScrollHandle::new(),
        }
    }
    fn protocol(&mut self) -> &mut Protocol {
        &mut self.protocol
    }
    fn reset(&mut self, cx: &mut Context<Self>) {
        self.virtual_scroll
            .0
            .borrow()
            .base_handle
            .set_offset(point(px(0.), px(0.)));
        let _ = cx;
    }
    fn advance(&mut self, state: FrameState, cx: &mut Context<Self>) {
        if state.running {
            self.virtual_scroll
                .0
                .borrow()
                .base_handle
                .set_offset(point(px(0.), px(-state.scroll_offset)));
        }
        let _ = cx;
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run::<BenchView>()
}
