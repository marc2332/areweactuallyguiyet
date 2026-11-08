use benchmark_common::{FrameState, HEIGHT, Harness, Scenario, WIDTH};
#[cfg(any(feature = "startup", feature = "scroll", feature = "virtual-scroll"))]
use benchmark_common::{ROW_HEIGHT, row_body, row_title};
use gpui::{
    App, Application, Bounds, Context, Entity, Render, WindowBounds, WindowOptions, prelude::*, px,
    size,
};
#[cfg(any(feature = "startup", feature = "scroll", feature = "virtual-scroll"))]
use gpui::{div, rgb};
use std::{borrow::Cow, cell::RefCell, io, rc::Rc, time::Duration};

pub(crate) struct Protocol {
    pub harness: Rc<RefCell<Harness>>,
    pub reset_generation: u64,
    pub rendered: bool,
}

pub(crate) trait ScenarioView: Render + Sized + 'static {
    const SCENARIO: Scenario;
    const NOTIFY_EVERY_TICK: bool = true;
    fn new(cx: &mut App, harness: Rc<RefCell<Harness>>) -> Self;
    fn protocol(&mut self) -> &mut Protocol;
    fn reset(&mut self, cx: &mut Context<Self>);
    fn advance(&mut self, state: FrameState, cx: &mut Context<Self>);
    fn before_ready(&mut self, _state: FrameState, _cx: &mut Context<Self>) {}
    fn applied(&self, state: FrameState) -> u64 {
        state.update_index
    }
    fn report(&self) -> io::Result<()> {
        Ok(())
    }
    fn bind_keys(_cx: &mut App) {}
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
    Application::new().run(move |cx: &mut App| {
        V::bind_keys(cx);
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
        let entity: Entity<V> = cx.new(|cx| V::new(cx, app_harness.clone()));
        let task_error = app_error.clone();
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |window, cx| {
                let entity = entity.clone();
                let task_entity = entity.clone();
                let error = task_error.clone();
                window
                    .spawn(cx, async move |cx| {
                        loop {
                            gpui::Timer::after(Duration::from_millis(16)).await;
                            let done = match cx.update(|_, cx| {
                                task_entity.update(cx, |view, cx| {
                                    let harness = view.protocol().harness.clone();
                                    let state = match harness.borrow_mut().poll() {
                                        Ok(state) => state,
                                        Err(failure) => {
                                            *error.borrow_mut() = Some(format!(
                                                "GPUI benchmark protocol failure: {failure}"
                                            ));
                                            cx.quit();
                                            return true;
                                        }
                                    };
                                    if state.reset_generation != view.protocol().reset_generation {
                                        view.reset(cx);
                                        view.protocol().reset_generation = state.reset_generation;
                                    }
                                    view.before_ready(state, cx);
                                    if view.protocol().rendered {
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
                                    view.advance(state, cx);
                                    if let Err(failure) =
                                        harness.borrow_mut().report_applied(view.applied(state))
                                    {
                                        *error.borrow_mut() = Some(format!(
                                            "GPUI benchmark protocol failure: {failure}"
                                        ));
                                        cx.quit();
                                        return true;
                                    }
                                    if let Err(failure) = view.report() {
                                        *error.borrow_mut() = Some(format!(
                                            "GPUI dashboard reporting failure: {failure}"
                                        ));
                                        cx.quit();
                                        return true;
                                    }
                                    if V::NOTIFY_EVERY_TICK {
                                        cx.notify();
                                    }
                                    false
                                })
                            }) {
                                Ok(done) => done,
                                Err(failure) => {
                                    *error.borrow_mut() = Some(format!(
                                        "GPUI benchmark event loop failure: {failure}"
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
            *app_error.borrow_mut() =
                Some(format!("Could not open GPUI benchmark window: {failure}"));
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
