use benchmark_common::{FrameState, HEIGHT, Harness, Scenario};
use vizia::prelude::*;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run::<Memory>()
}
struct Memory;
impl crate::ui::Adapter for Memory {
    type State = Signal<benchmark_common::dashboard::Dashboard>;
    const SCENARIO: Scenario = Scenario::Memory;
    fn state() -> Self::State {
        Signal::new(benchmark_common::dashboard::Dashboard::default())
    }
    fn build(cx: &mut Context, dashboard_state: &Self::State) {
        let dashboard: Signal<benchmark_common::dashboard::Dashboard> = dashboard_state.clone();
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
                let maximum =
                    (state.visible_records().len() as f32 * 64.0 - (HEIGHT - 56.0)).max(0.0);
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
                            let active = model.map(move |state| state.active.get(index));
                            let title =
                                model.map(move |state| state.fixtures.records[index].title.clone());
                            let body =
                                model.map(move |state| state.fixtures.records[index].body.clone());
                            HStack::new(cx, |cx| {
                                Button::new(cx, move |cx| {
                                    VStack::new(cx, |cx| {
                                        Label::new(cx, title).font_size(14.0).width(Stretch(1.0));
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
                                .on_press(move |_| toggler.update(|state| state.toggle(index)));
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
                        let value = form_dashboard.map(move |state| state.fields[field].clone());
                        let model = form_dashboard.clone();
                        HStack::new(cx, |cx| {
                            Label::new(cx, format!("Field {field:02}")).width(Pixels(80.0));
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
    fn update(
        dashboard: &Self::State,
        frame: FrameState,
        harness: &mut Harness,
        reset: bool,
    ) -> std::io::Result<u64> {
        let mut state = dashboard.get();
        if reset {
            state.reset();
        }
        if frame.running {
            state.advance(frame);
        }
        dashboard.set_if_changed(state);
        harness.report_dashboard(&dashboard.get())?;
        Ok(dashboard.get().applied_updates())
    }
}
