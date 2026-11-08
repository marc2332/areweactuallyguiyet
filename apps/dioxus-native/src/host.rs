use std::{
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use benchmark_common::{Harness, Scenario};
use blitz_shell::{BlitzApplication, BlitzShellProxy, WindowConfig};
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
    let config = WindowConfig::with_attributes(
        Box::new(document),
        DioxusNativeWindowRenderer::new(),
        attributes,
    );
    let mut inner = BlitzApplication::new(proxy, event_queue);
    inner.add_window(config);
    let application = Application::<A> {
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
}

impl<A: Adapter> Application<A> {
    fn update_scroll(&mut self) {
        A::update_scroll(&mut self.state, &mut self.inner);
    }
}

impl<A: Adapter> ApplicationHandler for Application<A> {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.inner.can_create_surfaces(event_loop);
        for view in self.inner.windows.values_mut() {
            view.downcast_doc_mut::<DioxusDocument>().initial_build();
            view.poll();
            view.request_redraw();
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
        if rendered
            && !self.ready
            && self
                .inner
                .windows
                .get(&window_id)
                .is_some_and(|view| view.waker.is_some())
        {
            if let Err(error) = self
                .bridge
                .harness
                .borrow_mut()
                .mark_ready("initial-render-returned-not-presented")
            {
                *self.bridge.failure.borrow_mut() = Some(error.to_string());
                event_loop.exit();
            }
            self.ready = true;
        }
    }
    fn proxy_wake_up(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.inner.proxy_wake_up(event_loop);
        self.update_scroll();
    }
    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.update_scroll();
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(16),
        ));
    }
}
