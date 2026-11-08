use benchmark_common::{FrameState, ROW_HEIGHT, Scenario};
use rune::{El, GrowthSpec, ScrollSpec, Tx, col};

use crate::ui::{Benchmark, Error, ScenarioUi, record};

pub struct VirtualScroll {
    visible_rows: usize,
}

impl ScenarioUi for VirtualScroll {
    const SCENARIO: Scenario = Scenario::VirtualScroll;

    fn new() -> Result<Self, Error> {
        Ok(Self { visible_rows: 0 })
    }

    fn reset(&mut self, transaction: &mut Tx) {
        transaction.scroll_to_y("records", 0.0);
    }

    fn update(
        &mut self,
        frame: FrameState,
        _: &benchmark_common::Harness,
        transaction: &mut Tx,
        _: bool,
    ) -> bool {
        transaction.scroll_to_y("records", frame.scroll_offset);
        false
    }

    fn view(_: &Benchmark<Self>) -> El {
        col()
            .key("records")
            .w_full()
            .h_full()
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .child(
                col()
                    .key("rows")
                    .w_full()
                    .growth(GrowthSpec::default())
                    .virtual_rows(
                        Self::SCENARIO.row_count(),
                        ROW_HEIGHT,
                        |_: &Benchmark<Self>, index| format!("record-{index}").into(),
                        |_: &Benchmark<Self>, index| record(index),
                    )
                    .virtual_rows_version(0_u64)
                    .on_visible_rows(|state: &mut Benchmark<Self>, range| {
                        state.content.visible_rows = range.len();
                        false
                    }),
            )
    }
}

pub fn run() -> Result<(), Error> {
    crate::ui::run::<VirtualScroll>()
}
