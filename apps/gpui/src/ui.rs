use benchmark_common::{
    FrameState, HEIGHT, Harness, ROW_HEIGHT, SWITCH_CELL_SIZE, SWITCH_COLUMNS, SWITCH_COUNT,
    SWITCH_FONT_SIZE, SWITCH_OFF_COLOR, SWITCH_ON_COLOR, SWITCH_ROWS, SWITCH_TEXT_COLOR, Scenario,
    Switches, WIDTH, row_body, row_title,
};
use gpui::{
    App, Application, Bounds, Context, Entity, IntoElement, Render, ScrollHandle, StyleRefinement,
    UniformListScrollHandle, Window, WindowBounds, WindowOptions, div, point, prelude::*, px, rgb,
    size, uniform_list,
};
use std::{borrow::Cow, cell::RefCell, rc::Rc, time::Duration};

struct BenchView {
    harness: Rc<RefCell<Harness>>,
    scenario: Scenario,
    state: FrameState,
    switches: Rc<RefCell<Switches>>,
    switch_cells: Vec<Entity<DummyToggle>>,
    scroll: ScrollHandle,
    virtual_scroll: UniformListScrollHandle,
    memory_sidebar_scroll: ScrollHandle,
    memory_details_scroll: ScrollHandle,
    memory_fields_scroll: ScrollHandle,
    reset_generation: u64,
    rendered: bool,
    startup_input: Option<Entity<crate::input::TextInput>>,
    dashboard: Option<benchmark_common::dashboard::Dashboard>,
    dashboard_inputs: Vec<Entity<crate::input::TextInput>>,
    thumbnails: Vec<std::sync::Arc<gpui::Image>>,
    startup_submitted: bool,
}

struct DummyToggle {
    index: usize,
    on: bool,
    switches: Rc<RefCell<Switches>>,
}

impl Render for DummyToggle {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let color = if self.on {
            SWITCH_ON_COLOR
        } else {
            SWITCH_OFF_COLOR
        };
        div()
            .w_full()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .bg(rgb(((color.0 as u32) << 16)
                | ((color.1 as u32) << 8)
                | color.2 as u32))
            .text_size(px(SWITCH_FONT_SIZE))
            .text_color(rgb(((SWITCH_TEXT_COLOR.0 as u32) << 16)
                | ((SWITCH_TEXT_COLOR.1 as u32) << 8)
                | SWITCH_TEXT_COLOR.2 as u32))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.on = !this.on;
                    this.switches.borrow_mut().toggle(this.index);
                    cx.notify();
                }),
            )
            .child(if self.on { "ON" } else { "OFF" })
    }
}

impl Render for BenchView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.scenario {
            Scenario::Scroll => self.rows(),
            Scenario::VirtualScroll => self.virtual_rows(cx),
            Scenario::Switches => self.switch_grid(),
            Scenario::Startup => self.startup(cx),
            Scenario::Memory => self.memory(cx),
        };
        self.rendered = true;
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

    fn switch_grid(&self) -> gpui::AnyElement {
        let mut grid = div().size_full().flex().flex_col().bg(rgb(0xffffff));
        for row_index in 0..SWITCH_ROWS {
            let mut line = div()
                .flex()
                .w(px(SWITCH_CELL_SIZE * SWITCH_COLUMNS as f32))
                .h(px(SWITCH_CELL_SIZE))
                .flex_shrink_0();
            for column in 0..SWITCH_COLUMNS {
                let index = row_index * SWITCH_COLUMNS + column;
                line = line.child(
                    div().size(px(SWITCH_CELL_SIZE)).flex_shrink_0().child(
                        gpui::AnyView::from(self.switch_cells[index].clone())
                            .cached(StyleRefinement::default()),
                    ),
                );
            }
            grid = grid.child(line);
        }
        grid.into_any_element()
    }
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
    Application::new().run(move |cx: &mut App| {
        crate::input::bind_keys(cx);
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
        let switches = Rc::new(RefCell::new(Switches::default()));
        let switch_cells = if scenario == Scenario::Switches {
            (0..SWITCH_COUNT)
                .map(|index| {
                    let switches = switches.clone();
                    cx.new(|_| DummyToggle {
                        index,
                        on: false,
                        switches,
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        let startup_input = if scenario == Scenario::Startup {
            Some(cx.new(|cx| crate::input::TextInput::new(String::new(), cx)))
        } else {
            None
        };
        let dashboard =
            (scenario == Scenario::Memory).then(benchmark_common::dashboard::Dashboard::default);
        let dashboard_inputs = if let Some(model) = dashboard.as_ref() {
            let mut inputs = Vec::with_capacity(21);
            inputs.push(cx.new(|cx| crate::input::TextInput::new(model.query.clone(), cx)));
            for value in &model.fields {
                inputs.push(cx.new(|cx| crate::input::TextInput::new(value.clone(), cx)));
            }
            inputs
        } else {
            Vec::new()
        };
        let thumbnails = if scenario == Scenario::Memory {
            benchmark_common::dashboard::THUMBNAILS
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
        };
        let entity: Entity<BenchView> = cx.new(|_| BenchView {
            harness: app_harness.clone(),
            scenario,
            state: FrameState::default(),
            switches,
            switch_cells,
            scroll: ScrollHandle::new(),
            virtual_scroll: UniformListScrollHandle::new(),
            memory_sidebar_scroll: ScrollHandle::new(),
            memory_details_scroll: ScrollHandle::new(),
            memory_fields_scroll: ScrollHandle::new(),
            reset_generation: 0,
            rendered: false,
            startup_input,
            dashboard,
            dashboard_inputs,
            thumbnails,
            startup_submitted: false,
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
                            gpui::Timer::after(Duration::from_millis(16)).await;
                            let done = match cx.update(|_, cx| {
                                task_entity.update(cx, |view, cx| {
                                    let state = match view.harness.borrow_mut().poll() {
                                        Ok(state) => state,
                                        Err(failure) => {
                                            *error.borrow_mut() = Some(format!(
                                                "GPUI benchmark protocol failure: {failure}"
                                            ));
                                            cx.quit();
                                            return true;
                                        }
                                    };
                                    if state.reset_generation != view.reset_generation {
                                        view.startup_submitted = false;
                                        if let Some(input) = view.startup_input.as_ref() {
                                            input.update(cx, |input, cx| {
                                                input.set_value(String::new(), cx)
                                            });
                                        }
                                        *view.switches.borrow_mut() = Switches::default();
                                        view.scroll.set_offset(point(px(0.), px(0.)));
                                        view.memory_sidebar_scroll
                                            .set_offset(point(px(0.), px(0.)));
                                        view.memory_details_scroll
                                            .set_offset(point(px(0.), px(0.)));
                                        view.memory_fields_scroll.set_offset(point(px(0.), px(0.)));
                                        view.virtual_scroll
                                            .0
                                            .borrow()
                                            .base_handle
                                            .set_offset(point(px(0.), px(0.)));
                                        if let Some(dashboard) = view.dashboard.as_mut() {
                                            dashboard.reset();
                                        }
                                        if let Some(input) = view.dashboard_inputs.first() {
                                            input.update(cx, |input, cx| {
                                                input.set_value(String::new(), cx)
                                            });
                                        }
                                        for (input, value) in
                                            view.dashboard_inputs.iter().skip(1).zip(
                                                view.dashboard
                                                    .as_ref()
                                                    .into_iter()
                                                    .flat_map(|model| model.fields.iter()),
                                            )
                                        {
                                            input.update(cx, |input, cx| {
                                                input.set_value(value.clone(), cx)
                                            });
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
                                    if scenario == Scenario::Memory {
                                        if let (Some(model), Some(search)) =
                                            (view.dashboard.as_mut(), view.dashboard_inputs.first())
                                        {
                                            model.search(search.read(cx).value().to_owned());
                                        }
                                        if let Some(model) = view.dashboard.as_mut() {
                                            for (index, input) in
                                                view.dashboard_inputs.iter().skip(1).enumerate()
                                            {
                                                model
                                                    .edit(index, input.read(cx).value().to_owned());
                                            }
                                            if state.running {
                                                model.advance(state);
                                            }
                                            if let Some(search) = view.dashboard_inputs.first() {
                                                if search.read(cx).value() != model.query {
                                                    search.update(cx, |input, cx| {
                                                        input.set_value(model.query.clone(), cx)
                                                    });
                                                }
                                            }
                                            for (input, value) in view
                                                .dashboard_inputs
                                                .iter()
                                                .skip(1)
                                                .zip(&model.fields)
                                            {
                                                if input.read(cx).value() != value {
                                                    input.update(cx, |input, cx| {
                                                        input.set_value(value.clone(), cx)
                                                    });
                                                }
                                            }
                                        }
                                    }
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
                                    if scenario == Scenario::Memory && state.running {
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
                                    if scenario == Scenario::Switches {
                                        let mode = view.harness.borrow().switch_mode();
                                        let cells = &view.switch_cells;
                                        view.switches.borrow_mut().advance_changed(
                                            state.update_index,
                                            mode,
                                            |index, value| {
                                                cells[index].update(cx, |cell, cx| {
                                                    cell.on = value;
                                                    cx.notify();
                                                });
                                            },
                                        );
                                    }
                                    let applied = if scenario == Scenario::Switches {
                                        view.switches.borrow().applied_updates()
                                    } else if scenario == Scenario::Memory {
                                        view.dashboard.as_ref().map_or(
                                            0,
                                            benchmark_common::dashboard::Dashboard::applied_updates,
                                        )
                                    } else {
                                        state.update_index
                                    };
                                    if let Err(failure) =
                                        view.harness.borrow_mut().report_applied(applied)
                                    {
                                        *error.borrow_mut() = Some(format!(
                                            "GPUI benchmark protocol failure: {failure}"
                                        ));
                                        cx.quit();
                                        return true;
                                    }
                                    if scenario == Scenario::Memory {
                                        if let Some(dashboard) = view.dashboard.as_ref() {
                                            if let Err(failure) = view
                                                .harness
                                                .borrow_mut()
                                                .report_dashboard(dashboard)
                                            {
                                                *error.borrow_mut() = Some(format!(
                                                    "GPUI dashboard reporting failure: {failure}"
                                                ));
                                                cx.quit();
                                                return true;
                                            }
                                        }
                                    }
                                    if scenario != Scenario::Switches {
                                        cx.notify();
                                    }
                                    false
                                })
                            }) {
                                Ok(done) => done,
                                Err(failure) => {
                                    *error.borrow_mut() = Some(format!(
                                        "GPUI benchmark event loop failure: {failure}"
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
            *app_error.borrow_mut() =
                Some(format!("Could not open GPUI benchmark window: {failure}"));
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
