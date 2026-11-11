#[path = "../../scroll_config.rs"]
mod scroll_config;

use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    ItemRow = <View> {
        width: 960, height: 48,
        padding: 12,
        show_bg: true,
        draw_bg: {color: #f5f5f5}
        label = <Label> {
            padding: 0,
            // Makepad converts 10.5 points to 14 logical pixels.
            draw_text: {color: #000000, text_style: {font_size: 10.5}}
        }
    }

    App = {{App}} {
        ui: <Root> {
            main_window = <Window> {
                window: {inner_size: vec2(1000, 700), title: "Makepad Scroll"}
                body = <FlatList> {
                    width: Fill, height: Fill,
                    Even = <ItemRow> {}
                    Odd = <ItemRow> {draw_bg: {color: #e6e6e6}}
                }
            }
        }
    }
}

app_main!(App);

#[derive(Live, LiveHook)]
struct App {
    #[live]
    ui: WidgetRef,
    #[rust]
    rows: Vec<WidgetRef>,
}

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
    }
}

impl MatchEvent for App {
    fn handle_startup(&mut self, cx: &mut Cx) {
        let list = self.ui.flat_list(id!(body));
        for index in 0..scroll_config::row_count() as u64 {
            let template = if index % 2 == 0 {
                live_id!(Even)
            } else {
                live_id!(Odd)
            };
            if let Some(row) = list.item(cx, LiveId(index), template) {
                row.label(id!(label))
                    .set_text(cx, &format!("Item {}", index + 1));
                self.rows.push(row);
            }
        }
    }
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::Draw(event) = event {
            let mut draw = CxDraw::new(cx, event);
            let mut cx = Cx2d::new(&mut draw);
            let mut scope = Scope::empty();
            while let Some(widget) = self.ui.draw(&mut cx, &mut scope).step() {
                if widget.as_flat_list().borrow().is_some() {
                    // FlatList owns native scrolling, while every retained row is drawn in order.
                    for row in &self.rows {
                        row.draw_all(&mut cx, &mut scope);
                    }
                }
            }
        } else {
            self.match_event(cx, event);
            self.ui.handle_event(cx, event, &mut Scope::empty());
        }
    }
}

pub fn run() {
    app_main();
}
