use std::{
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use benchmark_common::{Harness, Scenario};
use blitz_dom::{BaseDocument, NodeId, StyleThreading};
use blitz_shell::{BlitzApplication, BlitzShellProxy, WindowConfig};
use dioxus_native::{DioxusDocument, DioxusNativeWindowRenderer, DocumentConfig, prelude::*};
use winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{WindowAttributes, WindowId},
};

use crate::ui::{App, Bridge};

pub fn run(scenario: Scenario) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let _runtime = runtime.enter();
    let event_loop = EventLoop::new()?;
    let (proxy, event_queue) = BlitzShellProxy::new(event_loop.create_proxy());
    let bridge = Rc::new(Bridge::new(scenario, Harness::connect(scenario)?));
    let document = DioxusDocument::new(
        VirtualDom::new_with_props(
            App,
            crate::ui::AppProps {
                bridge: Rc::clone(&bridge),
            },
        ),
        DocumentConfig {
            net_provider: Some(Arc::new(blitz_net::Provider::new(Some(Arc::new(
                proxy.clone(),
            ))))),
            style_threading: StyleThreading::Parallel,
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
    let application = Application {
        inner,
        bridge: Rc::clone(&bridge),
        ready: false,
        scroll_node: None,
    };
    event_loop.run_app(application)?;
    if let Some(error) = bridge.failure.borrow_mut().take() {
        return Err(error.into());
    }
    Ok(())
}

struct Application {
    inner: BlitzApplication<DioxusNativeWindowRenderer>,
    bridge: Rc<Bridge>,
    ready: bool,
    scroll_node: Option<NodeId>,
}

impl Application {
    fn update_scroll(&mut self) {
        if !matches!(
            self.bridge.scenario,
            Scenario::Scroll | Scenario::VirtualScroll | Scenario::Memory
        ) {
            return;
        }
        for view in self.inner.windows.values_mut() {
            if self.scroll_node.is_none() {
                let document = view.doc.inner();
                self.scroll_node = find_scroll_node(&document, document.root_node().id);
            }
            let Some(node_id) = self.scroll_node else {
                continue;
            };
            if let Some(target) = self.bridge.scroll_target.take() {
                {
                    let mut document = view.doc.inner_mut();
                    if let Some(node) = document.get_node_mut(node_id) {
                        let maximum = node.final_layout().scroll_height().max(0.0) as f64;
                        node.scroll_offset_mut().y = target.min(maximum).max(0.0);
                    }
                }
                view.request_redraw();
            }
            if self.bridge.scenario == Scenario::VirtualScroll {
                let offset = view
                    .doc
                    .inner()
                    .get_node(node_id)
                    .map_or(0.0, |node| node.scroll_offset().y as f32);
                if let Some(mut position) = *self.bridge.virtual_position.borrow() {
                    let changed = *position.peek() != offset;
                    if changed {
                        view.downcast_doc_mut::<DioxusDocument>()
                            .vdom
                            .in_scope(ScopeId::ROOT, || position.set(offset));
                        view.poll();
                    }
                }
            }
        }
    }
}

impl ApplicationHandler for Application {
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

fn find_scroll_node(document: &BaseDocument, node_id: NodeId) -> Option<NodeId> {
    let node = document.get_node(node_id)?;
    if node
        .element_data()
        .and_then(|element| element.id.as_ref())
        .is_some_and(|id| &**id == "benchmark-scroll")
    {
        return Some(node_id);
    }
    node.children
        .iter()
        .find_map(|child| find_scroll_node(document, *child))
}
