use crate::ui::optional_input;
use crate::ui::{ScenarioView, row};
use benchmark_common::{FrameState, Scenario, SwitchMode};
use gpui::InteractiveElement;
use gpui::{Context, IntoElement, Render, Window, div, prelude::*, px, rgb};
use std::rc::Rc;

struct BenchView {
    startup_input: Option<gpui::Entity<crate::text_input::TextInput>>,
    startup_submitted: bool,
    rendered: bool,
}

impl Render for BenchView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.rendered = true;
        div()
            .size_full()
            .font_family("DejaVu Sans")
            .child(self.startup(cx))
    }
}

impl BenchView {
    fn startup(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        if self.startup_input.is_none() {
            self.startup_input = Some(crate::text_input::TextInput::create(
                cx,
                String::new(),
                "Type something…",
                Rc::new(|_, _| {}),
            ));
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_8()
            .bg(rgb(0xffffff))
            .child(div().text_2xl().child("Startup benchmark"))
            .child(
                div()
                    .text_base()
                    .child("A minimal interactive GPUI Fast application."),
            )
            .child(
                div()
                    .id("startup-input")
                    .w(px(420.))
                    .h(px(40.))
                    .border_1()
                    .border_color(rgb(0x888888))
                    .child(optional_input(self.startup_input.clone())),
            )
            .child(
                div()
                    .id("startup-button")
                    .w(px(140.))
                    .h(px(40.))
                    .bg(rgb(0x3355aa))
                    .text_color(rgb(0xffffff))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.startup_submitted = !this.startup_submitted;
                        cx.notify();
                    }))
                    .child(if self.startup_submitted {
                        "Submitted"
                    } else {
                        "Continue"
                    }),
            )
            .child(
                div()
                    .id("startup-records")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .children((0..20).map(row)),
            )
            .into_any_element()
    }
}

impl ScenarioView for BenchView {
    const SCENARIO: Scenario = Scenario::Startup;

    fn new() -> Self {
        Self {
            startup_input: None,
            startup_submitted: false,
            rendered: false,
        }
    }

    fn rendered(&self) -> bool {
        self.rendered
    }

    fn reset(&mut self, _cx: &mut Context<Self>) {}

    fn update(&mut self, state: FrameState, _mode: SwitchMode, _cx: &mut Context<Self>) -> u64 {
        state.update_index
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run::<BenchView>()
}
