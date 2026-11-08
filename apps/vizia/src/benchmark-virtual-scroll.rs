use benchmark_common::{FrameState, HEIGHT, Harness, Scenario, WIDTH};
use vizia::prelude::*;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run::<VirtualScroll>()
}
struct VirtualScroll;
impl crate::ui::Adapter for VirtualScroll {
    type State = Signal<f32>;
    const SCENARIO: Scenario = Scenario::VirtualScroll;
    fn state() -> Self::State {
        Signal::new(0.0)
    }
    fn build(cx: &mut Context, position: &Self::State) {
        let records = Signal::new((0..100_000).collect::<Vec<usize>>());
        VirtualList::new(cx, records, 48.0, |cx, _, record_index| {
            let index = record_index.get();
            HStack::new(cx, |cx| {
                VStack::new(cx, |cx| {
                    Label::new(cx, benchmark_common::row_title(index));
                    Label::new(cx, benchmark_common::row_body(index));
                });
            })
        })
        .scroll_y(position.clone())
        .height(Pixels(HEIGHT))
        .width(Pixels(WIDTH));
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
            let maximum = (100_000.0 * 48.0 - HEIGHT).max(0.0);
            position.set(if maximum > 0.0 {
                frame.scroll_offset / maximum
            } else {
                0.0
            });
        }
        Ok(frame.update_index)
    }
}
