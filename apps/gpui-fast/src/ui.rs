use benchmark_common::{FrameState, HEIGHT, Harness, Scenario, SwitchMode, WIDTH};
#[cfg(any(feature = "startup", feature = "scroll", feature = "virtual-scroll"))]
use benchmark_common::{ROW_HEIGHT, row_body, row_title};
#[cfg(any(
    feature = "startup",
    feature = "memory",
    feature = "scroll",
    feature = "virtual-scroll"
))]
use gpui::div;
#[cfg(any(feature = "startup", feature = "scroll", feature = "virtual-scroll"))]
use gpui::rgb;
use gpui::{
    App, Bounds, Context, Entity, Render, WindowBounds, WindowOptions, prelude::*, px, size,
};
use std::{borrow::Cow, cell::RefCell, io, rc::Rc, time::Duration};

pub(crate) trait ScenarioView: Render + Sized {
    const SCENARIO: Scenario;

    fn new() -> Self;
    fn rendered(&self) -> bool;
    fn reset(&mut self, cx: &mut Context<Self>);
    fn update(&mut self, state: FrameState, mode: SwitchMode, cx: &mut Context<Self>) -> u64;
    fn report(&self, _harness: &mut Harness) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(any(feature = "startup", feature = "memory"))]
pub(crate) fn optional_input(
    input: Option<Entity<crate::text_input::TextInput>>,
) -> gpui::AnyElement {
    match input {
        Some(input) => input.into_any_element(),
        None => div().into_any_element(),
    }
}

#[cfg(feature = "switches")]
pub(crate) fn color(value: (u8, u8, u8)) -> u32 {
    (u32::from(value.0) << 16) | (u32::from(value.1) << 8) | u32::from(value.2)
}

#[cfg(any(feature = "startup", feature = "scroll", feature = "virtual-scroll"))]
pub(crate) fn row(index: usize) -> gpui::Div {
    div()
        .h(px(ROW_HEIGHT))
        .flex_shrink_0()
        .w_full()
        .flex()
        .flex_col()
        .justify_center()
        .border_b_1()
        .border_color(rgb(0xdddddd))
        .px_3()
        .child(div().text_sm().truncate().child(row_title(index)))
        .child(
            div()
                .text_xs()
                .truncate()
                .text_color(rgb(0x666666))
                .child(row_body(index)),
        )
}

pub(crate) fn run<V: ScenarioView>() -> Result<(), Box<dyn std::error::Error>> {
    let harness = Rc::new(RefCell::new(Harness::connect(V::SCENARIO)?));
    let error = Rc::new(RefCell::new(None::<String>));
    let app_harness = harness.clone();
    let app_error = error.clone();
    gpui_platform::application().run(move |cx: &mut App| {
        #[cfg(any(feature = "startup", feature = "memory"))]
        crate::text_input::bind_keys(cx);
        if let Err(failure) = cx
            .text_system()
            .add_fonts(vec![Cow::Borrowed(include_bytes!(
                "../../../assets/fonts/DejaVuSans.ttf"
            ))])
        {
            *app_error.borrow_mut() = Some(format!("Could not load benchmark font: {failure}"));
            cx.quit();
            return;
        }
        let bounds = Bounds::centered(None, size(px(WIDTH), px(HEIGHT)), cx);
        let entity: Entity<V> = cx.new(|_| V::new());
        let task_error = app_error.clone();
        let task_harness = app_harness.clone();
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |window, cx| {
                let entity = entity.clone();
                let task_entity = entity.clone();
                let error = task_error.clone();
                let harness = task_harness.clone();
                window
                    .spawn(cx, async move |cx| {
                        let mut reset_generation = 0;
                        loop {
                            gpui_platform::background_executor()
                                .timer(Duration::from_millis(16))
                                .await;
                            let done = match cx.update(|_, cx| {
                                task_entity.update(cx, |view, cx| {
                                    let state = match harness.borrow_mut().poll() {
                                        Ok(state) => state,
                                        Err(failure) => {
                                            *error.borrow_mut() = Some(format!(
                                                "GPUI Fast benchmark protocol failure: {failure}"
                                            ));
                                            cx.quit();
                                            return true;
                                        }
                                    };
                                    if state.reset_generation != reset_generation {
                                        view.reset(cx);
                                        reset_generation = state.reset_generation;
                                    }
                                    if view.rendered() {
                                        if let Err(failure) = harness
                                            .borrow_mut()
                                            .mark_ready("initial-render-returned-not-presented")
                                        {
                                            *error.borrow_mut() = Some(failure.to_string());
                                            cx.quit();
                                            return true;
                                        }
                                    }
                                    if state.should_quit {
                                        cx.quit();
                                        return true;
                                    }
                                    let mode = harness.borrow().switch_mode();
                                    let applied = view.update(state, mode, cx);
                                    if let Err(failure) = view.report(&mut harness.borrow_mut()) {
                                        *error.borrow_mut() = Some(format!(
                                            "GPUI Fast dashboard report failure: {failure}"
                                        ));
                                        cx.quit();
                                        return true;
                                    }
                                    if let Err(failure) =
                                        harness.borrow_mut().report_applied(applied)
                                    {
                                        *error.borrow_mut() = Some(format!(
                                            "GPUI Fast benchmark protocol failure: {failure}"
                                        ));
                                        cx.quit();
                                        return true;
                                    }
                                    false
                                })
                            }) {
                                Ok(done) => done,
                                Err(failure) => {
                                    *error.borrow_mut() = Some(format!(
                                        "GPUI Fast benchmark event loop failure: {failure}"
                                    ));
                                    break;
                                }
                            };
                            if done {
                                break;
                            }
                        }
                    })
                    .detach();
                entity
            },
        );
        if let Err(failure) = opened {
            *app_error.borrow_mut() = Some(format!(
                "Could not open GPUI Fast benchmark window: {failure}"
            ));
            cx.quit();
            return;
        }
        cx.activate(true);
    });
    if let Some(failure) = error.borrow_mut().take() {
        return Err(failure.into());
    }
    Ok(())
}
