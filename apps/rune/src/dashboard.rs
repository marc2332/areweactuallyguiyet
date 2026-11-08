use benchmark_common::dashboard::{
    HEADER_HEIGHT, RECORD_HEIGHT, RECORDS_WIDTH, SECTION_COUNT, SIDEBAR_WIDTH,
};
use rune::{Color, El, ScrollSpec, TextStyle, col, el, row, text_input};

use crate::{
    selected_memory::Memory,
    ui::{Benchmark, color, label},
};

type MemoryState = Benchmark<Memory>;

fn input_style() -> TextStyle {
    TextStyle::new(14.0, 400.0)
}

pub fn view(state: &MemoryState) -> El {
    let dashboard = &state.content.dashboard;
    let search = text_input(&state.content.search, input_style())
        .key("search")
        .grow(1.0)
        .h(36.0)
        .single_line()
        .color(Color::BLACK)
        .fill(color((245, 245, 245)))
        .padding(6.0)
        .on_edit(|state: &mut MemoryState| &mut state.content.search)
        .on_input_change(|state: &mut MemoryState, transaction| {
            state
                .content
                .dashboard
                .search(state.content.search.text.clone());
            transaction.instant();
            transaction.scroll_to_y("records", 0.0);
        });
    let header = row()
        .w_full()
        .h(HEADER_HEIGHT)
        .center_y()
        .gap(12.0)
        .child(label("Dashboard", 18.0))
        .child(search);
    let navigation = col()
        .w(SIDEBAR_WIDTH)
        .h_full()
        .children((0..SECTION_COUNT).map(|section| {
            row()
                .key(format!("section-{section}"))
                .w_full()
                .h(24.0)
                .fill(if dashboard.section == section {
                    color((220, 230, 245))
                } else {
                    Color::WHITE
                })
                .on_press_tx(move |state: &mut MemoryState, transaction| {
                    state.content.dashboard.navigate(section);
                    transaction.instant();
                    transaction.scroll_to_y("records", 0.0);
                })
                .child(label(format!("Section {section}"), 14.0))
        }))
        .child(
            col().children(state.content.thumbnails.chunks(2).map(|images| {
                row().children(
                    images
                        .iter()
                        .map(|image| el().size(48.0, 48.0).pixels(image.pixels())),
                )
            })),
        );
    let records = col()
        .key("records")
        .w(RECORDS_WIDTH)
        .h_full()
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(
            col()
                .w_full()
                .children(dashboard.visible_records().into_iter().map(|index| {
                    let record = &dashboard.fixtures.records[index];
                    let on = dashboard.active.get(index);
                    row()
                        .key(format!("record-{index}"))
                        .w_full()
                        .h(RECORD_HEIGHT)
                        .fill(if dashboard.selected_record == index {
                            color((220, 230, 245))
                        } else {
                            Color::WHITE
                        })
                        .child(
                            col()
                                .grow(1.0)
                                .min_w(0.0)
                                .h_full()
                                .clip_rounded(0.0)
                                .on_press_tx(move |state: &mut MemoryState, transaction| {
                                    state.content.dashboard.select(index);
                                    transaction.instant();
                                })
                                .child(label(record.title.clone(), 14.0))
                                .child(label(record.body.clone(), 12.0).w_full().wrap()),
                        )
                        .child(
                            row()
                                .key(format!("toggle-{index}"))
                                .size(48.0, 28.0)
                                .center()
                                .fill(color(if on { (47, 158, 68) } else { (134, 142, 150) }))
                                .role("switch")
                                .label(format!("Toggle record {index}"))
                                .checked(on)
                                .focusable(true)
                                .on_press_tx(move |state: &mut MemoryState, transaction| {
                                    state.content.dashboard.toggle(index);
                                    transaction.instant();
                                })
                                .child(
                                    label(if on { "ON" } else { "OFF" }, 12.0).color(Color::WHITE),
                                ),
                        )
                })),
        );
    let fields = col()
        .key("fields")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(
            col()
                .w_full()
                .children(
                    state
                        .content
                        .fields
                        .iter()
                        .enumerate()
                        .map(|(index, editor)| {
                            row()
                                .w_full()
                                .h(40.0)
                                .center_y()
                                .child(label(format!("Field {index:02}"), 14.0).w(100.0))
                                .child(
                                    text_input(editor, input_style())
                                        .key(format!("field-{index}"))
                                        .grow(1.0)
                                        .h(32.0)
                                        .single_line()
                                        .color(Color::BLACK)
                                        .fill(color((245, 245, 245)))
                                        .padding(6.0)
                                        .on_edit(move |state: &mut MemoryState| {
                                            &mut state.content.fields[index]
                                        })
                                        .on_input_change(
                                            move |state: &mut MemoryState, transaction| {
                                                state.content.dashboard.edit(
                                                    index,
                                                    state.content.fields[index].text.clone(),
                                                );
                                                transaction.instant();
                                            },
                                        ),
                                )
                        }),
                ),
        );
    let detail = col()
        .key("inspector")
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .child(
            col()
                .key("detail")
                .w_full()
                .h(300.0)
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(label(dashboard.detail.to_string(), 14.0).w_full().wrap()),
        )
        .child(fields);
    col().w_full().h_full().child(header).child(
        row()
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .child(navigation)
            .child(records)
            .child(detail),
    )
}
