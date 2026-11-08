use benchmark_common::{FrameState, Scenario};
use rune::{El, ScrollSpec, TextEdit, TextStyle, Tx, col, memo, row, text_input};

use crate::ui::{Benchmark, Error, ScenarioUi, label, record};

pub struct Startup {
    search: TextEdit,
}

impl ScenarioUi for Startup {
    const SCENARIO: Scenario = Scenario::Startup;

    fn new() -> Result<Self, Error> {
        Ok(Self {
            search: TextEdit::new(""),
        })
    }

    fn reset(&mut self, transaction: &mut Tx) {
        transaction.scroll_to_y("records", 0.0);
    }

    fn update(
        &mut self,
        _: FrameState,
        _: &benchmark_common::Harness,
        _: &mut Tx,
        _: bool,
    ) -> bool {
        false
    }

    fn view(state: &Benchmark<Self>) -> El {
        col()
            .w_full()
            .h_full()
            .child(
                row()
                    .w_full()
                    .h(56.0)
                    .center_y()
                    .gap(12.0)
                    .child(label("Dashboard", 18.0))
                    .child(
                        text_input(&state.content.search, TextStyle::new(14.0, 400.0))
                            .key("search")
                            .grow(1.0)
                            .h(36.0)
                            .single_line()
                            .on_edit(|state: &mut Benchmark<Self>| &mut state.content.search),
                    ),
            )
            .child(
                col().w_full().grow(1.0).min_h(0.0).child(
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
                        })),
                ),
            )
    }
}

pub fn run() -> Result<(), Error> {
    crate::ui::run::<Startup>()
}
