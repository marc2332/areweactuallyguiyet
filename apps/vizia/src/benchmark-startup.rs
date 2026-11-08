use benchmark_common::{FrameState, Harness, Scenario};
use vizia::prelude::*;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run::<Startup>()
}
struct Startup;
impl crate::ui::Adapter for Startup {
    type State = Signal<usize>;
    const SCENARIO: Scenario = Scenario::Startup;
    fn state() -> Self::State {
        Signal::new(0)
    }
    fn build(cx: &mut Context, applied: &Self::State) {
        Label::new(cx, "Vizia benchmark startup");
        Label::new(cx, "A deterministic benchmark screen");
        let search = Signal::new(String::new());
        Textbox::new(cx, search)
            .on_edit(move |_, value| search.set(value))
            .width(Pixels(430.0))
            .height(Pixels(32.0));
        Button::new(cx, |cx| Label::new(cx, "Apply")).on_press({
            let applied = applied.clone();
            move |_| applied.update(|value| *value += 1)
        });
        ScrollView::new(cx, |cx| crate::ui::build_rows(cx, 20));
    }
    fn update(
        applied: &Self::State,
        frame: FrameState,
        _: &mut Harness,
        _: bool,
    ) -> std::io::Result<u64> {
        let _ = applied;
        Ok(frame.update_index)
    }
}
