slint::slint! {
    import { Palette, Button, LineEdit, ScrollView } from "std-widgets.slint";
    import { RecordValue, RowView } from "common.slint";
    export component BenchWindow inherits Window {
        width: 1280px; height: 800px; title: "Slint benchmark";
        background: white; default-font-family: "DejaVu Sans"; default-font-size: 14px;
        init => { Palette.color-scheme = ColorScheme.light; }
        in property <[RecordValue]> rows;
        in-out property <string> query;
        Rectangle {
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
    }
}

use benchmark_common::Scenario;
use slint::{ModelRc, VecModel};
use std::rc::Rc;

fn record(index: usize) -> RecordValue {
    RecordValue {
        record_id: index as i32,
        title: benchmark_common::row_title(index).into(),
        body: benchmark_common::row_body(index).into(),
        active: false,
        selected: false,
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let window = BenchWindow::new()?;
    let rows = Rc::new(VecModel::from((0..20).map(record).collect::<Vec<_>>()));
    window.set_rows(ModelRc::from(rows));
    crate::ui::run(
        Scenario::Startup,
        window,
        move |_window, frame, _harness, _reset| Ok(frame.update_index),
    )
}
