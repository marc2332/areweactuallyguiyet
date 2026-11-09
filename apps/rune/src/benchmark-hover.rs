use benchmark_common::{FrameState, Harness, Scenario, hover};
use rune::{Anim, El, Tx, col, row};

use crate::ui::{Benchmark, Error, ScenarioUi, color};

pub struct HoverGrid;

impl ScenarioUi for HoverGrid {
    const SCENARIO: Scenario = Scenario::Hover;

    fn new() -> Result<Self, Error> {
        Ok(Self)
    }

    fn reset(&mut self, _: &mut Tx) {}

    fn update(&mut self, _: FrameState, _: &Harness, _: &mut Tx, _: bool) -> bool {
        false
    }

    fn view(_: &Benchmark<Self>) -> El {
        col()
            .size(hover::GRID_SIZE, hover::GRID_SIZE)
            .children((0..hover::ROWS).map(|row_index| {
                row()
                    .h(hover::CELL_SIZE)
                    .children((0..hover::COLUMNS).map(move |column| {
                        let index = row_index * hover::COLUMNS + column;
                        col()
                            .key(format!("hover-cell-{index}"))
                            .size(hover::CELL_SIZE, hover::CELL_SIZE)
                            .fill(color(hover::NORMAL_COLOR))
                            .hover(color(hover::HOVERED_COLOR))
                            .hover_motion(Anim::INSTANT, Anim::INSTANT)
                            .on_hover_change(
                                move |state: &mut Benchmark<Self>, inside, transaction| {
                                    if inside {
                                        state.hover_observer.enter(index);
                                    } else {
                                        state.hover_observer.leave(index);
                                    }
                                    transaction.live_only();
                                },
                            )
                    }))
            }))
    }
}

pub fn run() -> Result<(), Error> {
    crate::ui::run::<HoverGrid>()
}
