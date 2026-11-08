use crate::items::BenchItems;
use crate::ui::Scene as SceneBehavior;
use benchmark_common::{
    FrameState, Harness, Scenario, SwitchMode,
    dashboard::{Dashboard, THUMBNAILS},
};
use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;
    use crate::items::BenchItems;
    use crate::widgets::BenchWindow;
    use crate::widgets::BenchText;
    use crate::widgets::BenchInput;
    use crate::widgets::BenchButton;

    pub SceneTemplate = <BenchWindow> {main_window = {body = {
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

#[derive(Default)]
pub(crate) struct Scene {
    dashboard: Option<Dashboard>,
    visible_records: Vec<usize>,
}

impl SceneBehavior for Scene {
    const SCENARIO: Scenario = Scenario::Memory;
    fn initialize(
        &mut self,
        context: &mut Cx,
        ui: &WidgetRef,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.dashboard = Some(Dashboard::default());
        for (identifier, count) in [
            (id!(sections), 20),
            (id!(fields), 20),
            (id!(thumbnails), 10),
        ] {
            let container = ui.widget(identifier);
            container
                .borrow_mut::<BenchItems>()
                .ok_or("Missing dashboard container")?
                .resize(context, count)?;
        }
        let container = ui.widget(id!(sections));
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
        let container = ui.widget(id!(thumbnails));
        if let Some(thumbnails) = container.borrow::<BenchItems>() {
            for (index, image) in thumbnails.items.iter().enumerate() {
                image
                    .as_image()
                    .load_png_from_data(context, THUMBNAILS[index])
                    .map_err(|error| format!("PNG fixture failed: {error:?}"))?;
            }
        }
        self.sync_dashboard(context, ui)?;
        Ok(())
    }

    fn reset(
        &mut self,
        context: &mut Cx,
        ui: &WidgetRef,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(state) = &mut self.dashboard {
            state.reset();
        }
        self.sync_dashboard(context, ui)?;
        ui.view(id!(scroller))
            .set_scroll_pos(context, dvec2(0.0, 0.0));
        Ok(())
    }

    fn update(
        &mut self,
        context: &mut Cx,
        ui: &WidgetRef,
        frame: FrameState,
        _mode: SwitchMode,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(state) = &mut self.dashboard {
            if state.applied_updates() != frame.update_index {
                state.advance(frame);
                self.sync_dashboard(context, ui)?;
            }
        }
        ui.view(id!(scroller))
            .set_scroll_pos(context, dvec2(0.0, f64::from(frame.scroll_offset)));
        ui.view(id!(scroller)).redraw(context);
        Ok(())
    }

    fn applied(&self, frame: FrameState) -> u64 {
        self.dashboard
            .as_ref()
            .map_or(frame.update_index, Dashboard::applied_updates)
    }

    fn report(&self, harness: &mut Harness) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(state) = &self.dashboard {
            harness.report_dashboard(state)?;
        }
        Ok(())
    }

    fn actions(
        &mut self,
        context: &mut Cx,
        ui: &WidgetRef,
        actions: &Actions,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(state) = &mut self.dashboard else {
            return Ok(());
        };
        let mut changed = false;
        if let Some(query) = ui.text_input(id!(search)).changed(actions) {
            state.search(query);
            changed = true;
        }
        let container = ui.widget(id!(sections));
        if let Some(sections) = container.borrow::<BenchItems>() {
            for (index, section) in sections.items.iter().enumerate() {
                if section.as_button().clicked(actions) {
                    state.navigate(index);
                    changed = true;
                }
            }
        }
        let container = ui.widget(id!(rows));
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
        let container = ui.widget(id!(fields));
        if let Some(fields) = container.borrow::<BenchItems>() {
            for (index, row) in fields.items.iter().enumerate() {
                if let Some(value) = row.text_input(id!(editor)).changed(actions) {
                    state.edit(index, value);
                    changed = true;
                }
            }
        }
        if changed {
            if let Err(error) = self.sync_dashboard(context, ui) {
                return Err(error.into());
            }
        }
        Ok(())
    }
}

impl Scene {
    fn sync_dashboard(&mut self, context: &mut Cx, ui: &WidgetRef) -> Result<(), String> {
        let state = self.dashboard.as_ref().ok_or("Missing dashboard state")?;
        let visible = state.visible_records();
        if visible != self.visible_records {
            ui.view(id!(scroller))
                .set_scroll_pos(context, dvec2(0.0, 0.0));
        }
        let container = ui.widget(id!(rows));
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
        let search = ui.text_input(id!(search));
        if search.text() != state.query {
            search.set_text(context, &state.query);
        }
        ui.label(id!(selected_title)).set_text(
            context,
            &state.fixtures.records[state.selected_record].title,
        );
        ui.label(id!(detail)).set_text(context, &state.detail);
        let container = ui.widget(id!(fields));
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
        ui.redraw(context);
        Ok(())
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run()
}
