use benchmark_common::{
    FrameState, Harness, SWITCH_CELL_SIZE, SWITCH_COLUMNS, SWITCH_FONT_SIZE, SWITCH_OFF_COLOR,
    SWITCH_ON_COLOR, SWITCH_ROWS, SWITCH_TEXT_COLOR, Scenario, Switches,
};
use std::{cell::RefCell, rc::Rc};
use vizia::prelude::*;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run::<SwitchScenario>()
}
struct SwitchScenario;
#[derive(Clone)]
struct State {
    model: Signal<Switches>,
    values: Rc<RefCell<Vec<(WriteSignal<bool>, WriteSignal<String>)>>>,
}
impl crate::ui::Adapter for SwitchScenario {
    type State = State;
    const SCENARIO: Scenario = Scenario::Switches;
    fn state() -> State {
        State {
            model: Signal::new(Switches::default()),
            values: Rc::new(RefCell::new(Vec::new())),
        }
    }
    fn build(cx: &mut Context, state: &State) {
        for _ in 0..SWITCH_ROWS {
            let values = state.values.clone();
            let model = state.model.clone();
            HStack::new(cx, move |cx| {
                for _ in 0..SWITCH_COLUMNS {
                    let value = Signal::new(false);
                    let text = Signal::new(String::from("OFF"));
                    let index = values.borrow().len();
                    values
                        .borrow_mut()
                        .push((value.write_only(), text.write_only()));
                    DummyToggle::new(cx, value, text, index, model.clone());
                }
            })
            .height(Pixels(SWITCH_CELL_SIZE))
            .width(Pixels(SWITCH_CELL_SIZE * SWITCH_COLUMNS as f32));
        }
    }
    fn update(
        state: &State,
        frame: FrameState,
        harness: &mut Harness,
        reset: bool,
    ) -> std::io::Result<u64> {
        if reset {
            let current = state.model.get();
            let values = state.values.borrow();
            for (index, (signal, text)) in values.iter().enumerate() {
                if current.get(index) {
                    signal.set(false);
                    text.set("OFF".to_owned());
                }
            }
            state.model.set(Switches::default());
        }
        if frame.running {
            let mut model = state.model.get();
            let values = state.values.borrow();
            model.advance_changed(frame.update_index, harness.switch_mode(), |index, value| {
                if let Some((signal, text)) = values.get(index) {
                    signal.set(value);
                    text.set(if value { "ON" } else { "OFF" }.to_owned());
                }
            });
            state.model.set(model);
        }
        Ok(state.model.get().applied_updates())
    }
}
struct DummyToggle;
impl DummyToggle {
    fn new(
        cx: &mut Context,
        value: Signal<bool>,
        text: Signal<String>,
        index: usize,
        model: Signal<Switches>,
    ) {
        let background = value.clone();
        let click_value = value.clone();
        let click_text = text.clone();
        Self.build(cx, |cx| {
            Label::new(cx, text)
                .font_family(vec![FamilyOwned::Named("DejaVu Sans".to_owned())])
                .font_size(SWITCH_FONT_SIZE)
                .text_align(TextAlign::Center)
                .color(Color::rgb(
                    SWITCH_TEXT_COLOR.0,
                    SWITCH_TEXT_COLOR.1,
                    SWITCH_TEXT_COLOR.2,
                ));
        })
        .width(Pixels(SWITCH_CELL_SIZE))
        .height(Pixels(SWITCH_CELL_SIZE))
        .alignment(Alignment::Center)
        .bind(background.clone(), move |handle| {
            handle.background_color(if background.get() {
                Color::rgb(SWITCH_ON_COLOR.0, SWITCH_ON_COLOR.1, SWITCH_ON_COLOR.2)
            } else {
                Color::rgb(SWITCH_OFF_COLOR.0, SWITCH_OFF_COLOR.1, SWITCH_OFF_COLOR.2)
            });
        })
        .on_press(move |_| {
            click_value.update(|current| *current = !*current);
            model.update(|model| model.toggle(index));
            click_text
                .update(|text| *text = if click_value.get() { "ON" } else { "OFF" }.to_owned());
        });
    }
}
impl View for DummyToggle {}
