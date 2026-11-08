slint::slint! {
    import { Palette, ListView } from "std-widgets.slint";
    import { RecordValue, RowView } from "common.slint";
    export component BenchWindow inherits Window {
        width: 1280px; height: 800px; title: "Slint benchmark";
        background: white; default-font-family: "DejaVu Sans"; default-font-size: 14px;
        init => { Palette.color-scheme = ColorScheme.light; }
        in property <[RecordValue]> rows;
        in-out property <length> scroll-y;
        ListView {
            width: parent.width; height: parent.height;
            content-y <=> root.scroll-y;
            for row[index] in root.rows: RowView { width: parent.width; title: row.title; body: row.body; }
        }
    }
}

use benchmark_common::Scenario;
use slint::{Model, ModelRc};

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
        title: benchmark_common::row_title(index).into(),
        body: benchmark_common::row_body(index).into(),
        active: false,
        selected: false,
    }
}
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let window = BenchWindow::new()?;
    window.set_rows(ModelRc::new(GeneratedRows));
    crate::ui::run(
        Scenario::VirtualScroll,
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
