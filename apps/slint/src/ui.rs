use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use benchmark_common::{
    Harness, Scenario, Switches,
    dashboard::{Dashboard, THUMBNAILS},
    row_body, row_title,
};
use slint::{ComponentHandle, Model, ModelRc, VecModel};

slint::slint! {
    import { Button, LineEdit, ScrollView, ListView, Palette } from "std-widgets.slint";
    import "../../../assets/fonts/DejaVuSans.ttf";
    export struct RecordValue { record-id: int, title: string, body: string, active: bool, selected: bool }
    component RowView inherits Rectangle {
        in property <string> title;
        in property <string> body;
        height: 48px;
        Text { x: 8px; y: 4px; width: parent.width - 16px; height: 20px; text: root.title; font-size: 14px; }
        Text { x: 8px; y: 24px; width: parent.width - 16px; height: 18px; text: root.body; font-size: 12px; overflow: elide; }
    }
    component DummyCell inherits Rectangle {
        in property <bool> active;
        callback clicked();
        width: 30px; height: 30px;
        background: active ? #2f9e44 : #868e96;
        Text { x: 0px; y: 0px; width: parent.width; height: parent.height; text: root.active ? "ON" : "OFF"; color: white; font-size: 12px; font-family: "DejaVu Sans"; horizontal-alignment: center; vertical-alignment: center; }
        TouchArea { clicked => { root.clicked(); } }
    }
    export component BenchWindow inherits Window {
        width: 1280px; height: 800px; title: "Slint benchmark";
        background: white; default-font-family: "DejaVu Sans"; default-font-size: 14px;
        init => { Palette.color-scheme = ColorScheme.light; }
        in property <int> mode;
        in property <[RecordValue]> rows;
        in property <[bool]> cells;
        in property <[image]> thumbnails;
        in property <[string]> fields;
        in-out property <string> query;
        in property <string> detail;
        in property <string> selected-title;
        in-out property <length> scroll-y;
        callback cell-clicked(int);
        callback record-selected(int);
        callback record-toggled(int);
        callback section-selected(int);
        callback query-edited(string);
        callback field-edited(int, string);
        if mode == 0: Rectangle {
            width: parent.width; height: parent.height;
            Text { x: 16px; y: 16px; text: "Benchmark startup"; font-size: 20px; }
            Text { x: 16px; y: 48px; text: "Prepared initial content"; }
            LineEdit { x: 16px; y: 76px; width: 430px; height: 32px; text <=> root.query; }
            Button { x: 16px; y: 116px; width: 120px; height: 32px; text: "Apply"; }
            ScrollView { x: 16px; y: 156px; width: parent.width - 32px; height: parent.height - 172px;
                content-height: 20 * 48px;
                for row[index] in root.rows: RowView { y: index * 48px; width: parent.width; title: row.title; body: row.body; }
            }
        }
        if mode == 1: ScrollView {
            width: parent.width; height: parent.height;
            content-y <=> root.scroll-y;
            content-height: 2000 * 48px;
            for row[index] in root.rows: RowView { y: index * 48px; width: parent.width; title: row.title; body: row.body; }
        }
        if mode == 2: ListView {
            width: parent.width; height: parent.height;
            content-y <=> root.scroll-y;
            for row[index] in root.rows: RowView { width: parent.width; title: row.title; body: row.body; }
        }
        if mode == 3: Rectangle {
            width: parent.width; height: parent.height;
            for value[index] in root.cells: DummyCell {
                x: mod(index, 30) * 30px; y: floor(index / 30) * 30px;
                active: value; clicked => { root.cell-clicked(index); }
            }
        }
        if mode == 4: Rectangle {
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

struct GeneratedRows;
impl Model for GeneratedRows {
    type Data = RecordValue;
    fn row_count(&self) -> usize {
        100_000
    }
    fn row_data(&self, row: usize) -> Option<Self::Data> {
        (row < 100_000).then(|| record(row))
    }
    fn model_tracker(&self) -> &dyn slint::ModelTracker {
        &()
    }
}

fn record(index: usize) -> RecordValue {
    RecordValue {
        record_id: index as i32,
        title: row_title(index).into(),
        body: row_body(index).into(),
        active: false,
        selected: false,
    }
}

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

pub fn run(scenario: Scenario) -> Result<(), Box<dyn std::error::Error>> {
    let window = BenchWindow::new()?;
    window.set_mode(match scenario {
        Scenario::Startup => 0,
        Scenario::Scroll => 1,
        Scenario::VirtualScroll => 2,
        Scenario::Switches => 3,
        Scenario::Memory => 4,
    });
    let harness = Rc::new(RefCell::new(Harness::connect(scenario)?));
    let failure = Rc::new(RefCell::new(None::<String>));
    let ready = Rc::new(Cell::new(false));
    let ready_harness = Rc::clone(&harness);
    let ready_failure = Rc::clone(&failure);
    window.window().set_rendering_notifier(move |state, _| {
        if matches!(state, slint::RenderingState::AfterRendering) && !ready.replace(true) {
            if let Err(error) = ready_harness
                .borrow_mut()
                .mark_ready("initial-render-returned-not-presented")
            {
                *ready_failure.borrow_mut() = Some(error.to_string());
                let _ = slint::quit_event_loop();
            }
        }
    })?;
    let rows = Rc::new(VecModel::from(if scenario == Scenario::Startup {
        (0..20).map(record).collect()
    } else if scenario == Scenario::Scroll {
        (0..2000).map(record).collect()
    } else {
        Vec::new()
    }));
    window.set_rows(if scenario == Scenario::VirtualScroll {
        ModelRc::new(GeneratedRows)
    } else {
        rows.clone().into()
    });
    let cells = Rc::new(VecModel::from(if scenario == Scenario::Switches {
        vec![false; 900]
    } else {
        Vec::new()
    }));
    window.set_cells(cells.clone().into());
    let switches = Rc::new(RefCell::new(Switches::default()));
    let click_cells = cells.clone();
    let click_switches = switches.clone();
    window.on_cell_clicked(move |index| {
        let index = index as usize;
        let mut state = click_switches.borrow_mut();
        state.toggle(index);
        click_cells.set_row_data(index, state.get(index));
    });
    let dashboard =
        (scenario == Scenario::Memory).then(|| Rc::new(RefCell::new(Dashboard::default())));
    let fields = Rc::new(VecModel::from(Vec::<slint::SharedString>::new()));
    window.set_fields(fields.clone().into());
    if let Some(model) = dashboard.as_ref() {
        fields.set_vec(
            model
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
        let selected = model.clone();
        window.on_record_selected(move |index| selected.borrow_mut().select(index as usize));
        let toggled = model.clone();
        window.on_record_toggled(move |index| toggled.borrow_mut().toggle(index as usize));
        let navigation = model.clone();
        window.on_section_selected(move |index| navigation.borrow_mut().navigate(index as usize));
        let query = model.clone();
        window.on_query_edited(move |text| query.borrow_mut().search(text.to_string()));
        let edited = model.clone();
        window.on_field_edited(move |index, text| {
            edited.borrow_mut().edit(index as usize, text.to_string())
        });
        sync_dashboard(&window, &model.borrow(), &rows, &fields);
    }
    let timer = slint::Timer::default();
    let weak = window.as_weak();
    let timer_failure = failure.clone();
    let observed_reset = Cell::new(0);
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(16),
        move || {
            let Some(window) = weak.upgrade() else { return };
            let result = (|| -> std::io::Result<()> {
                let mut harness = harness.borrow_mut();
                let frame = harness.poll()?;
                if frame.should_quit {
                    slint::quit_event_loop().map_err(std::io::Error::other)?;
                    return Ok(());
                }
                if observed_reset.replace(frame.reset_generation) != frame.reset_generation {
                    *switches.borrow_mut() = Switches::default();
                    for index in 0..cells.row_count() {
                        if cells.row_data(index) == Some(true) {
                            cells.set_row_data(index, false);
                        }
                    }
                    if let Some(model) = dashboard.as_ref() {
                        model.borrow_mut().reset();
                    }
                    window.set_scroll_y(0.0);
                }
                let applied = if scenario == Scenario::Switches {
                    if frame.running {
                        switches.borrow_mut().advance_changed(
                            frame.update_index,
                            harness.switch_mode(),
                            |index, value| cells.set_row_data(index, value),
                        );
                    }
                    switches.borrow().applied_updates()
                } else if let Some(model) = dashboard.as_ref() {
                    let mut state = model.borrow_mut();
                    if frame.running {
                        state.advance(frame);
                    }
                    sync_dashboard(&window, &state, &rows, &fields);
                    harness.report_dashboard(&state)?;
                    state.applied_updates()
                } else {
                    frame.update_index
                };
                if frame.running
                    && matches!(
                        scenario,
                        Scenario::Scroll | Scenario::VirtualScroll | Scenario::Memory
                    )
                {
                    window.set_scroll_y(-frame.scroll_offset);
                }
                harness.report_applied(applied)?;
                Ok(())
            })();
            if let Err(error) = result {
                *timer_failure.borrow_mut() = Some(error.to_string());
                let _ = slint::quit_event_loop();
            }
        },
    );
    window.run()?;
    if let Some(error) = failure.borrow_mut().take() {
        return Err(error.into());
    }
    Ok(())
}
