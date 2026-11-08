use benchmark_common::{
    HEIGHT, Harness, ROW_HEIGHT, SWITCH_CELL_SIZE, SWITCH_COLUMNS, SWITCH_FONT_SIZE,
    SWITCH_OFF_COLOR, SWITCH_ON_COLOR, SWITCH_ROWS, SWITCH_TEXT_COLOR, Scenario, Switches, WIDTH,
    row_body, row_title,
};
use std::{cell::RefCell, rc::Rc, time::Duration};
use vizia::prelude::*;

pub fn run(scenario: Scenario) -> Result<(), Box<dyn std::error::Error>> {
    match scenario {
        Scenario::Startup
        | Scenario::Scroll
        | Scenario::Switches
        | Scenario::Memory
        | Scenario::VirtualScroll => {}
    }

    let harness = Rc::new(RefCell::new(Some(Harness::connect(scenario)?)));
    let failure = Rc::new(RefCell::new(None::<String>));
    let timer_harness = Rc::clone(&harness);
    let timer_failure = Rc::clone(&failure);
    let application = Application::new(move |cx| {
        let font = include_bytes!("../../../assets/fonts/DejaVuSans.ttf");
        cx.add_font_mem(font);
        let scroll_write = Rc::new(RefCell::new(None::<WriteSignal<f32>>));
        let switch_values = Rc::new(RefCell::new(
            Vec::<(WriteSignal<bool>, WriteSignal<String>)>::new(),
        ));
        let switch_state = Signal::new(Switches::default());
        let observed_reset = Rc::new(RefCell::new(0_u64));
        let dashboard = (scenario == Scenario::Memory)
            .then(|| Signal::new(benchmark_common::dashboard::Dashboard::default()));
        let dashboard_for_timer = dashboard.clone();
        let scroll_signal_for_timer = Rc::clone(&scroll_write);
        let values_for_timer = Rc::clone(&switch_values);
        let state_for_timer = switch_state;
        let reset_for_timer = Rc::clone(&observed_reset);

        VStack::new(cx, |cx| match scenario {
            Scenario::Startup => {
                Label::new(cx, "Vizia benchmark startup");
                Label::new(cx, "A deterministic benchmark screen");
                let search = Signal::new(String::new());
                Textbox::new(cx, search)
                    .on_edit(move |_, value| search.set(value))
                    .width(Pixels(430.0))
                    .height(Pixels(32.0));
                let applied = Signal::new(0usize);
                Button::new(cx, |cx| Label::new(cx, "Apply"))
                    .on_press(move |_| applied.update(|value| *value += 1));
                ScrollView::new(cx, |cx| build_rows(cx, 20));
            }
            Scenario::Scroll => {
                let scroll_y = Signal::new(0.0_f32);
                *scroll_write.borrow_mut() = Some(scroll_y.write_only());
                ScrollView::new(cx, |cx| build_rows(cx, 2_000)).scroll_y(scroll_y);
            }
            Scenario::VirtualScroll => {
                let records = Signal::new((0..100_000).collect::<Vec<usize>>());
                let scroll_y = Signal::new(0.0_f32);
                *scroll_write.borrow_mut() = Some(scroll_y.write_only());
                VirtualList::new(cx, records, 48.0, |cx, _visible_index, record_index| {
                    let index = record_index.get();
                    HStack::new(cx, |cx| {
                        VStack::new(cx, |cx| {
                            Label::new(cx, row_title(index));
                            Label::new(cx, row_body(index));
                        });
                    })
                })
                .scroll_y(scroll_y)
                .height(Pixels(HEIGHT))
                .width(Pixels(WIDTH));
            }
            Scenario::Switches => {
                for _ in 0..SWITCH_ROWS {
                    HStack::new(cx, |cx| {
                        for _ in 0..SWITCH_COLUMNS {
                            let value = Signal::new(false);
                            let text = Signal::new(String::from("OFF"));
                            let index = switch_values.borrow().len();
                            switch_values
                                .borrow_mut()
                                .push((value.write_only(), text.write_only()));
                            DummyToggle::new(cx, value, text, index, switch_state.clone());
                        }
                    })
                    .height(Pixels(SWITCH_CELL_SIZE))
                    .width(Pixels(SWITCH_CELL_SIZE * SWITCH_COLUMNS as f32));
                }
            }
            Scenario::Memory => {
                if let Some(dashboard) = dashboard.clone() {
                    let search = dashboard.map(|state| state.query.clone());
                    HStack::new(cx, |cx| {
                        Label::new(cx, "Dashboard").width(Pixels(120.0));
                        Textbox::new(cx, search)
                            .on_edit({
                                let dashboard = dashboard.clone();
                                move |_, query| dashboard.update(|state| state.search(query))
                            })
                            .width(Stretch(1.0))
                            .height(Pixels(32.0));
                    })
                    .height(Pixels(56.0))
                    .width(Stretch(1.0))
                    .alignment(Alignment::Center)
                    .padding(Pixels(8.0));
                    HStack::new(cx, |cx| {
                        VStack::new(cx, |cx| {
                            for row in 0..(benchmark_common::dashboard::THUMBNAIL_COUNT / 2) {
                                HStack::new(cx, |cx| {
                                    for column in 0..2 {
                                        let thumbnail = row * 2 + column;
                                        let path = format!("dashboard-thumbnail-{thumbnail}");
                                        cx.load_image(
                                            &path,
                                            benchmark_common::dashboard::THUMBNAILS[thumbnail],
                                            ImageRetentionPolicy::DropWhenNoObservers,
                                        );
                                        Image::new(cx, path)
                                            .width(Pixels(48.0))
                                            .height(Pixels(48.0));
                                    }
                                });
                            }
                            for section in 0..benchmark_common::dashboard::SECTION_COUNT {
                                let model = dashboard.clone();
                                Button::new(cx, move |cx| {
                                    Label::new(cx, format!("Section {section:02}"))
                                })
                                .on_press(move |_| model.update(|state| state.navigate(section)))
                                .height(Pixels(22.0))
                                .padding(Pixels(2.0))
                                .font_size(12.0);
                            }
                        })
                        .width(Pixels(180.0))
                        .height(Stretch(1.0));
                        let visible = dashboard.map(|state| state.visible_records());
                        let scroll_position = dashboard.map(|state| {
                            let maximum = (state.visible_records().len() as f32 * 64.0
                                - (HEIGHT - 56.0))
                                .max(0.0);
                            if maximum > 0.0 {
                                (state.scroll_offset / maximum).clamp(0.0, 1.0)
                            } else {
                                0.0
                            }
                        });
                        ScrollView::new(cx, move |cx| {
                            Binding::new(cx, visible, {
                                let model = dashboard.clone();
                                move |cx| {
                                    for index in visible.get() {
                                        let select_action = model.clone();
                                        let is_selected =
                                            model.map(move |state| state.selected_record == index);
                                        let active =
                                            model.map(move |state| state.active.get(index));
                                        let title = model.map(move |state| {
                                            state.fixtures.records[index].title.clone()
                                        });
                                        let body = model.map(move |state| {
                                            state.fixtures.records[index].body.clone()
                                        });
                                        HStack::new(cx, |cx| {
                                            Button::new(cx, move |cx| {
                                                VStack::new(cx, |cx| {
                                                    Label::new(cx, title)
                                                        .font_size(14.0)
                                                        .width(Stretch(1.0));
                                                    Label::new(cx, body)
                                                        .font_size(12.0)
                                                        .width(Stretch(1.0))
                                                        .text_wrap(true);
                                                })
                                                .width(Pixels(500.0))
                                            })
                                            .width(Stretch(1.0))
                                            .height(Pixels(64.0))
                                            .padding(Pixels(4.0))
                                            .color(Color::rgb(0, 0, 0))
                                            .bind(is_selected.clone(), move |handle| {
                                                handle.background_color(if is_selected.get() {
                                                    Color::rgb(210, 225, 245)
                                                } else {
                                                    Color::rgb(255, 255, 255)
                                                });
                                            })
                                            .on_press(move |_| {
                                                select_action.update(|state| state.select(index))
                                            });
                                            let toggler = model.clone();
                                            Button::new(cx, move |cx| {
                                                Label::new(
                                                    cx,
                                                    active.map(|enabled| {
                                                        if *enabled {
                                                            "ON".to_owned()
                                                        } else {
                                                            "OFF".to_owned()
                                                        }
                                                    }),
                                                )
                                            })
                                            .on_press(move |_| {
                                                toggler.update(|state| state.toggle(index))
                                            });
                                        })
                                        .height(Pixels(64.0));
                                    }
                                }
                            });
                        })
                        .width(Pixels(600.0))
                        .height(Stretch(1.0))
                        .scroll_y(scroll_position);
                        VStack::new(cx, |cx| {
                            let detail = dashboard.map(|state| state.detail.to_string());
                            ScrollView::new(cx, move |cx| {
                                Label::new(cx, detail).width(Stretch(1.0)).text_wrap(true);
                            })
                            .height(Pixels(300.0))
                            .width(Stretch(1.0));
                            let form_dashboard = dashboard.clone();
                            ScrollView::new(cx, move |cx| {
                                for field in 0..benchmark_common::dashboard::FIELD_COUNT {
                                    let value = form_dashboard
                                        .map(move |state| state.fields[field].clone());
                                    let model = form_dashboard.clone();
                                    HStack::new(cx, |cx| {
                                        Label::new(cx, format!("Field {field:02}"))
                                            .width(Pixels(80.0));
                                        Textbox::new(cx, value)
                                            .on_edit(move |_, text| {
                                                model.update(|state| state.edit(field, text));
                                            })
                                            .width(Stretch(1.0));
                                    })
                                    .height(Pixels(34.0));
                                }
                            })
                            .height(Stretch(1.0));
                        })
                        .width(Stretch(1.0))
                        .height(Stretch(1.0));
                    })
                    .width(Stretch(1.0))
                    .height(Stretch(1.0));
                }
            }
        })
        .width(Stretch(1.0))
        .height(Stretch(1.0))
        .font_family(vec![FamilyOwned::Named("DejaVu Sans".to_owned())]);

        let ready_sent = std::cell::Cell::new(false);
        let polling_timer = cx.add_timer(Duration::from_millis(16), None, move |cx, _| {
            let result = (|| -> std::io::Result<()> {
                let mut borrowed = timer_harness.borrow_mut();
                let Some(harness) = borrowed.as_mut() else {
                    return Ok(());
                };
                if !ready_sent.get() {
                    harness.mark_ready("event-loop-after-renderer-init")?;
                    ready_sent.set(true);
                }
                let frame = harness.poll()?;
                let reset_changed = {
                    let mut observed = reset_for_timer.borrow_mut();
                    if *observed == frame.reset_generation {
                        false
                    } else {
                        *observed = frame.reset_generation;
                        true
                    }
                };
                if scenario == Scenario::Memory {
                    if let Some(dashboard) = dashboard_for_timer.as_ref() {
                        let mut state = dashboard.get();
                        if reset_changed {
                            state.reset();
                        }
                        if frame.running {
                            state.advance(frame);
                        }
                        dashboard.set_if_changed(state);
                        harness.report_dashboard(&dashboard.get())?;
                    }
                }
                if reset_changed {
                    if let Some(scroll) = *scroll_signal_for_timer.borrow() {
                        scroll.set(0.0);
                    }
                    let model = state_for_timer.get();
                    let values = values_for_timer.borrow();
                    for (index, (signal, text)) in values.iter().enumerate() {
                        if model.get(index) {
                            signal.set(false);
                            text.set("OFF".to_owned());
                        }
                    }
                    state_for_timer.set(Switches::default());
                }

                if (scenario == Scenario::Scroll || scenario == Scenario::VirtualScroll)
                    && frame.running
                {
                    if let Some(scroll) = *scroll_signal_for_timer.borrow() {
                        let row_count = if scenario == Scenario::VirtualScroll {
                            100_000.0
                        } else {
                            2_000.0
                        };
                        let row_height = if scenario == Scenario::VirtualScroll {
                            48.0
                        } else {
                            ROW_HEIGHT
                        };
                        let maximum = (row_count * row_height - HEIGHT).max(0.0);
                        scroll.set(if maximum > 0.0 {
                            frame.scroll_offset / maximum
                        } else {
                            0.0
                        });
                    }
                }

                if scenario == Scenario::Switches && frame.running {
                    let mut state = state_for_timer.get();
                    let values = values_for_timer.borrow();
                    state.advance_changed(
                        frame.update_index,
                        harness.switch_mode(),
                        |index, value| {
                            if let Some((signal, text)) = values.get(index) {
                                signal.set(value);
                                text.set(if value { "ON" } else { "OFF" }.to_owned());
                            }
                        },
                    );
                    state_for_timer.set(state);
                }
                let applied = match scenario {
                    Scenario::Switches => state_for_timer.get().applied_updates(),
                    Scenario::Memory => dashboard_for_timer
                        .as_ref()
                        .map_or(0, |dashboard| dashboard.get().applied_updates()),
                    _ => frame.update_index,
                };
                harness.report_applied(applied)?;
                if frame.should_quit {
                    cx.emit(WindowEvent::WindowClose);
                }
                Ok(())
            })();
            if let Err(error) = result {
                *timer_failure.borrow_mut() = Some(error.to_string());
                cx.emit(WindowEvent::WindowClose);
            }
        });
        cx.start_timer(polling_timer);
    })
    .inner_size((1280, 800));

    let event_loop_result = application.run();
    if let Some(error) = failure.borrow_mut().take() {
        return Err(error.into());
    }
    event_loop_result?;
    Ok(())
}

struct DummyToggle;

impl DummyToggle {
    fn new(
        cx: &mut Context,
        value: Signal<bool>,
        text: Signal<String>,
        index: usize,
        model: Signal<Switches>,
    ) {
        let background = value.clone();
        let click_value = value.clone();
        let click_text = text.clone();
        Self.build(cx, |cx| {
            Label::new(cx, text)
                .font_family(vec![FamilyOwned::Named("DejaVu Sans".to_owned())])
                .font_size(SWITCH_FONT_SIZE)
                .text_align(TextAlign::Center)
                .color(Color::rgb(
                    SWITCH_TEXT_COLOR.0,
                    SWITCH_TEXT_COLOR.1,
                    SWITCH_TEXT_COLOR.2,
                ));
        })
        .width(Pixels(SWITCH_CELL_SIZE))
        .height(Pixels(SWITCH_CELL_SIZE))
        .alignment(Alignment::Center)
        .bind(background.clone(), move |handle| {
            let color = if background.get() {
                Color::rgb(SWITCH_ON_COLOR.0, SWITCH_ON_COLOR.1, SWITCH_ON_COLOR.2)
            } else {
                Color::rgb(SWITCH_OFF_COLOR.0, SWITCH_OFF_COLOR.1, SWITCH_OFF_COLOR.2)
            };
            handle.background_color(color);
        })
        .on_press(move |_| {
            click_value.update(|current| *current = !*current);
            model.update(|model| model.toggle(index));
            click_text
                .update(|text| *text = if click_value.get() { "ON" } else { "OFF" }.to_owned());
        });
    }
}

impl View for DummyToggle {}

fn build_rows(cx: &mut Context, count: usize) {
    for index in 0..count {
        VStack::new(cx, |cx| {
            Label::new(cx, row_title(index));
            Label::new(cx, row_body(index));
        })
        .height(Pixels(ROW_HEIGHT));
    }
}
