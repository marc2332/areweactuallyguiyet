use std::{
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use benchmark_common::{Harness, Scenario};
use blitz_shell::{BlitzApplication, BlitzShellEvent, BlitzShellProxy, WindowConfig};
use dioxus_native::{DioxusDocument, DioxusNativeWindowRenderer, DocumentConfig, prelude::*};
use winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{WindowAttributes, WindowId},
};

use crate::ui::Bridge;

pub type Windows = BlitzApplication<DioxusNativeWindowRenderer>;

pub trait Adapter: 'static {
    type State: 'static;
    const SCENARIO: Scenario;
    fn create(bridge: Rc<Bridge>) -> (VirtualDom, Self::State);
    fn update_scroll(_state: &mut Self::State, _windows: &mut Windows) {}
}

pub fn run<A: Adapter>() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let _runtime = runtime.enter();
    let event_loop = EventLoop::new()?;
    let (proxy, event_queue) = BlitzShellProxy::new(event_loop.create_proxy());
    let bridge = Rc::new(Bridge::new(Harness::connect(A::SCENARIO)?));
    let (vdom, state) = A::create(Rc::clone(&bridge));
    let document = DioxusDocument::new(
        vdom,
        DocumentConfig {
            net_provider: Some(Arc::new(blitz_net::Provider::new(Some(Arc::new(
                proxy.clone(),
            ))))),
            style_threading: blitz_dom::StyleThreading::Parallel,
            incremental: Some(true),
            ..Default::default()
        },
    );
    let attributes = WindowAttributes::default()
        .with_title("Dioxus Native benchmark")
        .with_surface_size(winit::dpi::LogicalSize::new(
            benchmark_common::WIDTH,
            benchmark_common::HEIGHT,
        ));
    #[cfg(feature = "hover")]
    let attributes = attributes
        .with_decorations(false)
        .with_fullscreen(Some(winit::monitor::Fullscreen::Borderless(None)));
    let config = WindowConfig::with_attributes(
        Box::new(document),
        DioxusNativeWindowRenderer::new(),
        attributes,
    );
    let mut inner = BlitzApplication::new(proxy, event_queue);
    inner.add_window(config);
    #[cfg(feature = "hover")]
    let hover_size = std::env::var("GUI_BENCH_HOVER_WIDTH")
        .ok()
        .zip(std::env::var("GUI_BENCH_HOVER_HEIGHT").ok())
        .map(|(width, height)| {
            Ok::<_, std::num::ParseIntError>((width.parse::<u32>()?, height.parse::<u32>()?))
        })
        .transpose()?;
    let application = Application::<A> {
        #[cfg(feature = "hover")]
        hover_size,
        inner,
        bridge: Rc::clone(&bridge),
        state,
        ready: false,
    };
    event_loop.run_app(application)?;
    if let Some(error) = bridge.failure.borrow_mut().take() {
        return Err(error.into());
    }
    Ok(())
}

struct Application<A: Adapter> {
    inner: Windows,
    bridge: Rc<Bridge>,
    state: A::State,
    ready: bool,
    #[cfg(feature = "hover")]
    hover_size: Option<(u32, u32)>,
}

impl<A: Adapter> Application<A> {
    fn update_scroll(&mut self) {
        A::update_scroll(&mut self.state, &mut self.inner);
    }

    fn try_ready(&mut self, window_id: WindowId, event_loop: &dyn ActiveEventLoop) {
        if self.ready {
            return;
        }
        let Some(view) = self.inner.windows.get(&window_id) else {
            return;
        };
        if view.waker.is_none() {
            return;
        }
        #[cfg(feature = "hover")]
        {
            let size = view.window.surface_size();
            if size.width == 0
                || size.height == 0
                || self
                    .hover_size
                    .is_some_and(|expected| expected != (size.width, size.height))
                || view.doc.inner().viewport().window_size != (size.width, size.height)
            {
                return;
            }
        }
        #[cfg(feature = "hover")]
        let result = {
            use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
            let backend = match view
                .window
                .window_handle()
                .ok()
                .map(|handle| handle.as_raw())
            {
                Some(RawWindowHandle::Xlib(_) | RawWindowHandle::Xcb(_)) => "x11",
                Some(RawWindowHandle::Wayland(_)) => "wayland",
                _ => "unspecified",
            };
            self.bridge.harness.borrow_mut().mark_hover_ready(
                "initial-render-returned-not-presented",
                backend,
                view.window.scale_factor(),
                (f64::from(view.window.surface_size().width) / view.window.scale_factor()) as f32,
                (f64::from(view.window.surface_size().height) / view.window.scale_factor()) as f32,
            )
        };
        #[cfg(not(feature = "hover"))]
        let result = self
            .bridge
            .harness
            .borrow_mut()
            .mark_ready("initial-render-returned-not-presented");
        if let Err(error) = result {
            *self.bridge.failure.borrow_mut() = Some(error.to_string());
            event_loop.exit();
            return;
        }
        self.ready = true;
    }
}

impl<A: Adapter> ApplicationHandler for Application<A> {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.inner.can_create_surfaces(event_loop);
        for view in self.inner.windows.values_mut() {
            view.downcast_doc_mut::<DioxusDocument>().initial_build();
        }
    }
    fn destroy_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.inner.destroy_surfaces(event_loop);
    }
    fn resumed(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.inner.resumed(event_loop);
    }
    fn suspended(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.inner.suspended(event_loop);
    }
    fn new_events(&mut self, event_loop: &dyn ActiveEventLoop, cause: StartCause) {
        self.inner.new_events(event_loop, cause);
        if self.bridge.quit.get() {
            self.inner.windows.clear();
            event_loop.exit();
        }
    }
    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let rendered = matches!(event, WindowEvent::RedrawRequested);
        self.inner.window_event(event_loop, window_id, event);
        self.update_scroll();
        if rendered {
            self.try_ready(window_id, event_loop);
        }
    }
    fn proxy_wake_up(&mut self, event_loop: &dyn ActiveEventLoop) {
        while let Ok(event) = self.inner.event_queue.try_recv() {
            let resumed_window = match &event {
                BlitzShellEvent::ResumeReady { window_id } => Some(*window_id),
                _ => None,
            };
            self.inner.handle_blitz_shell_event(event_loop, event);
            if let Some(window_id) = resumed_window {
                if let Some(view) = self.inner.windows.get_mut(&window_id) {
                    view.poll();
                    view.request_redraw();
                }
                self.try_ready(window_id, event_loop);
            }
        }
        self.update_scroll();
    }
    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.update_scroll();
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(16),
        ));
    }
}
