use std::sync::OnceLock;

use benchmark_common::{
    Harness, Scenario, Switches,
    dashboard::{Dashboard, THUMBNAILS},
    row_body, row_title,
};
use makepad_widgets::*;

use crate::items::BenchItems;

static SCENARIO: OnceLock<Scenario> = OnceLock::new();
static FAILURE: OnceLock<String> = OnceLock::new();

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;
    use crate::items::BenchItems;

    BenchText = <Label> {
        padding: 0,
        draw_text: {
            color: #111111,
            text_style: {
                font_size: 10.5,
                font_family: {latin = font("crate://gui-bench-makepad/../../assets/fonts/DejaVuSans.ttf", 0.0, 0.0)}
            }
        }
    }
    BenchInput = <TextInput> {
        width: Fill, height: 32,
        margin: 0, padding: {left: 6, right: 6, top: 4, bottom: 4},
        draw_text: {
            color: #111111, color_hover: #111111, color_focus: #111111, color_down: #111111,
            color_empty: #555555, color_empty_hover: #555555, color_empty_focus: #555555,
            text_style: {
                font_size: 9,
                font_family: {latin = font("crate://gui-bench-makepad/../../assets/fonts/DejaVuSans.ttf", 0.0, 0.0)}
            }
        }
    }
    BenchButton = <Button> {
        margin: 0, padding: 0,
        draw_text: {
            color: #111111, color_hover: #111111, color_focus: #111111, color_down: #111111,
            text_style: {
                font_size: 9,
                font_family: {latin = font("crate://gui-bench-makepad/../../assets/fonts/DejaVuSans.ttf", 0.0, 0.0)}
            }
        }
    }
    BenchRow = <View> {
        width: Fill, height: 48, flow: Down,
        padding: {left: 8, right: 8, top: 4, bottom: 4},
        title = <BenchText> {width: Fill, height: 20, draw_text: {wrap: Line}}
        body = <BenchText> {width: Fill, height: 20, draw_text: {wrap: Line, text_style: {font_size: 9}}}
    }
    BenchWindow = <Root> {
        main_window = <Window> {
            window: {inner_size: vec2(1280, 800), title: "Makepad benchmark"},
            body = <View> {width: Fill, height: Fill, show_bg: true, draw_bg: {color: #ffffff}}
        }
    }
    App = {{App}} {
        startup_template: <BenchWindow> {main_window = {body = {
            flow: Down, padding: 16, spacing: 8,
            <BenchText> {text: "Benchmark startup", height: 24, draw_text: {text_style: {font_size: 15}}}
            <BenchText> {text: "Prepared initial content", height: 20}
            <BenchInput> {width: 430}
            <BenchButton> {width: 120, height: 32, text: "Apply"}
            <ScrollYView> {width: Fill, height: Fill, rows = <BenchItems> {template: <BenchRow> {}}}
        }}}
        scroll_template: <BenchWindow> {main_window = {body = {
            scroller = <ScrollYView> {width: Fill, height: Fill, rows = <BenchItems> {template: <BenchRow> {}}}
        }}}
        virtual_template: <BenchWindow> {main_window = {body = {
            virtual_list = <PortalList> {
                width: Fill, height: Fill, reuse_items: true,
                Row = <BenchRow> {}
            }
        }}}
        switches_template: <BenchWindow> {main_window = {body = {
            cells = <BenchItems> {
                width: 900, height: 900, flow: RightWrap,
                template: <View> {
                    width: 30, height: 30, padding: 0, margin: 0,
                    cursor: Hand, show_bg: true, draw_bg: {color: #868e96},
                    align: {x: 0.5, y: 0.5},
                    value = <BenchText> {
                        width: Fit, height: Fit,
                        align: {x: 0.5, y: 0.5},
                        draw_text: {color: #ffffff, text_style: {font_size: 9}}, text: "OFF"
                    }
                }
            }
        }}}
        memory_template: <BenchWindow> {main_window = {body = {
            flow: Down,
            <View> {
                width: Fill, height: 56, flow: Right, padding: {left: 8, right: 8, top: 10, bottom: 10}, spacing: 8,
                <BenchText> {text: "Dashboard", width: 88, height: 36}
                search = <BenchInput> {height: 36, empty_text: "Search records"}
            }
            <View> {
                width: Fill, height: Fill, flow: Right,
                <View> {
                    width: 180, height: Fill, flow: Down, padding: 8, spacing: 8,
                    show_bg: true, draw_bg: {color: #f1f3f5},
                    sections = <BenchItems> {template: <BenchButton> {width: Fill, height: 22}}
                    thumbnails = <BenchItems> {
                        width: 164, height: 260, flow: RightWrap,
                        template: <Image> {width: 48, height: 48, margin: {right: 16, bottom: 4}, fit: Smallest}
                    }
                }
                scroller = <ScrollYView> {
                    width: 600, height: Fill,
                    rows = <BenchItems> {
                        template: <View> {
                            width: Fill, height: 64, flow: Right,
                            show_bg: true, draw_bg: {color: #ffffff},
                            select = <View> {
                                width: Fill, height: 64, flow: Down, padding: {left: 8, right: 8, top: 4, bottom: 4}, cursor: Hand,
                                title = <BenchText> {width: Fill, height: 20, draw_text: {wrap: Line}}
                                body = <BenchText> {width: Fill, height: 36, draw_text: {wrap: Word, text_style: {font_size: 9}}}
                            }
                            toggle = <BenchButton> {width: 48, height: 24, margin: {top: 20, right: 8}, text: "OFF"}
                        }
                    }
                }
                <View> {
                    width: Fill, height: Fill, flow: Down, padding: 8, spacing: 8,
                    selected_title = <BenchText> {width: Fill, height: 24, draw_text: {text_style: {font_size: 13.5}}}
                    <ScrollYView> {width: Fill, height: 260,
                        detail = <BenchText> {width: Fill, height: Fit, draw_text: {wrap: Word}}
                    }
                    <ScrollYView> {
                        width: Fill, height: Fill,
                        fields = <BenchItems> {
                            template: <View> {
                                width: Fill, height: 36, flow: Right, spacing: 8,
                                name = <BenchText> {width: 80, height: 32}
                                editor = <BenchInput> {}
                            }
                        }
                    }
                }
            }
        }}}
    }
}

app_main!(App);

#[derive(Live, LiveHook)]
pub struct App {
    #[live]
    startup_template: Option<LivePtr>,
    #[live]
    scroll_template: Option<LivePtr>,
    #[live]
    virtual_template: Option<LivePtr>,
    #[live]
    switches_template: Option<LivePtr>,
    #[live]
    memory_template: Option<LivePtr>,
    #[rust]
    ui: WidgetRef,
    #[rust]
    harness: Option<Harness>,
    #[rust]
    dashboard: Option<Dashboard>,
    #[rust]
    switches: Switches,
    #[rust]
    visible_records: Vec<usize>,
    #[rust]
    timer: Timer,
    #[rust]
    drawn: bool,
    #[rust]
    reset_generation: u64,
}

impl LiveRegister for App {
    fn live_register(context: &mut Cx) {
        makepad_widgets::live_design(context);
        crate::items::live_design(context);
    }
}

impl App {
    fn scenario(&self) -> Scenario {
        SCENARIO.get().copied().unwrap_or(Scenario::Startup)
    }

    fn initialize(&mut self, context: &mut Cx) -> Result<(), Box<dyn std::error::Error>> {
        let scenario = self.scenario();
        self.harness = Some(Harness::connect(scenario)?);
        let template = match scenario {
            Scenario::Startup => self.startup_template,
            Scenario::Scroll => self.scroll_template,
            Scenario::VirtualScroll => self.virtual_template,
            Scenario::Switches => self.switches_template,
            Scenario::Memory => self.memory_template,
        }
        .ok_or("Missing scene template")?;
        self.ui = WidgetRef::new_from_ptr(context, Some(template));
        match scenario {
            Scenario::Startup | Scenario::Scroll => {
                let container = self.ui.widget(id!(rows));
                let mut rows = container
                    .borrow_mut::<BenchItems>()
                    .ok_or("Missing native row container")?;
                rows.resize(context, scenario.row_count())?;
                for (index, row) in rows.items.iter().enumerate() {
                    row.label(id!(title)).set_text(context, &row_title(index));
                    row.label(id!(body)).set_text(context, &row_body(index));
                }
            }
            Scenario::VirtualScroll => {
                self.ui
                    .portal_list(id!(virtual_list))
                    .borrow_mut()
                    .ok_or("Missing native virtual list")?
                    .set_item_range(context, 0, 100_000);
            }
            Scenario::Switches => {
                let container = self.ui.widget(id!(cells));
                container
                    .borrow_mut::<BenchItems>()
                    .ok_or("Missing native cell container")?
                    .resize(context, 900)?;
            }
            Scenario::Memory => {
                self.dashboard = Some(Dashboard::default());
                for (identifier, count) in [
                    (id!(sections), 20),
                    (id!(fields), 20),
                    (id!(thumbnails), 10),
                ] {
                    let container = self.ui.widget(identifier);
                    container
                        .borrow_mut::<BenchItems>()
                        .ok_or("Missing dashboard container")?
                        .resize(context, count)?;
                }
                let container = self.ui.widget(id!(sections));
                if let Some(sections) = container.borrow::<BenchItems>() {
                    for (index, section) in sections.items.iter().enumerate() {
                        section.set_text(
                            context,
                            &if index == 0 {
                                "All records".to_owned()
                            } else {
                                format!("Section {index}")
                            },
                        );
                    }
                }
                let container = self.ui.widget(id!(thumbnails));
                if let Some(thumbnails) = container.borrow::<BenchItems>() {
                    for (index, image) in thumbnails.items.iter().enumerate() {
                        image
                            .as_image()
                            .load_png_from_data(context, THUMBNAILS[index])
                            .map_err(|error| format!("PNG fixture failed: {error:?}"))?;
                    }
                }
                self.sync_dashboard(context)?;
            }
        }
        self.timer = context.start_interval(0.016);
        self.ui.redraw(context);
        Ok(())
    }

    fn sync_dashboard(&mut self, context: &mut Cx) -> Result<(), String> {
        let state = self.dashboard.as_ref().ok_or("Missing dashboard state")?;
        let visible = state.visible_records();
        if visible != self.visible_records {
            self.ui
                .view(id!(scroller))
                .set_scroll_pos(context, dvec2(0.0, 0.0));
        }
        let container = self.ui.widget(id!(rows));
        let mut rows = container
            .borrow_mut::<BenchItems>()
            .ok_or("Missing dashboard rows")?;
        rows.resize(context, visible.len())?;
        for (row, index) in rows.items.iter().zip(&visible) {
            let record = &state.fixtures.records[*index];
            row.label(id!(title)).set_text(context, &record.title);
            row.label(id!(body)).set_text(context, &record.body);
            let color = if state.selected_record == *index {
                vec4(0.89, 0.93, 1.0, 1.0)
            } else {
                vec4(1.0, 1.0, 1.0, 1.0)
            };
            row.apply_over(context, live! { draw_bg: {color: (color)} });
            row.button(id!(toggle)).set_text(
                context,
                if state.active.get(*index) {
                    "ON"
                } else {
                    "OFF"
                },
            );
        }
        drop(rows);
        self.visible_records = visible;
        let search = self.ui.text_input(id!(search));
        if search.text() != state.query {
            search.set_text(context, &state.query);
        }
        self.ui.label(id!(selected_title)).set_text(
            context,
            &state.fixtures.records[state.selected_record].title,
        );
        self.ui.label(id!(detail)).set_text(context, &state.detail);
        let container = self.ui.widget(id!(fields));
        let fields = container
            .borrow::<BenchItems>()
            .ok_or("Missing property inputs")?;
        for (index, row) in fields.items.iter().enumerate() {
            row.label(id!(name))
                .set_text(context, &format!("Field {index}"));
            let editor = row.text_input(id!(editor));
            if editor.text() != state.fields[index] {
                editor.set_text(context, &state.fields[index]);
            }
        }
        drop(fields);
        self.ui.redraw(context);
        Ok(())
    }

    fn set_cell(context: &mut Cx, cell: &WidgetRef, active: bool) {
        let color = if active {
            vec4(47.0 / 255.0, 158.0 / 255.0, 68.0 / 255.0, 1.0)
        } else {
            vec4(134.0 / 255.0, 142.0 / 255.0, 150.0 / 255.0, 1.0)
        };
        cell.apply_over(context, live! {draw_bg: {color: (color)}});
        cell.label(id!(value))
            .set_text(context, if active { "ON" } else { "OFF" });
        cell.redraw(context);
    }

    fn tick(&mut self, context: &mut Cx) -> Result<(), Box<dyn std::error::Error>> {
        let Some(harness) = &mut self.harness else {
            return Ok(());
        };
        if self.drawn {
            harness
                .mark_ready("native-draw-processed-and-following-event-loop-tick-not-presented")?;
        }
        let frame = harness.poll()?;
        let mode = harness.switch_mode();
        if frame.should_quit {
            context.quit();
            return Ok(());
        }
        if frame.reset_generation != self.reset_generation {
            self.switches = Switches::default();
            match self.scenario() {
                Scenario::Switches => {
                    let container = self.ui.widget(id!(cells));
                    if let Some(cells) = container.borrow::<BenchItems>() {
                        for cell in &cells.items {
                            Self::set_cell(context, cell, false);
                        }
                    }
                }
                Scenario::Memory => {
                    if let Some(state) = &mut self.dashboard {
                        state.reset();
                    }
                    self.sync_dashboard(context)?;
                    self.ui
                        .view(id!(scroller))
                        .set_scroll_pos(context, dvec2(0.0, 0.0));
                }
                Scenario::Scroll => self
                    .ui
                    .view(id!(scroller))
                    .set_scroll_pos(context, dvec2(0.0, 0.0)),
                Scenario::VirtualScroll => self
                    .ui
                    .portal_list(id!(virtual_list))
                    .set_first_id_and_scroll(0, 0.0),
                Scenario::Startup => {}
            }
            self.reset_generation = frame.reset_generation;
            self.ui.redraw(context);
        }
        if frame.running {
            match self.scenario() {
                Scenario::Scroll => {
                    self.ui
                        .view(id!(scroller))
                        .set_scroll_pos(context, dvec2(0.0, f64::from(frame.scroll_offset)));
                    self.ui.view(id!(scroller)).redraw(context);
                }
                Scenario::VirtualScroll => {
                    let offset = f64::from(frame.scroll_offset);
                    let list = self.ui.portal_list(id!(virtual_list));
                    list.set_first_id_and_scroll(
                        (offset / 48.0).floor() as usize,
                        -(offset % 48.0),
                    );
                    list.redraw(context);
                }
                Scenario::Switches => {
                    let container = self.ui.widget(id!(cells));
                    if let Some(cells) = container.borrow::<BenchItems>() {
                        self.switches
                            .advance_changed(frame.update_index, mode, |index, active| {
                                Self::set_cell(context, &cells.items[index], active)
                            });
                    }
                }
                Scenario::Memory => {
                    if let Some(state) = &mut self.dashboard {
                        if state.applied_updates() != frame.update_index {
                            state.advance(frame);
                            self.sync_dashboard(context)?;
                        }
                    }
                    self.ui
                        .view(id!(scroller))
                        .set_scroll_pos(context, dvec2(0.0, f64::from(frame.scroll_offset)));
                    self.ui.view(id!(scroller)).redraw(context);
                }
                Scenario::Startup => {}
            }
        }
        let applied = self.dashboard.as_ref().map_or_else(
            || {
                if self.scenario() == Scenario::Switches {
                    self.switches.applied_updates()
                } else {
                    frame.update_index
                }
            },
            Dashboard::applied_updates,
        );
        if let Some(harness) = &mut self.harness {
            harness.report_applied(applied)?;
            if let Some(state) = &self.dashboard {
                harness.report_dashboard(state)?;
            }
        }
        Ok(())
    }

    fn fail(&self, context: &mut Cx, error: impl std::fmt::Display) {
        let _ = FAILURE.set(error.to_string());
        context.quit();
    }
}

impl MatchEvent for App {
    fn handle_startup(&mut self, context: &mut Cx) {
        if let Err(error) = self.initialize(context) {
            self.fail(context, error);
        }
    }

    fn handle_actions(&mut self, context: &mut Cx, actions: &Actions) {
        if self.scenario() == Scenario::Switches {
            let container = self.ui.widget(id!(cells));
            if let Some(cells) = container.borrow::<BenchItems>() {
                for (index, cell) in cells.items.iter().enumerate() {
                    if cell
                        .as_view()
                        .finger_up(actions)
                        .is_some_and(|event| event.is_over)
                    {
                        self.switches.toggle(index);
                        Self::set_cell(context, cell, self.switches.get(index));
                    }
                }
            }
        }
        let Some(state) = &mut self.dashboard else {
            return;
        };
        let mut changed = false;
        if let Some(query) = self.ui.text_input(id!(search)).changed(actions) {
            state.search(query);
            changed = true;
        }
        let container = self.ui.widget(id!(sections));
        if let Some(sections) = container.borrow::<BenchItems>() {
            for (index, section) in sections.items.iter().enumerate() {
                if section.as_button().clicked(actions) {
                    state.navigate(index);
                    changed = true;
                }
            }
        }
        let container = self.ui.widget(id!(rows));
        if let Some(rows) = container.borrow::<BenchItems>() {
            for (row, index) in rows.items.iter().zip(&self.visible_records) {
                if row
                    .view(id!(select))
                    .finger_up(actions)
                    .is_some_and(|event| event.is_over)
                {
                    state.select(*index);
                    changed = true;
                }
                if row.button(id!(toggle)).clicked(actions) {
                    state.toggle(*index);
                    changed = true;
                }
            }
        }
        let container = self.ui.widget(id!(fields));
        if let Some(fields) = container.borrow::<BenchItems>() {
            for (index, row) in fields.items.iter().enumerate() {
                if let Some(value) = row.text_input(id!(editor)).changed(actions) {
                    state.edit(index, value);
                    changed = true;
                }
            }
        }
        if changed {
            if let Err(error) = self.sync_dashboard(context) {
                self.fail(context, error);
            }
        }
    }
}

impl AppMain for App {
    fn handle_event(&mut self, context: &mut Cx, event: &Event) {
        if let Event::Draw(draw) = event {
            let mut drawing = CxDraw::new(context, draw);
            let mut context = Cx2d::new(&mut drawing);
            let mut scope = Scope::empty();
            while let Some(widget) = self.ui.draw(&mut context, &mut scope).step() {
                if let Some(mut list) = widget.as_portal_list().borrow_mut() {
                    while let Some(index) = list.next_visible_item(&mut context) {
                        let row = list.item(&mut context, index, live_id!(Row));
                        row.label(id!(title))
                            .set_text(&mut context, &row_title(index));
                        row.label(id!(body))
                            .set_text(&mut context, &row_body(index));
                        row.draw_all(&mut context, &mut scope);
                    }
                }
            }
            self.drawn = true;
            return;
        }
        self.match_event(context, event);
        self.ui.handle_event(context, event, &mut Scope::empty());
        if self.timer.is_event(event).is_some() {
            if let Err(error) = self.tick(context) {
                self.fail(context, error);
            }
        }
    }
}

pub fn run(scenario: Scenario) -> Result<(), Box<dyn std::error::Error>> {
    SCENARIO
        .set(scenario)
        .map_err(|_| "Makepad scenario already initialized")?;
    app_main();
    if let Some(error) = FAILURE.get() {
        return Err(error.clone().into());
    }
    Ok(())
}
