use std::{cell::Cell, rc::Rc};

use benchmark_common::{
    Scenario,
    dashboard::{Dashboard, THUMBNAILS},
};
use floem::{kurbo::Point, prelude::*, reactive::create_effect};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let model = create_rw_signal(Dashboard::default());
    let scrolling = create_rw_signal(None);
    let reset = Rc::new(Cell::new(0));
    let tick_reset = Rc::clone(&reset);
    crate::ui::run(
        Scenario::Memory,
        move |_| dashboard_view(model, scrolling).into_any(),
        move |runtime, frame| {
            if tick_reset.replace(frame.reset_generation) != frame.reset_generation {
                model.update(|state| state.reset());
                scrolling.set(Some(Point::ZERO));
            }
            if frame.running {
                model.update(|state| state.advance(frame));
            }
            let state = model.get_untracked();
            runtime.harness.borrow_mut().report_dashboard(&state)?;
            if frame.running {
                scrolling.set(Some(Point::new(0.0, f64::from(frame.scroll_offset))));
            }
            Ok(state.applied_updates())
        },
    )
}

fn dashboard_view(model: RwSignal<Dashboard>, scrolling: RwSignal<Option<Point>>) -> impl IntoView {
    let query = create_rw_signal(String::new());
    let fields: Vec<_> = (0..20)
        .map(|index| create_rw_signal(format!("Initial value {index:02}")))
        .collect();
    let field_sync = fields.clone();
    create_effect(move |_| {
        let state = model.get();
        if query.get_untracked() != state.query {
            query.set(state.query);
        }
        for (index, field) in field_sync.iter().enumerate() {
            if field.get_untracked() != state.fields[index] {
                field.set(state.fields[index].clone());
            }
        }
    });
    create_effect(move |_| {
        let value = query.get();
        if model.get_untracked().query != value {
            model.update(|state| state.search(value));
        }
    });
    for (index, field) in fields.iter().copied().enumerate() {
        create_effect(move |_| {
            let value = field.get();
            if model.get_untracked().fields[index] != value {
                model.update(|state| state.edit(index, value));
            }
        });
    }
    let sidebar = v_stack((
        v_stack_from_iter((0..20).map(move |section| {
            label(move || {
                if section == 0 {
                    "All records".to_owned()
                } else {
                    format!("Section {section:02}")
                }
            })
            .style(|style| style.width_full().height(22.0).font_size(12.0))
            .on_click_stop(move |_| model.update(|state| state.navigate(section)))
        })),
        v_stack_from_iter((0..5).map(|row| {
            h_stack_from_iter((0..2).map(move |column| {
                let bytes = THUMBNAILS[row * 2 + column];
                img(move || bytes.to_vec()).style(|style| style.width(48.0).height(48.0))
            }))
            .style(|style| style.gap(8.0))
        }))
        .style(|style| style.gap(4.0)),
    ));
    let records = dyn_stack(
        move || model.with(|state| state.visible_records()),
        |index| *index,
        move |index| {
            let record = model.with_untracked(|state| state.fixtures.records[index].clone());
            let title = record.title;
            let body = record.body;
            h_stack((
                v_stack((
                    label(move || title.clone()),
                    label(move || body.clone()).style(|style| style.font_size(12.0)),
                ))
                .style(|style| style.flex_grow(1.0).min_width(0.0).height(56.0))
                .on_click_stop(move |_| model.update(|state| state.select(index))),
                container(label(move || {
                    model.with(|state| {
                        if state.active.get(index) {
                            "ON".to_owned()
                        } else {
                            "OFF".to_owned()
                        }
                    })
                }))
                .style(|style| {
                    style
                        .width(48.0)
                        .flex_shrink(0.0)
                        .height(24.0)
                        .items_center()
                        .justify_center()
                        .background(Color::rgb8(134, 142, 150))
                        .color(Color::WHITE)
                })
                .on_click_stop(move |_| model.update(|state| state.toggle(index))),
            ))
            .style(move |style| {
                style
                    .width_full()
                    .height(64.0)
                    .min_height(64.0)
                    .padding(8.0)
                    .items_center()
                    .background(if model.with(|state| state.selected_record == index) {
                        Color::rgb8(227, 237, 255)
                    } else {
                        Color::WHITE
                    })
            })
        },
    )
    .style(|style| style.flex_col().width_full());
    let detail = scroll(
        v_stack((
            label(move || {
                model.with(|state| state.fixtures.records[state.selected_record].title.clone())
            })
            .style(|style| style.font_size(18.0)),
            label(move || model.with(|state| state.detail.to_string())).style(|style| {
                style
                    .width_full()
                    .text_overflow(floem::style::TextOverflow::Wrap)
            }),
        ))
        .style(|style| style.width_full().min_width(0.0)),
    )
    .style(|style| style.width_full().height(300.0).min_height(300.0));
    let form = scroll(
        v_stack_from_iter(fields.into_iter().enumerate().map(|(index, field)| {
            h_stack((
                label(move || format!("Field {index:02}")).style(|style| style.width(80.0)),
                text_input(field).style(|style| {
                    style
                        .flex_grow(1.0)
                        .min_width(0.0)
                        .height(32.0)
                        .border(1.0)
                        .border_color(Color::rgb8(170, 170, 170))
                }),
            ))
            .style(|style| style.width_full().height(36.0).items_center().gap(8.0))
        }))
        .style(|style| style.width_full()),
    )
    .style(|style| style.width_full().min_height(0.0).flex_grow(1.0));
    v_stack((
        h_stack((
            label(|| "Dashboard".to_owned()),
            text_input(query)
                .placeholder("Search records")
                .style(|style| {
                    style
                        .flex_grow(1.0)
                        .height(32.0)
                        .border(1.0)
                        .border_color(Color::rgb8(170, 170, 170))
                }),
        ))
        .style(|style| {
            style
                .width_full()
                .height(56.0)
                .min_height(56.0)
                .padding(8.0)
                .gap(12.0)
                .items_center()
        }),
        h_stack((
            scroll(sidebar).style(|style| {
                style
                    .width(180.0)
                    .min_width(180.0)
                    .height_full()
                    .padding(8.0)
                    .background(Color::rgb8(241, 243, 245))
            }),
            scroll(records)
                .scroll_to(move || scrolling.get())
                .style(|style| style.width(600.0).min_width(600.0).height_full()),
            v_stack((detail, form)).style(|style| {
                style
                    .flex_grow(1.0)
                    .min_width(0.0)
                    .height_full()
                    .padding(8.0)
            }),
        ))
        .style(|style| style.width_full().flex_grow(1.0).min_height(0.0)),
    ))
    .style(|style| style.size_full())
}
