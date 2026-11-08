use makepad_widgets::*;

live_design! {
    pub BenchItems = {{BenchItems}} {
        width: Fill, height: Fit,
        flow: Down
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct BenchItems {
    #[walk]
    walk: Walk,
    #[layout]
    layout: Layout,
    #[redraw]
    #[rust]
    area: Area,
    #[live]
    template: Option<LivePtr>,
    #[rust]
    pub items: Vec<WidgetRef>,
}

impl BenchItems {
    pub fn resize(&mut self, context: &mut Cx, count: usize) -> Result<(), String> {
        let template = self.template.ok_or("Missing native item template")?;
        while self.items.len() < count {
            self.items
                .push(WidgetRef::new_from_ptr(context, Some(template)));
        }
        self.items.truncate(count);
        self.area.redraw(context);
        Ok(())
    }
}

impl Widget for BenchItems {
    fn handle_event(&mut self, context: &mut Cx, event: &Event, scope: &mut Scope) {
        for (index, item) in self.items.iter().enumerate() {
            scope.with_id(LiveId(index as u64), |scope| {
                item.handle_event(context, event, scope)
            });
        }
    }

    fn draw_walk(&mut self, context: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        context.begin_turtle(walk, self.layout);
        for (index, item) in self.items.iter().enumerate() {
            scope.with_id(LiveId(index as u64), |scope| item.draw_all(context, scope));
        }
        context.end_turtle_with_area(&mut self.area);
        DrawStep::done()
    }
}
