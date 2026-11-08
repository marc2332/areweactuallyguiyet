use crate::ui::{Protocol, ScenarioView};
use benchmark_common::{FrameState, Harness, Scenario};
use gpui::{App, Context, IntoElement, Render, Window, div, prelude::*, px, rgb};
use std::{cell::RefCell, rc::Rc};

use crate::ui::row;
use gpui::{ScrollHandle, point};

struct BenchView {
    protocol: Protocol,
    scroll: ScrollHandle,
}

impl Render for BenchView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let content = self.rows();
        self.protocol.rendered = true;
        div().size_full().font_family("DejaVu Sans").child(content)
    }
}

impl BenchView {
    fn rows(&self) -> gpui::AnyElement {
        let content = (0..2000).map(|index| row(index));
        div()
            .id("scroll-rows")
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0xffffff))
            .overflow_scroll()
            .track_scroll(&self.scroll)
            .children(content)
            .into_any_element()
    }
}

impl ScenarioView for BenchView {
    const SCENARIO: Scenario = Scenario::Scroll;
    fn new(_cx: &mut App, harness: Rc<RefCell<Harness>>) -> Self {
        Self {
            protocol: Protocol {
                harness,
                reset_generation: 0,
                rendered: false,
            },
            scroll: ScrollHandle::new(),
        }
    }
    fn protocol(&mut self) -> &mut Protocol {
        &mut self.protocol
    }
    fn reset(&mut self, cx: &mut Context<Self>) {
        self.scroll.set_offset(point(px(0.), px(0.)));
        let _ = cx;
    }
    fn advance(&mut self, state: FrameState, cx: &mut Context<Self>) {
        if state.running {
            self.scroll
                .set_offset(point(px(0.), px(-state.scroll_offset)));
        }
        let _ = cx;
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run::<BenchView>()
}
