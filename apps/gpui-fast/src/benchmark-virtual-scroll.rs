use crate::ui::{ScenarioView, row};
use benchmark_common::{FrameState, Scenario, SwitchMode};
use gpui::{Context, IntoElement, Render, Window, div, prelude::*, px, rgb, uniform_list};

struct BenchView {
    virtual_scroll: gpui::UniformListScrollHandle,
    rendered: bool,
}

impl Render for BenchView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.rendered = true;
        div()
            .size_full()
            .font_family("DejaVu Sans")
            .child(self.virtual_rows(cx))
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
        .track_scroll(&self.virtual_scroll);
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

    fn new() -> Self {
        Self {
            virtual_scroll: gpui::UniformListScrollHandle::new(),
            rendered: false,
        }
    }

    fn rendered(&self) -> bool {
        self.rendered
    }

    fn reset(&mut self, _cx: &mut Context<Self>) {
        self.virtual_scroll
            .0
            .borrow()
            .base_handle
            .set_offset(gpui::point(px(0.), px(0.)));
    }

    fn update(&mut self, state: FrameState, _mode: SwitchMode, _cx: &mut Context<Self>) -> u64 {
        if state.running {
            self.virtual_scroll
                .0
                .borrow()
                .base_handle
                .set_offset(gpui::point(px(0.), px(-state.scroll_offset)));
        }
        state.update_index
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run::<BenchView>()
}
