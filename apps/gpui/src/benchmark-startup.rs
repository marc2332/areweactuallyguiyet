use crate::ui::{Protocol, ScenarioView};
use benchmark_common::{FrameState, Harness, Scenario};
use gpui::{App, Context, IntoElement, Render, Window, div, prelude::*, rgb};
use std::{cell::RefCell, rc::Rc};

use crate::ui::row;
use gpui::Entity;

struct BenchView {
    protocol: Protocol,
    startup_input: Option<Entity<crate::input::TextInput>>,
    startup_submitted: bool,
}

impl Render for BenchView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = self.startup(cx);
        self.protocol.rendered = true;
        div().size_full().font_family("DejaVu Sans").child(content)
    }
}

impl BenchView {
    fn startup(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let input = self.startup_input.as_ref().map(|input| input.clone());
        div()
            .id("startup-screen")
            .size_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .gap_4()
            .p_8()
            .bg(rgb(0xffffff))
            .child(div().text_2xl().child("Startup form"))
            .child(div().child("Enter a value"))
            .children(input)
            .child(
                div()
                    .px_4()
                    .py_2()
                    .bg(rgb(0x2f9e44))
                    .text_color(rgb(0xffffff))
                    .cursor_pointer()
                    .child(if self.startup_submitted {
                        "Submitted"
                    } else {
                        "Submit"
                    })
                    .on_mouse_up(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.startup_submitted = true;
                            cx.notify();
                        }),
                    ),
            )
            .children((0..20).map(|index| row(index)))
            .into_any_element()
    }
}

impl ScenarioView for BenchView {
    const SCENARIO: Scenario = Scenario::Startup;
    fn new(cx: &mut App, harness: Rc<RefCell<Harness>>) -> Self {
        Self {
            protocol: Protocol {
                harness,
                reset_generation: 0,
                rendered: false,
            },
            startup_input: Some(cx.new(|cx| crate::input::TextInput::new(String::new(), cx))),
            startup_submitted: false,
        }
    }
    fn protocol(&mut self) -> &mut Protocol {
        &mut self.protocol
    }
    fn reset(&mut self, cx: &mut Context<Self>) {
        self.startup_submitted = false;
        if let Some(input) = self.startup_input.as_ref() {
            input.update(cx, |input, cx| input.set_value(String::new(), cx));
        }
    }
    fn advance(&mut self, state: FrameState, cx: &mut Context<Self>) {
        let _ = (state, cx);
    }
    fn bind_keys(cx: &mut App) {
        crate::input::bind_keys(cx);
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run::<BenchView>()
}
