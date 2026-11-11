use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    ProjectRow = <View> {
        width: Fill, height: 104,
        padding: 12,
        spacing: 16,
        flow: Right,
        show_bg: true,
        draw_bg: {color: #f5f5f5}
        image = <Image> {
            width: 64, height: 64,
            source: dep("crate://self/../../assets/images/dashboard-0.png")
        }
        details = <View> {
            width: Fill, height: Fill,
            flow: Down,
            spacing: 8,
            title = <Label> {
                text: "Project";
                draw_text: {color: #111111, text_style: {font_size: 13.0}}
            }
            description = <Label> {
                text: "A project record with notes, ownership details, and recent activity.";
                draw_text: {color: #555555, text_style: {font_size: 10.0}}
            }
        }
    }

    App = {{App}} {
        ui: <Root> {
            main_window = <Window> {
                window: {inner_size: vec2(1000, 700), title: "Makepad Scroll 2000"}
                body = <View> {
                    width: Fill, height: Fill,
                    flow: Down,
                    show_bg: true,
                    draw_bg: {color: #ffffff}
                    header = <View> {
                        width: Fill, height: 180,
                        flow: Down,
                        padding: {left: 24, top: 16, right: 24, bottom: 0}
                        spacing: 22,
                        heading = <Label> {
                            width: Fill, height: 32,
                            text: "Project dashboard";
                            draw_text: {color: #111111, text_style: {font_size: 18.0}}
                        }
                        toolbar = <View> {
                            width: Fill, height: 40,
                            flow: Right,
                            spacing: 0,
                            notes = <TextInput> {
                                width: 360, height: 40,
                                margin: 0,
                                text: "Project notes",
                                empty_text: "Project notes"
                            }
                            notes_gap = <View> {width: 32, height: 1}
                            refresh = <Button> {
                                width: 160, height: 40,
                                margin: 0,
                                text: "Refresh (0)"
                                draw_text: {color: #111111}
                            }
                            spacer = <View> {width: 48, height: 1}
                            toggle = <CheckBox> {
                                width: 160, height: 40,
                                margin: 0,
                                text: "Enabled",
                                draw_text: {color: #111111}
                            }
                        }
                        status = <Label> {
                            width: Fill, height: 24,
                            text: "Ready · Disabled";
                            draw_text: {color: #444444, text_style: {font_size: 10.0}}
                        }
                    }
                    records = <FlatList> {
                        width: Fill, height: Fill,
                        padding: {left: 20, right: 20, top: 0, bottom: 0}
                        Even = <ProjectRow> {}
                        Odd = <ProjectRow> {draw_bg: {color: #e6e6e6}}
                    }
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
    refresh_count: usize,
    #[rust]
    enabled: bool,
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
        self.refresh_count = 0;
        self.enabled = false;
        self.ui
            .text_input(id!(body.header.toolbar.notes))
            .set_text(cx, "Project notes");
        let list = self.ui.flat_list(id!(body.records));
        for index in 0..1000 {
            let template = if index % 2 == 0 {
                live_id!(Even)
            } else {
                live_id!(Odd)
            };
            if let Some(row) = list.item(cx, LiveId(index), template) {
                row.label(id!(details.title))
                    .set_text(cx, &format!("Project {}", index + 1));
                self.rows.push(row);
            }
        }
    }

    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        let refresh = self.ui.button(id!(body.header.toolbar.refresh));
        if refresh.clicked(actions) {
            self.refresh_count += 1;
            refresh.set_text(cx, &format!("Refresh ({})", self.refresh_count));
            self.update_status(cx);
        }
        if let Some(enabled) = self
            .ui
            .check_box(id!(body.header.toolbar.toggle))
            .changed(actions)
        {
            self.enabled = enabled;
            self.update_status(cx);
        }
    }
}

impl App {
    fn update_status(&self, cx: &mut Cx) {
        let state = if self.enabled { "Enabled" } else { "Disabled" };
        let text = if self.refresh_count == 0 {
            format!("Ready · {state}")
        } else {
            format!("Refreshed {} times · {state}", self.refresh_count)
        };
        self.ui.label(id!(body.header.status)).set_text(cx, &text);
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
