use crate::host::Windows;
use blitz_dom::{BaseDocument, NodeId};
use std::cell::Cell;

pub fn apply<const TRACK_POSITION: bool>(
    windows: &mut Windows,
    node_id: &mut Option<NodeId>,
    target: &Cell<Option<f64>>,
    mut offset_changed: impl FnMut(&mut dioxus_native::DioxusDocument, f32) -> bool,
) {
    for view in windows.windows.values_mut() {
        if node_id.is_none() {
            let document = view.doc.inner();
            *node_id = find_scroll_node(&document, document.root_node().id);
        }
        let Some(id) = *node_id else { continue };
        if let Some(target) = target.take() {
            {
                let mut document = view.doc.inner_mut();
                if let Some(node) = document.get_node_mut(id) {
                    let maximum = node.final_layout().scroll_height().max(0.0) as f64;
                    node.scroll_offset_mut().y = target.min(maximum).max(0.0);
                }
            }
            view.request_redraw();
        }
        let offset = view
            .doc
            .inner()
            .get_node(id)
            .map_or(0.0, |node| node.scroll_offset().y as f32);
        if offset_changed(
            view.downcast_doc_mut::<dioxus_native::DioxusDocument>(),
            offset,
        ) {
            view.poll();
        }
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
