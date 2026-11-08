use crate::ui::{Protocol, ScenarioView};
use benchmark_common::{FrameState, Harness, Scenario};
use gpui::{App, Context, IntoElement, Render, Window, div, prelude::*, px, rgb};
use std::{cell::RefCell, rc::Rc};

use gpui::{Entity, ScrollHandle, point};
use std::sync::Arc;

struct BenchView {
    protocol: Protocol,
    dashboard: Option<benchmark_common::dashboard::Dashboard>,
    dashboard_inputs: Vec<Entity<crate::input::TextInput>>,
    thumbnails: Vec<Arc<gpui::Image>>,
    scroll: ScrollHandle,
    memory_sidebar_scroll: ScrollHandle,
    memory_details_scroll: ScrollHandle,
    memory_fields_scroll: ScrollHandle,
}

impl Render for BenchView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = self.memory(cx);
        self.protocol.rendered = true;
        div().size_full().font_family("DejaVu Sans").child(content)
    }
}

impl BenchView {
    fn memory(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(dashboard) = self.dashboard.as_ref() else {
            return div().into_any_element();
        };
        let visible = dashboard.visible_records();
        let mut sidebar = div()
            .id("dashboard-sidebar")
            .w(px(180.))
            .h_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .track_scroll(&self.memory_sidebar_scroll)
            .bg(rgb(0xf1f3f5));
        for section in 0..benchmark_common::dashboard::SECTION_COUNT {
            let label = if section == 0 {
                "All records".to_owned()
            } else {
                format!("Section {section:02}")
            };
            sidebar = sidebar.child(
                div()
                    .px_2()
                    .h(px(22.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .text_size(px(12.))
                    .cursor_pointer()
                    .child(label)
                    .on_mouse_up(
                        gpui::MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            if let Some(model) = this.dashboard.as_mut() {
                                model.navigate(section);
                            }
                            cx.notify();
                        }),
                    ),
            );
        }
        let mut thumbnails = div().flex().flex_wrap().w_full();
        for image in &self.thumbnails {
            thumbnails = thumbnails.child(
                div()
                    .w(px(80.))
                    .h(px(52.))
                    .child(gpui::img(image.clone()).size(px(48.))),
            );
        }
        sidebar = sidebar.child(thumbnails);
        let mut records = div()
            .id("dashboard-records")
            .w(px(600.))
            .h_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .bg(rgb(0xffffff));
        for index in visible {
            let record = &dashboard.fixtures.records[index];
            let title = record.title.clone();
            let body = record.body.clone();
            let selected = dashboard.selected_record == index;
            let toggle_label = if dashboard.active.get(index) {
                "ON"
            } else {
                "OFF"
            };
            records = records.child(
                div()
                    .h(px(64.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .bg(if selected {
                        rgb(0xdbeafe)
                    } else {
                        rgb(0xffffff)
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().truncate().child(title))
                            .child(div().text_xs().truncate().child(body))
                            .on_mouse_up(
                                gpui::MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    if let Some(model) = this.dashboard.as_mut() {
                                        model.select(index);
                                    }
                                    cx.notify();
                                }),
                            ),
                    )
                    .child(
                        div()
                            .w(px(48.))
                            .h(px(28.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(12.))
                            .bg(if dashboard.active.get(index) {
                                rgb(0x2f9e44)
                            } else {
                                rgb(0x868e96)
                            })
                            .text_color(rgb(0xffffff))
                            .cursor_pointer()
                            .child(toggle_label)
                            .on_mouse_up(
                                gpui::MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    if let Some(model) = this.dashboard.as_mut() {
                                        model.toggle(index);
                                    }
                                    cx.notify();
                                }),
                            ),
                    ),
            );
        }
        let description = div()
            .id("dashboard-description")
            .h(px(300.))
            .flex_shrink_0()
            .overflow_y_scroll()
            .track_scroll(&self.memory_details_scroll)
            .child(
                div()
                    .whitespace_normal()
                    .child(dashboard.detail.to_string()),
            );
        let mut fields = div()
            .id("dashboard-fields")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.memory_fields_scroll);
        for (index, input) in self.dashboard_inputs.iter().skip(1).enumerate() {
            fields = fields.child(
                div()
                    .h(px(36.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .w(px(90.))
                            .text_sm()
                            .child(format!("Field {:02}", index + 1)),
                    )
                    .child(div().flex_1().min_w_0().child(input.clone())),
            );
        }
        let detail = div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .p_3()
            .bg(rgb(0xffffff))
            .child(
                div().text_lg().child(
                    dashboard.fixtures.records[dashboard.selected_record]
                        .title
                        .clone(),
                ),
            )
            .child(description)
            .child(fields);
        div()
            .size_full()
            .font_family("DejaVu Sans")
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(56.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .px_3()
                    .bg(rgb(0x343a40))
                    .text_color(rgb(0xffffff))
                    .child("Dashboard")
                    .child("Search")
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .children(self.dashboard_inputs.first().cloned()),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(sidebar)
                    .child(records)
                    .child(detail),
            )
            .into_any_element()
    }
}

impl ScenarioView for BenchView {
    const SCENARIO: Scenario = Scenario::Memory;
    fn new(cx: &mut App, harness: Rc<RefCell<Harness>>) -> Self {
        let dashboard = benchmark_common::dashboard::Dashboard::default();
        let mut inputs = Vec::with_capacity(21);
        inputs.push(cx.new(|cx| crate::input::TextInput::new(dashboard.query.clone(), cx)));
        for value in &dashboard.fields {
            inputs.push(cx.new(|cx| crate::input::TextInput::new(value.clone(), cx)));
        }
        Self {
            protocol: Protocol {
                harness,
                reset_generation: 0,
                rendered: false,
            },
            dashboard: Some(dashboard),
            dashboard_inputs: inputs,
            thumbnails: benchmark_common::dashboard::THUMBNAILS
                .iter()
                .map(|bytes| {
                    Arc::new(gpui::Image::from_bytes(
                        gpui::ImageFormat::Png,
                        bytes.to_vec(),
                    ))
                })
                .collect(),
            scroll: ScrollHandle::new(),
            memory_sidebar_scroll: ScrollHandle::new(),
            memory_details_scroll: ScrollHandle::new(),
            memory_fields_scroll: ScrollHandle::new(),
        }
    }
    fn protocol(&mut self) -> &mut Protocol {
        &mut self.protocol
    }
    fn reset(&mut self, cx: &mut Context<Self>) {
        self.scroll.set_offset(point(px(0.), px(0.)));
        self.memory_sidebar_scroll.set_offset(point(px(0.), px(0.)));
        self.memory_details_scroll.set_offset(point(px(0.), px(0.)));
        self.memory_fields_scroll.set_offset(point(px(0.), px(0.)));
        if let Some(dashboard) = self.dashboard.as_mut() {
            dashboard.reset();
        }
        if let Some(input) = self.dashboard_inputs.first() {
            input.update(cx, |input, cx| input.set_value(String::new(), cx));
        }
        for (input, value) in self.dashboard_inputs.iter().skip(1).zip(
            self.dashboard
                .as_ref()
                .into_iter()
                .flat_map(|model| model.fields.iter()),
        ) {
            input.update(cx, |input, cx| input.set_value(value.clone(), cx));
        }
    }
    fn advance(&mut self, state: FrameState, cx: &mut Context<Self>) {
        if state.running {
            self.scroll
                .set_offset(point(px(0.), px(-state.scroll_offset)));
        }
        let _ = cx;
    }
    fn applied(&self, _state: FrameState) -> u64 {
        self.dashboard
            .as_ref()
            .map_or(0, benchmark_common::dashboard::Dashboard::applied_updates)
    }
    fn before_ready(&mut self, state: FrameState, cx: &mut Context<Self>) {
        if let (Some(model), Some(search)) =
            (self.dashboard.as_mut(), self.dashboard_inputs.first())
        {
            model.search(search.read(cx).value().to_owned());
        }
        if let Some(model) = self.dashboard.as_mut() {
            for (index, input) in self.dashboard_inputs.iter().skip(1).enumerate() {
                model.edit(index, input.read(cx).value().to_owned());
            }
            if state.running {
                model.advance(state);
            }
            if let Some(search) = self.dashboard_inputs.first() {
                if search.read(cx).value() != model.query {
                    search.update(cx, |input, cx| input.set_value(model.query.clone(), cx));
                }
            }
            for (input, value) in self.dashboard_inputs.iter().skip(1).zip(&model.fields) {
                if input.read(cx).value() != value {
                    input.update(cx, |input, cx| input.set_value(value.clone(), cx));
                }
            }
        }
    }
    fn report(&self) -> std::io::Result<()> {
        if let Some(dashboard) = self.dashboard.as_ref() {
            self.protocol
                .harness
                .borrow_mut()
                .report_dashboard(dashboard)?;
        }
        Ok(())
    }
    fn bind_keys(cx: &mut App) {
        crate::input::bind_keys(cx);
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run::<BenchView>()
}
