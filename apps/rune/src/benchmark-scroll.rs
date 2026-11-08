use benchmark_common::{FrameState, Scenario};
use rune::{El, ScrollSpec, Tx, col, memo};

use crate::ui::{Benchmark, Error, ScenarioUi, record};

pub struct Scroll;

impl ScenarioUi for Scroll {
    const SCENARIO: Scenario = Scenario::Scroll;

    fn new() -> Result<Self, Error> {
        Ok(Self)
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
            .child(memo("retained-records", Self::SCENARIO.row_count(), || {
                col()
                    .w_full()
                    .children((0..Self::SCENARIO.row_count()).map(record))
            }))
    }
}

pub fn run() -> Result<(), Error> {
    crate::ui::run::<Scroll>()
}
