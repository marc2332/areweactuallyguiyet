use benchmark_common::dashboard::{Dashboard, RECORD_HEIGHT, SECTION_COUNT, THUMBNAILS};
use benchmark_common::{
    FrameState, HEIGHT, Harness, ROW_HEIGHT, SWITCH_CELL_SIZE, SWITCH_COLUMNS, SWITCH_COUNT,
    SWITCH_FONT_SIZE, SWITCH_OFF_COLOR, SWITCH_ON_COLOR, SWITCH_ROWS, SWITCH_TEXT_COLOR, Scenario,
    Switches, WIDTH, row_body, row_title,
};
use gpui::{
    App, Bounds, Context, Entity, ImageSource, InteractiveElement, IntoElement, Render,
    ScrollHandle, UniformListScrollHandle, Window, WindowBounds, WindowOptions, div, point,
    prelude::*, px, rgb, size, uniform_list,
};
use std::{borrow::Cow, cell::RefCell, rc::Rc, time::Duration};

struct BenchView {
    harness: Rc<RefCell<Harness>>,
    scenario: Scenario,
    state: FrameState,
    switches: Rc<RefCell<Switches>>,
    scroll: ScrollHandle,
    virtual_scroll: UniformListScrollHandle,
    switch_cells: Vec<Entity<SwitchCell>>,
    reset_generation: u64,
    rendered: bool,
    dashboard: Option<Rc<RefCell<Dashboard>>>,
    search_input: Option<Entity<crate::text_input::TextInput>>,
    field_inputs: Vec<Entity<crate::text_input::TextInput>>,
    startup_input: Option<Entity<crate::text_input::TextInput>>,
    startup_submitted: bool,
    memory_sidebar_scroll: ScrollHandle,
    memory_records_scroll: ScrollHandle,
    memory_details_scroll: ScrollHandle,
    memory_fields_scroll: ScrollHandle,
    thumbnails: Vec<std::sync::Arc<gpui::Image>>,
}

impl Render for BenchView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.scenario {
            Scenario::Scroll => self.rows(),
            Scenario::VirtualScroll => self.virtual_rows(cx),
            Scenario::Switches => self.switch_grid(cx),
            Scenario::Startup => self.startup(cx),
            Scenario::Memory => self.memory(cx),
        };
        self.rendered = true;
        div().size_full().font_family("DejaVu Sans").child(content)
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

fn optional_input(input: Option<Entity<crate::text_input::TextInput>>) -> gpui::AnyElement {
    match input {
        Some(input) => input.into_any_element(),
        None => div().into_any_element(),
    }
}

fn color(value: (u8, u8, u8)) -> u32 {
    (u32::from(value.0) << 16) | (u32::from(value.1) << 8) | u32::from(value.2)
}

fn row(index: usize) -> gpui::Div {
    div()
        .h(px(ROW_HEIGHT))
        .flex_shrink_0()
        .w_full()
        .flex()
        .flex_col()
        .justify_center()
        .border_b_1()
        .border_color(rgb(0xdddddd))
        .px_3()
        .child(div().text_sm().truncate().child(row_title(index)))
        .child(
            div()
                .text_xs()
                .truncate()
                .text_color(rgb(0x666666))
                .child(row_body(index)),
        )
}

pub fn run(scenario: Scenario) -> Result<(), Box<dyn std::error::Error>> {
    let harness = Rc::new(RefCell::new(Harness::connect(scenario)?));
    let error = Rc::new(RefCell::new(None::<String>));
    let app_harness = harness.clone();
    let app_error = error.clone();
    gpui_platform::application().run(move |cx: &mut App| {
        if matches!(scenario, Scenario::Startup | Scenario::Memory) {
            crate::text_input::bind_keys(cx);
        }
        if let Err(failure) = cx
            .text_system()
            .add_fonts(vec![Cow::Borrowed(include_bytes!(
                "../../../assets/fonts/DejaVuSans.ttf"
            ))])
        {
            *app_error.borrow_mut() = Some(format!("Could not load benchmark font: {failure}"));
            cx.quit();
            return;
        }
        let bounds = Bounds::centered(None, size(px(WIDTH), px(HEIGHT)), cx);
        let entity: Entity<BenchView> = cx.new(|_| BenchView {
            harness: app_harness.clone(),
            scenario,
            state: FrameState::default(),
            switches: Rc::new(RefCell::new(Switches::default())),
            scroll: ScrollHandle::new(),
            virtual_scroll: UniformListScrollHandle::new(),
            switch_cells: Vec::new(),
            reset_generation: 0,
            rendered: false,
            dashboard: (scenario == Scenario::Memory)
                .then(|| Rc::new(RefCell::new(Dashboard::default()))),
            search_input: None,
            field_inputs: Vec::new(),
            startup_input: None,
            startup_submitted: false,
            memory_sidebar_scroll: ScrollHandle::new(),
            memory_records_scroll: ScrollHandle::new(),
            memory_details_scroll: ScrollHandle::new(),
            memory_fields_scroll: ScrollHandle::new(),
            thumbnails: if scenario == Scenario::Memory {
                THUMBNAILS
                    .iter()
                    .map(|bytes| {
                        std::sync::Arc::new(gpui::Image::from_bytes(
                            gpui::ImageFormat::Png,
                            bytes.to_vec(),
                        ))
                    })
                    .collect()
            } else {
                Vec::new()
            },
        });
        let task_error = app_error.clone();
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |window, cx| {
                let entity = entity.clone();
                let task_entity = entity.clone();
                let error = task_error.clone();
                window
                    .spawn(cx, async move |cx| {
                        loop {
                            gpui_platform::background_executor()
                                .timer(Duration::from_millis(16))
                                .await;
                            let done = match cx.update(|_, cx| {
                                task_entity.update(cx, |view, cx| {
                                    let state = match view.harness.borrow_mut().poll() {
                                        Ok(state) => state,
                                        Err(failure) => {
                                            *error.borrow_mut() = Some(format!(
                                                "GPUI Fast benchmark protocol failure: {failure}"
                                            ));
                                            cx.quit();
                                            return true;
                                        }
                                    };
                                    if state.reset_generation != view.reset_generation {
                                        *view.switches.borrow_mut() = Switches::default();
                                        view.scroll.set_offset(point(px(0.), px(0.)));
                                        view.memory_records_scroll
                                            .set_offset(point(px(0.), px(0.)));
                                        view.memory_details_scroll
                                            .set_offset(point(px(0.), px(0.)));
                                        view.memory_fields_scroll.set_offset(point(px(0.), px(0.)));
                                        view.memory_sidebar_scroll
                                            .set_offset(point(px(0.), px(0.)));
                                        view.virtual_scroll
                                            .0
                                            .borrow()
                                            .base_handle
                                            .set_offset(point(px(0.), px(0.)));
                                        if let Some(dashboard) = &view.dashboard {
                                            dashboard.borrow_mut().reset();
                                            let model = dashboard.borrow();
                                            if let Some(input) = &view.search_input {
                                                input.update(cx, |input, cx| {
                                                    input.set_value(model.query.clone(), cx)
                                                });
                                            }
                                            for (index, input) in
                                                view.field_inputs.iter().enumerate()
                                            {
                                                if let Some(value) = model.fields.get(index) {
                                                    input.update(cx, |input, cx| {
                                                        input.set_value(value.clone(), cx)
                                                    });
                                                }
                                            }
                                            cx.notify();
                                        }
                                        for cell in &view.switch_cells {
                                            cell.update(cx, |cell, cx| {
                                                if cell.on {
                                                    cell.on = false;
                                                    cx.notify();
                                                }
                                            });
                                        }
                                        view.reset_generation = state.reset_generation;
                                    }
                                    view.state = state;
                                    if view.rendered {
                                        if let Err(failure) = view
                                            .harness
                                            .borrow_mut()
                                            .mark_ready("initial-render-returned-not-presented")
                                        {
                                            *error.borrow_mut() = Some(failure.to_string());
                                            cx.quit();
                                            return true;
                                        }
                                    }
                                    if state.should_quit {
                                        cx.quit();
                                        return true;
                                    }
                                    if scenario == Scenario::Scroll && state.running {
                                        view.scroll
                                            .set_offset(point(px(0.), px(-state.scroll_offset)));
                                    }
                                    if scenario == Scenario::VirtualScroll && state.running {
                                        view.virtual_scroll
                                            .0
                                            .borrow()
                                            .base_handle
                                            .set_offset(point(px(0.), px(-state.scroll_offset)));
                                    }
                                    if scenario == Scenario::Memory {
                                        if let Some(dashboard) = &view.dashboard {
                                            if state.running {
                                                dashboard.borrow_mut().advance(state);
                                                let model = dashboard.borrow();
                                                view.memory_records_scroll.set_offset(point(
                                                    px(0.),
                                                    px(-model.scroll_offset),
                                                ));
                                                if let Some(input) = &view.search_input {
                                                    if input.read(cx).value() != model.query {
                                                        input.update(cx, |input, cx| {
                                                            input.set_value(model.query.clone(), cx)
                                                        });
                                                    }
                                                }
                                                for (index, input) in
                                                    view.field_inputs.iter().enumerate()
                                                {
                                                    if let Some(value) = model.fields.get(index) {
                                                        if input.read(cx).value() != value {
                                                            input.update(cx, |input, cx| {
                                                                input.set_value(value.clone(), cx)
                                                            });
                                                        }
                                                    }
                                                }
                                            }
                                            if let Err(failure) = view
                                                .harness
                                                .borrow_mut()
                                                .report_dashboard(&dashboard.borrow())
                                            {
                                                *error.borrow_mut() = Some(format!(
                                                    "GPUI Fast dashboard report failure: {failure}"
                                                ));
                                                cx.quit();
                                                return true;
                                            }
                                        }
                                        if state.running {
                                            cx.notify();
                                        }
                                    }
                                    if scenario == Scenario::Switches {
                                        let mode = view.harness.borrow().switch_mode();
                                        view.switches.borrow_mut().advance_changed(
                                            state.update_index,
                                            mode,
                                            |index, value| {
                                                view.switch_cells[index].update(cx, |cell, cx| {
                                                    cell.on = value;
                                                    cx.notify();
                                                });
                                            },
                                        );
                                    }
                                    let applied = match scenario {
                                        Scenario::Switches => {
                                            view.switches.borrow().applied_updates()
                                        }
                                        Scenario::Memory => {
                                            view.dashboard.as_ref().map_or(0, |dashboard| {
                                                dashboard.borrow().applied_updates()
                                            })
                                        }
                                        _ => state.update_index,
                                    };
                                    if let Err(failure) =
                                        view.harness.borrow_mut().report_applied(applied)
                                    {
                                        *error.borrow_mut() = Some(format!(
                                            "GPUI Fast benchmark protocol failure: {failure}"
                                        ));
                                        cx.quit();
                                        return true;
                                    }
                                    false
                                })
                            }) {
                                Ok(done) => done,
                                Err(failure) => {
                                    *error.borrow_mut() = Some(format!(
                                        "GPUI Fast benchmark event loop failure: {failure}"
                                    ));
                                    break;
                                }
                            };
                            if done {
                                break;
                            }
                        }
                    })
                    .detach();
                entity
            },
        );
        if let Err(failure) = opened {
            *app_error.borrow_mut() = Some(format!(
                "Could not open GPUI Fast benchmark window: {failure}"
            ));
            cx.quit();
            return;
        }
        cx.activate(true);
    });
    if let Some(failure) = error.borrow_mut().take() {
        return Err(failure.into());
    }
    Ok(())
}
