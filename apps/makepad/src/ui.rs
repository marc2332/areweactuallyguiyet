use std::sync::OnceLock;

use crate::scenario::Scene as ScenarioScene;
use benchmark_common::{FrameState, Harness, Scenario, SwitchMode};
use makepad_widgets::*;

static FAILURE: OnceLock<String> = OnceLock::new();

macro_rules! app_design {
    ($scenario:ident) => {
        live_design! {
            use crate::$scenario::SceneTemplate;
            App = {{App}} { scene_template: <SceneTemplate> {} }
        }
    };
}

#[cfg(feature = "startup")]
app_design!(selected_startup);
#[cfg(feature = "scroll")]
app_design!(selected_scroll);
#[cfg(feature = "virtual-scroll")]
app_design!(selected_virtual_scroll);
#[cfg(feature = "switches")]
app_design!(selected_switches);
#[cfg(feature = "memory")]
app_design!(selected_memory);
#[cfg(feature = "hover")]
app_design!(selected_hover);

pub(crate) trait Scene: Default {
    const SCENARIO: Scenario;

    fn mark_ready(
        &self,
        context: &mut Cx,
        ui: &WidgetRef,
        harness: &mut Harness,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let _ = (context, ui);
        harness.mark_ready("native-draw-processed-and-following-event-loop-tick-not-presented")?;
        Ok(())
    }
    #[cfg(feature = "hover")]
    fn observe_hover(
        &mut self,
        context: &mut Cx,
        ui: &WidgetRef,
        observer: benchmark_common::HoverObserver,
    ) -> Result<(), Box<dyn std::error::Error>>;

    fn initialize(
        &mut self,
        context: &mut Cx,
        ui: &WidgetRef,
    ) -> Result<(), Box<dyn std::error::Error>>;
    fn reset(
        &mut self,
        _context: &mut Cx,
        _ui: &WidgetRef,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    fn update(
        &mut self,
        _context: &mut Cx,
        _ui: &WidgetRef,
        _frame: FrameState,
        _mode: SwitchMode,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    fn applied(&self, frame: FrameState) -> u64 {
        frame.update_index
    }
    fn report(&self, _harness: &mut Harness) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    fn actions(
        &mut self,
        _context: &mut Cx,
        _ui: &WidgetRef,
        _actions: &Actions,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    fn draw(&mut self, context: &mut Cx2d, ui: &mut WidgetRef) {
        let mut scope = Scope::empty();
        while ui.draw(context, &mut scope).step().is_some() {}
    }
}

app_main!(App);

#[derive(Live, LiveHook)]
pub struct App {
    #[live]
    scene_template: Option<LivePtr>,
    #[rust]
    ui: WidgetRef,
    #[rust]
    harness: Option<Harness>,
    #[rust]
    scene: ScenarioScene,
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
        #[cfg(not(feature = "virtual-scroll"))]
        crate::items::live_design(context);
        crate::widgets::live_design(context);
        crate::scenario::live_design(context);
    }
}

impl App {
    fn initialize(&mut self, context: &mut Cx) -> Result<(), Box<dyn std::error::Error>> {
        self.harness = Some(Harness::connect(
            <crate::scenario::Scene as Scene>::SCENARIO,
        )?);
        let template = self.scene_template.ok_or("Missing scene template")?;
        self.ui = WidgetRef::new_from_ptr(context, Some(template));
        self.scene.initialize(context, &self.ui)?;
        #[cfg(feature = "hover")]
        self.scene.observe_hover(
            context,
            &self.ui,
            self.harness
                .as_ref()
                .ok_or("Missing harness")?
                .hover_observer(),
        )?;
        self.timer = context.start_interval(0.016);
        self.ui.redraw(context);
        Ok(())
    }

    fn tick(&mut self, context: &mut Cx) -> Result<(), Box<dyn std::error::Error>> {
        let Some(harness) = &mut self.harness else {
            return Ok(());
        };
        if self.drawn {
            self.scene.mark_ready(context, &self.ui, harness)?;
        }
        let frame = harness.poll()?;
        let mode = harness.switch_mode();
        if frame.should_quit {
            context.quit();
            return Ok(());
        }
        if frame.reset_generation != self.reset_generation {
            self.scene.reset(context, &self.ui)?;
            self.reset_generation = frame.reset_generation;
            self.ui.redraw(context);
        }
        if frame.running {
            self.scene.update(context, &self.ui, frame, mode)?;
        }
        if let Some(harness) = &mut self.harness {
            harness.report_applied(self.scene.applied(frame))?;
            self.scene.report(harness)?;
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
        if let Err(error) = self.scene.actions(context, &self.ui, actions) {
            self.fail(context, error);
        }
    }
}

impl AppMain for App {
    fn handle_event(&mut self, context: &mut Cx, event: &Event) {
        if let Event::Draw(draw) = event {
            let mut drawing = CxDraw::new(context, draw);
            let mut context = Cx2d::new(&mut drawing);
            self.scene.draw(&mut context, &mut self.ui);
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

pub(crate) fn run() -> Result<(), Box<dyn std::error::Error>> {
    app_main();
    if let Some(error) = FAILURE.get() {
        return Err(error.clone().into());
    }
    Ok(())
}
