use std::{cell::RefCell, rc::Rc};

use benchmark_common::{Harness, Scenario};
#[cfg(any(feature = "startup", feature = "scroll", feature = "virtual-scroll"))]
use benchmark_common::{row_body, row_title};
use freya::prelude::launch as freya_launch;
use freya::prelude::*;
use freya_winit::{
    plugins::{FreyaPlugin, PluginEvent, PluginHandle},
    renderer::NativeGenericEvent,
};

#[derive(Clone)]
pub struct HarnessContext {
    pub harness: Rc<RefCell<Harness>>,
    pub failure: Rc<RefCell<Option<String>>>,
}

impl HarnessContext {
    pub fn connect(scenario: Scenario) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            harness: Rc::new(RefCell::new(Harness::connect(scenario)?)),
            failure: Rc::new(RefCell::new(None)),
        })
    }
}

struct InitialFrameReady {
    context: HarnessContext,
    ready_sent: bool,
}

impl FreyaPlugin for InitialFrameReady {
    fn plugin_id(&self) -> &'static str {
        "benchmark-initial-frame-ready"
    }

    fn on_event(&mut self, event: &mut PluginEvent, handle: PluginHandle) {
        if self.ready_sent || !matches!(event, PluginEvent::AfterPresenting { .. }) {
            return;
        }
        self.ready_sent = true;
        if let Err(error) = self
            .context
            .harness
            .borrow_mut()
            .mark_ready("initial-present-returned-no-display-confirmation")
        {
            *self.context.failure.borrow_mut() = Some(error.to_string());
            handle.send_event_loop_event(NativeEvent::Generic(NativeGenericEvent::User(
                GlobalUserEvent::Exit,
            )));
        }
    }
}

pub fn launch(
    app: fn() -> Element,
    context: HarnessContext,
    failure: Rc<RefCell<Option<String>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    freya_launch(
        LaunchConfig::new()
            .with_plugin(InitialFrameReady {
                context: context.clone(),
                ready_sent: false,
            })
            .with_default_font("DejaVu Sans")
            .with_font(
                "DejaVu Sans",
                Bytes::from_static(include_bytes!("../../../assets/fonts/DejaVuSans.ttf")),
            )
            .with_window(WindowConfig::new(app).with_root_context(context).with_size(
                f64::from(benchmark_common::WIDTH),
                f64::from(benchmark_common::HEIGHT),
            )),
    );
    match failure.borrow_mut().take() {
        Some(message) => Err(message.into()),
        None => Ok(()),
    }
}

#[cfg(any(feature = "startup", feature = "scroll", feature = "virtual-scroll"))]
pub fn row(index: usize) -> Element {
    rect()
        .key(index)
        .height(Size::px(benchmark_common::ROW_HEIGHT))
        .padding(4.)
        .child(label().text(row_title(index)))
        .child(label().text(row_body(index)))
        .into()
}
