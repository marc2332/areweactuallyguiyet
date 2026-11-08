use crate::ui::ScenarioView;
use crate::ui::optional_input;
use benchmark_common::dashboard::{Dashboard, RECORD_HEIGHT, SECTION_COUNT};
use benchmark_common::{FrameState, Scenario, SwitchMode};
use gpui::{Context, IntoElement, Render, Window, div, prelude::*, px, rgb};
use gpui::{ImageSource, InteractiveElement};
use std::{cell::RefCell, rc::Rc};

struct BenchView {
    dashboard: Option<std::rc::Rc<std::cell::RefCell<benchmark_common::dashboard::Dashboard>>>,
    search_input: Option<gpui::Entity<crate::text_input::TextInput>>,
    field_inputs: Vec<gpui::Entity<crate::text_input::TextInput>>,
    memory_sidebar_scroll: gpui::ScrollHandle,
    memory_records_scroll: gpui::ScrollHandle,
    memory_details_scroll: gpui::ScrollHandle,
    memory_fields_scroll: gpui::ScrollHandle,
    thumbnails: Vec<std::sync::Arc<gpui::Image>>,
    rendered: bool,
}

impl Render for BenchView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.rendered = true;
        div()
            .size_full()
            .font_family("DejaVu Sans")
            .child(self.memory(cx))
    }
}

impl BenchView {
    fn memory(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let dashboard = self
            .dashboard
            .get_or_insert_with(|| Rc::new(RefCell::new(Dashboard::default())))
            .clone();
        let owner = cx.entity().downgrade();
        if self.search_input.is_none() {
            let state = dashboard.clone();
            let owner_search = owner.clone();
            let query = dashboard.borrow().query.clone();
            self.search_input = Some(crate::text_input::TextInput::create(
                cx,
                query,
                "Search records",
                Rc::new(move |query, cx| {
                    state.borrow_mut().search(query);
                    let _ = owner_search.update(cx, |_, cx| cx.notify());
                }),
            ));
            for index in 0..benchmark_common::dashboard::FIELD_COUNT {
                let state = dashboard.clone();
                let owner_field = owner.clone();
                let initial = dashboard.borrow().fields[index].clone();
                self.field_inputs.push(crate::text_input::TextInput::create(
                    cx,
                    initial,
                    format!("Field {index:02}"),
                    Rc::new(move |value, cx| {
                        state.borrow_mut().edit(index, value);
                        let _ = owner_field.update(cx, |_, cx| cx.notify());
                    }),
                ));
            }
        }
        let dashboard = dashboard.borrow();
        let mut sidebar = div()
            .id("memory-sidebar")
            .w(px(180.))
            .h_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .track_scroll(&self.memory_sidebar_scroll)
            .bg(rgb(0xf3f4f6));
        for section in 0..SECTION_COUNT {
            let label = if section == 0 {
                "All records".to_owned()
            } else {
                format!("Section {section:02}")
            };
            sidebar = sidebar.child(
                div()
                    .id(section)
                    .h(px(22.))
                    .flex_shrink_0()
                    .text_size(px(12.))
                    .px_2()
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(dashboard) = &this.dashboard {
                            dashboard.borrow_mut().navigate(section);
                        }
                        cx.notify();
                    }))
                    .child(label),
            );
        }
        let mut thumbs = div()
            .id("memory-thumbnails")
            .w_full()
            .h(px(260.))
            .flex_shrink_0()
            .flex()
            .flex_wrap();
        for image in &self.thumbnails {
            thumbs = thumbs.child(
                div()
                    .w(px(80.))
                    .h(px(52.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        gpui::img(ImageSource::Image(image.clone()))
                            .w(px(48.))
                            .h(px(48.)),
                    ),
            );
        }
        sidebar = sidebar.child(thumbs);
        let mut records = div()
            .id("memory-records")
            .w(px(600.))
            .h_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .track_scroll(&self.memory_records_scroll)
            .bg(rgb(0xffffff));
        for index in dashboard.visible_records() {
            let record = &dashboard.fixtures.records[index];
            let title = record.title.clone();
            let body = record.body.clone();
            let selected = index == dashboard.selected_record;
            let active = dashboard.active.get(index);
            records = records.child(
                div()
                    .id(index)
                    .h(px(RECORD_HEIGHT))
                    .flex_shrink_0()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .bg(if selected {
                        rgb(0xe5edff)
                    } else {
                        rgb(0xffffff)
                    })
                    .border_b_1()
                    .border_color(rgb(0xdddddd))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(dashboard) = &this.dashboard {
                            dashboard.borrow_mut().select(index);
                        }
                        cx.notify();
                    }))
                    .child(
                        div()
                            .w(px(430.))
                            .flex()
                            .flex_col()
                            .child(div().text_sm().truncate().child(title))
                            .child(
                                div()
                                    .text_xs()
                                    .truncate()
                                    .text_color(rgb(0x666666))
                                    .child(body),
                            ),
                    )
                    .child(
                        div()
                            .id(("toggle", index))
                            .w(px(56.))
                            .h(px(26.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(if active { rgb(0x2f9e44) } else { rgb(0x868e96) })
                            .text_color(rgb(0xffffff))
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(dashboard) = &this.dashboard {
                                    dashboard.borrow_mut().toggle(index);
                                }
                                cx.notify();
                            }))
                            .child(if active { "ON" } else { "OFF" }),
                    ),
            );
        }
        let description = div()
            .id("memory-description")
            .h(px(300.))
            .flex_shrink_0()
            .overflow_y_scroll()
            .track_scroll(&self.memory_details_scroll)
            .child(
                div()
                    .text_sm()
                    .whitespace_normal()
                    .child(dashboard.detail.to_string()),
            );
        let mut fields = div()
            .id("memory-fields")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.memory_fields_scroll);
        for (index, input) in self.field_inputs.iter().enumerate() {
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
                            .child(format!("Field {index:02}")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .h(px(30.))
                            .border_1()
                            .border_color(rgb(0xaaaaaa))
                            .child(input.clone()),
                    ),
            );
        }
        let details = div()
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
            .flex()
            .flex_col()
            .bg(rgb(0xffffff))
            .child(
                div()
                    .h(px(56.))
                    .w_full()
                    .flex()
                    .items_center()
                    .px_4()
                    .gap_3()
                    .border_b_1()
                    .border_color(rgb(0xdddddd))
                    .child("Memory dashboard")
                    .child(
                        div()
                            .w(px(280.))
                            .h(px(36.))
                            .border_1()
                            .border_color(rgb(0xaaaaaa))
                            .child(optional_input(self.search_input.clone())),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(sidebar)
                    .child(records)
                    .child(details),
            )
            .into_any_element()
    }
}

impl ScenarioView for BenchView {
    const SCENARIO: Scenario = Scenario::Memory;

    fn new() -> Self {
        Self {
            dashboard: Some(std::rc::Rc::new(std::cell::RefCell::new(
                benchmark_common::dashboard::Dashboard::default(),
            ))),
            search_input: None,
            field_inputs: Vec::new(),
            memory_sidebar_scroll: gpui::ScrollHandle::new(),
            memory_records_scroll: gpui::ScrollHandle::new(),
            memory_details_scroll: gpui::ScrollHandle::new(),
            memory_fields_scroll: gpui::ScrollHandle::new(),
            thumbnails: benchmark_common::dashboard::THUMBNAILS
                .iter()
                .map(|bytes| {
                    std::sync::Arc::new(gpui::Image::from_bytes(
                        gpui::ImageFormat::Png,
                        bytes.to_vec(),
                    ))
                })
                .collect(),
            rendered: false,
        }
    }

    fn rendered(&self) -> bool {
        self.rendered
    }

    fn reset(&mut self, cx: &mut Context<Self>) {
        self.memory_records_scroll
            .set_offset(gpui::point(px(0.), px(0.)));
        self.memory_details_scroll
            .set_offset(gpui::point(px(0.), px(0.)));
        self.memory_fields_scroll
            .set_offset(gpui::point(px(0.), px(0.)));
        self.memory_sidebar_scroll
            .set_offset(gpui::point(px(0.), px(0.)));
        if let Some(dashboard) = &self.dashboard {
            dashboard.borrow_mut().reset();
            let model = dashboard.borrow();
            if let Some(input) = &self.search_input {
                input.update(cx, |input, cx| input.set_value(model.query.clone(), cx));
            }
            for (index, input) in self.field_inputs.iter().enumerate() {
                if let Some(value) = model.fields.get(index) {
                    input.update(cx, |input, cx| input.set_value(value.clone(), cx));
                }
            }
            cx.notify();
        }
    }

    fn update(&mut self, state: FrameState, _mode: SwitchMode, cx: &mut Context<Self>) -> u64 {
        if let Some(dashboard) = &self.dashboard {
            if state.running {
                dashboard.borrow_mut().advance(state);
                let model = dashboard.borrow();
                self.memory_records_scroll
                    .set_offset(gpui::point(px(0.), px(-model.scroll_offset)));
                if let Some(input) = &self.search_input {
                    if input.read(cx).value() != model.query {
                        input.update(cx, |input, cx| input.set_value(model.query.clone(), cx));
                    }
                }
                for (index, input) in self.field_inputs.iter().enumerate() {
                    if let Some(value) = model.fields.get(index) {
                        if input.read(cx).value() != value {
                            input.update(cx, |input, cx| input.set_value(value.clone(), cx));
                        }
                    }
                }
            }
        }
        if state.running {
            cx.notify();
        }
        self.dashboard
            .as_ref()
            .map_or(0, |dashboard| dashboard.borrow().applied_updates())
    }
    fn report(&self, harness: &mut benchmark_common::Harness) -> std::io::Result<()> {
        if let Some(dashboard) = &self.dashboard {
            harness.report_dashboard(&dashboard.borrow())?;
        }
        Ok(())
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run::<BenchView>()
}
