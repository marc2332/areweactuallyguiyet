slint::slint! {
    import { Palette, ScrollView } from "std-widgets.slint";
    import { RecordValue, RowView } from "common.slint";
    export component BenchWindow inherits Window {
        width: 1280px; height: 800px; title: "Slint benchmark";
        background: white; default-font-family: "DejaVu Sans"; default-font-size: 14px;
        init => { Palette.color-scheme = ColorScheme.light; }
        in property <[RecordValue]> rows;
        in-out property <length> scroll-y;
        ScrollView {
            width: parent.width; height: parent.height;
            content-y <=> root.scroll-y;
            content-height: 2000 * 48px;
            for row[index] in root.rows: RowView { y: index * 48px; width: parent.width; title: row.title; body: row.body; }
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
    let rows = Rc::new(VecModel::from((0..2000).map(record).collect::<Vec<_>>()));
    window.set_rows(ModelRc::from(rows));
    crate::ui::run(
        Scenario::Scroll,
        window,
        move |window, frame, _harness, reset| {
            if reset {
                window.set_scroll_y(0.0);
            }
            if frame.running {
                window.set_scroll_y(-frame.scroll_offset);
            }
            Ok(frame.update_index)
        },
    )
}
