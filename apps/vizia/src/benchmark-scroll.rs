use benchmark_common::{FrameState, HEIGHT, Harness, ROW_HEIGHT, Scenario};
use vizia::prelude::*;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run::<Scroll>()
}
struct Scroll;
impl crate::ui::Adapter for Scroll {
    type State = Signal<f32>;
    const SCENARIO: Scenario = Scenario::Scroll;
    fn state() -> Self::State {
        Signal::new(0.0)
    }
    fn build(cx: &mut Context, position: &Self::State) {
        ScrollView::new(cx, |cx| crate::ui::build_rows(cx, 2_000)).scroll_y(position.clone());
    }
    fn update(
        position: &Self::State,
        frame: FrameState,
        _: &mut Harness,
        reset: bool,
    ) -> std::io::Result<u64> {
        if reset {
            position.set(0.0);
        }
        if frame.running {
            let maximum = (2_000.0 * ROW_HEIGHT - HEIGHT).max(0.0);
            position.set(if maximum > 0.0 {
                frame.scroll_offset / maximum
            } else {
                0.0
            });
        }
        Ok(frame.update_index)
    }
}
