slint::slint! {
    import { Palette, LineEdit, ScrollView } from "std-widgets.slint";
    import { RecordValue } from "common.slint";
    export component BenchWindow inherits Window {
        width: 1280px; height: 800px; title: "Slint benchmark";
        background: white; default-font-family: "DejaVu Sans"; default-font-size: 14px;
        init => { Palette.color-scheme = ColorScheme.light; }
        in property <[RecordValue]> rows;
        in property <[image]> thumbnails;
        in property <[string]> fields;
        in-out property <string> query;
        in property <string> detail;
        in property <string> selected-title;
        in-out property <length> scroll-y;
        callback record-selected(int);
        callback record-toggled(int);
        callback section-selected(int);
        callback query-edited(string);
        callback field-edited(int, string);
        Rectangle {
            width: parent.width; height: parent.height;
            Rectangle { x: 0px; y: 0px; width: parent.width; height: 56px;
                Text { x: 8px; y: 0px; width: 88px; height: 56px; text: "Dashboard"; vertical-alignment: center; }
                LineEdit { x: 104px; y: 10px; width: parent.width - 112px; height: 36px; placeholder-text: "Search records"; text <=> root.query; edited(value) => { root.query-edited(value); } }
            }
            Rectangle { y: 56px; width: parent.width; height: parent.height - 56px;
                Rectangle { x: 0px; y: 0px; width: 180px; height: parent.height; background: #f1f3f5;
                    for section[index] in 20: Rectangle { x: 8px; y: index * 22px; width: 164px; height: 22px;
                        Text { x: 0px; y: 0px; width: parent.width; height: parent.height; text: index == 0 ? "All records" : "Section " + index; font-size: 12px; vertical-alignment: center; }
                        TouchArea { clicked => { root.section-selected(index); } }
                    }
                    for thumbnail[index] in root.thumbnails: Image { x: 8px + mod(index, 2) * 64px; y: 450px + floor(index / 2) * 52px; width: 48px; height: 48px; source: thumbnail; image-fit: contain; }
                }
                ScrollView { x: 180px; y: 0px; width: 600px; height: parent.height; content-y <=> root.scroll-y;
                    content-height: root.rows.length * 64px;
                    for row[index] in root.rows: Rectangle { y: index * 64px; width: parent.width; height: 64px; background: row.selected ? #e3edff : white;
                        Rectangle { x: 8px; width: parent.width - 72px; height: 64px;
                            Text { y: 4px; height: 20px; width: parent.width; text: row.title; font-size: 14px; }
                            Text { y: 24px; height: 32px; width: parent.width; text: row.body; font-size: 12px; wrap: word-wrap; overflow: elide; }
                            TouchArea { clicked => { root.record-selected(row.record-id); } }
                        }
                        Rectangle { x: parent.width - 56px; y: 20px; width: 48px; height: 24px; background: #868e96;
                            Text { text: row.active ? "ON" : "OFF"; color: white; horizontal-alignment: center; vertical-alignment: center; }
                            TouchArea { clicked => { root.record-toggled(row.record-id); } }
                        }
                    }
                }
                Rectangle { x: 780px; y: 0px; width: parent.width - 780px; height: parent.height;
                    Text { x: 8px; y: 8px; width: parent.width - 16px; height: 24px; text: root.selected-title; font-size: 18px; }
                    ScrollView { x: 8px; y: 40px; width: parent.width - 16px; height: 260px;
                        Text { x: 0px; y: 0px; width: parent.width; text: root.detail; wrap: word-wrap; }
                    }
                    ScrollView { x: 8px; y: 308px; width: parent.width - 16px; height: parent.height - 308px;
                        content-height: 20 * 36px;
                        for value[index] in root.fields: Rectangle { y: index * 36px; width: parent.width; height: 36px;
                            Text { x: 0px; y: 0px; width: 80px; height: 36px; text: "Field " + index; vertical-alignment: center; }
                            LineEdit { x: 88px; y: 2px; width: parent.width - 88px; height: 32px; text: value; edited(text) => { root.field-edited(index, text); } }
                        }
                    }
                }
            }
        }
    }
}

use benchmark_common::{
    Scenario,
    dashboard::{Dashboard, THUMBNAILS},
};
use slint::{Model, VecModel};
use std::{cell::RefCell, rc::Rc};

fn sync_dashboard(
    window: &BenchWindow,
    state: &Dashboard,
    records: &VecModel<RecordValue>,
    fields: &VecModel<slint::SharedString>,
) {
    let values: Vec<_> = state
        .visible_records()
        .into_iter()
        .map(|index| RecordValue {
            record_id: index as i32,
            title: state.fixtures.records[index].title.as_str().into(),
            body: state.fixtures.records[index].body.as_str().into(),
            active: state.active.get(index),
            selected: state.selected_record == index,
        })
        .collect();
    if records.row_count() != values.len()
        || values.iter().enumerate().any(|(index, value)| {
            records
                .row_data(index)
                .is_none_or(|old| old.record_id != value.record_id)
        })
    {
        records.set_vec(values);
    } else {
        for (index, value) in values.into_iter().enumerate() {
            if records.row_data(index).as_ref() != Some(&value) {
                records.set_row_data(index, value);
            }
        }
    }
    for (index, value) in state.fields.iter().enumerate() {
        if fields
            .row_data(index)
            .is_none_or(|old| old.as_str() != value)
        {
            fields.set_row_data(index, value.as_str().into());
        }
    }
    if window.get_query().as_str() != state.query {
        window.set_query(state.query.as_str().into());
    }
    if window.get_detail().as_str() != state.detail.as_ref() {
        window.set_detail(state.detail.as_ref().into());
    }
    let title = &state.fixtures.records[state.selected_record].title;
    if window.get_selected_title().as_str() != title {
        window.set_selected_title(title.as_str().into());
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let window = BenchWindow::new()?;
    let rows = Rc::new(VecModel::from(Vec::<RecordValue>::new()));
    window.set_rows(rows.clone().into());
    let fields = Rc::new(VecModel::from(Vec::<slint::SharedString>::new()));
    window.set_fields(fields.clone().into());
    let dashboard = Rc::new(RefCell::new(Dashboard::default()));
    fields.set_vec(
        dashboard
            .borrow()
            .fields
            .iter()
            .map(|value| value.as_str().into())
            .collect::<Vec<slint::SharedString>>(),
    );
    let images = THUMBNAILS
        .iter()
        .map(|bytes| slint::Image::load_from_data(bytes, Some("png")))
        .collect::<Result<Vec<_>, _>>()?;
    window.set_thumbnails(Rc::new(VecModel::from(images)).into());
    let selected = dashboard.clone();
    window.on_record_selected(move |index| selected.borrow_mut().select(index as usize));
    let toggled = dashboard.clone();
    window.on_record_toggled(move |index| toggled.borrow_mut().toggle(index as usize));
    let navigation = dashboard.clone();
    window.on_section_selected(move |index| navigation.borrow_mut().navigate(index as usize));
    let query = dashboard.clone();
    window.on_query_edited(move |text| query.borrow_mut().search(text.to_string()));
    let edited = dashboard.clone();
    window.on_field_edited(move |index, text| {
        edited.borrow_mut().edit(index as usize, text.to_string())
    });
    sync_dashboard(&window, &dashboard.borrow(), &rows, &fields);
    crate::ui::run(
        Scenario::Memory,
        window,
        move |window, frame, harness, reset| {
            if reset {
                dashboard.borrow_mut().reset();
                window.set_scroll_y(0.0);
            }
            let mut state = dashboard.borrow_mut();
            if frame.running {
                state.advance(frame);
            }
            sync_dashboard(window, &state, &rows, &fields);
            harness.report_dashboard(&state)?;
            let applied = state.applied_updates();
            if frame.running {
                window.set_scroll_y(-frame.scroll_offset);
            }
            Ok(applied)
        },
    )
}
