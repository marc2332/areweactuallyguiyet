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
        if self.ready_sent {
            return;
        }
        let PluginEvent::AfterPresenting { window, .. } = event else {
            return;
        };
        #[cfg(feature = "hover")]
        if !matches!(
            window.fullscreen(),
            Some(freya_winit::winit::window::Fullscreen::Borderless(_))
        ) || !window.current_monitor().is_some_and(|monitor| {
            let size = window.inner_size();
            size.width > 0 && size.height > 0 && size == monitor.size()
        }) {
            return;
        }
        self.ready_sent = true;
        #[cfg(feature = "hover")]
        let result = {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            let backend = match window.window_handle().ok().map(|handle| handle.as_raw()) {
                Some(RawWindowHandle::Xlib(_) | RawWindowHandle::Xcb(_)) => "x11",
                Some(RawWindowHandle::Wayland(_)) => "wayland",
                _ => "unspecified",
            };
            self.context.harness.borrow_mut().mark_hover_ready(
                "initial-present-returned-no-display-confirmation",
                backend,
                window.scale_factor(),
                (f64::from(window.inner_size().width) / window.scale_factor()) as f32,
                (f64::from(window.inner_size().height) / window.scale_factor()) as f32,
            )
        };
        #[cfg(not(feature = "hover"))]
        let result = {
            let _ = window;
            self.context
                .harness
                .borrow_mut()
                .mark_ready("initial-present-returned-no-display-confirmation")
        };
        if let Err(error) = result {
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
            .with_window(
                WindowConfig::new(app)
                    .with_root_context(context)
                    .with_size(
                        f64::from(benchmark_common::WIDTH),
                        f64::from(benchmark_common::HEIGHT),
                    )
                    .with_window_attributes(|attributes, _| {
                        #[cfg(feature = "hover")]
                        let attributes = attributes.with_decorations(false).with_fullscreen(Some(
                            freya_winit::winit::window::Fullscreen::Borderless(None),
                        ));
                        attributes
                    }),
            ),
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
