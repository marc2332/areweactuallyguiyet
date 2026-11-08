use crate::ui::{ScenarioView, row};
use benchmark_common::{FrameState, Scenario, SwitchMode};
use gpui::{Context, IntoElement, Render, Window, div, prelude::*, px, rgb};

struct BenchView {
    scroll: gpui::ScrollHandle,
    rendered: bool,
}

impl Render for BenchView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.rendered = true;
        div()
            .size_full()
            .font_family("DejaVu Sans")
            .child(self.rows())
    }
}

impl BenchView {
    fn rows(&self) -> gpui::AnyElement {
        let content = (0..2000).map(row);
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

    fn new() -> Self {
        Self {
            scroll: gpui::ScrollHandle::new(),
            rendered: false,
        }
    }

    fn rendered(&self) -> bool {
        self.rendered
    }

    fn reset(&mut self, _cx: &mut Context<Self>) {
        self.scroll.set_offset(gpui::point(px(0.), px(0.)));
    }

    fn update(&mut self, state: FrameState, _mode: SwitchMode, _cx: &mut Context<Self>) -> u64 {
        if state.running {
            self.scroll
                .set_offset(gpui::point(px(0.), px(-state.scroll_offset)));
        }
        state.update_index
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run::<BenchView>()
}
