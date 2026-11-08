use benchmark_common::{
    FrameState, Harness, SWITCH_COLUMNS, SWITCH_COUNT, SWITCH_FONT_SIZE, SWITCH_OFF_COLOR,
    SWITCH_ON_COLOR, Scenario, Switches,
};
use rune::{Color, El, TextStyle, Tx, col, memo, row, text};

use crate::ui::{Benchmark, Error, ScenarioUi, color};

pub struct SwitchGrid {
    switches: Switches,
}

impl ScenarioUi for SwitchGrid {
    const SCENARIO: Scenario = Scenario::Switches;

    fn new() -> Result<Self, Error> {
        Ok(Self {
            switches: Switches::default(),
        })
    }

    fn reset(&mut self, transaction: &mut Tx) {
        self.switches = Switches::default();
        transaction.scroll_to_y("records", 0.0);
    }

    fn update(&mut self, frame: FrameState, harness: &Harness, _: &mut Tx, updated: bool) -> bool {
        if updated {
            self.switches
                .advance(frame.update_index, harness.switch_mode());
        }
        updated
    }

    fn applied_updates(&self, _: FrameState) -> u64 {
        self.switches.applied_updates()
    }

    fn view(state: &Benchmark<Self>) -> El {
        col()
            .key("dummy-grid")
            .size(900.0, 900.0)
            .children((0..SWITCH_COUNT / SWITCH_COLUMNS).map(|row_index| {
                row().h(30.0).children((0..SWITCH_COLUMNS).map(|column| {
                    let index = row_index * SWITCH_COLUMNS + column;
                    let on = state.content.switches.get(index);
                    memo(format!("dummy-cache-{index}"), on, || dummy(index, on))
                }))
            }))
    }
}

fn dummy(index: usize, on: bool) -> El {
    row()
        .key(format!("dummy-{index}"))
        .size(30.0, 30.0)
        .center()
        .fill(color(if on {
            SWITCH_ON_COLOR
        } else {
            SWITCH_OFF_COLOR
        }))
        .role("switch")
        .label(format!("Dummy {index}"))
        .checked(on)
        .focusable(true)
        .on_press_tx(move |state: &mut Benchmark<SwitchGrid>, transaction| {
            state.content.switches.toggle(index);
            transaction.instant();
        })
        .child(
            text(if on { "ON" } else { "OFF" })
                .font(TextStyle::new(SWITCH_FONT_SIZE, 400.0))
                .color(Color::WHITE),
        )
}

pub fn run() -> Result<(), Error> {
    crate::ui::run::<SwitchGrid>()
}
