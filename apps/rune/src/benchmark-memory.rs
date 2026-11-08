use benchmark_common::{
    FrameState, Harness, Scenario,
    dashboard::{Dashboard, THUMBNAILS},
};
use rune::{El, Image, ImageCache, TextEdit, Tx};
use std::sync::Arc;

use crate::ui::{Benchmark, Error, ScenarioUi};

pub struct Memory {
    pub dashboard: Dashboard,
    pub search: TextEdit,
    pub fields: Vec<TextEdit>,
    pub thumbnails: Vec<Image>,
}

impl Memory {
    fn synchronize_editors(&mut self) {
        if self.search.text != self.dashboard.query {
            self.search.replace_text(self.dashboard.query.clone());
        }
        for (editor, value) in self.fields.iter_mut().zip(&self.dashboard.fields) {
            if editor.text != *value {
                editor.replace_text(value.clone());
            }
        }
    }
}

impl ScenarioUi for Memory {
    const SCENARIO: Scenario = Scenario::Memory;

    fn new() -> Result<Self, Error> {
        let dashboard = Dashboard::default();
        let fields = dashboard
            .fields
            .iter()
            .map(|value| TextEdit::new(value.clone()))
            .collect();
        let cache = ImageCache::new(Arc::new(|| {}));
        let thumbnails = THUMBNAILS
            .iter()
            .map(|bytes| cache.load(Arc::<[u8]>::from(*bytes)))
            .collect::<Vec<_>>();
        for image in &thumbnails {
            image.wait().map_err(|error| error.to_string())?;
        }
        Ok(Self {
            dashboard,
            search: TextEdit::new(""),
            fields,
            thumbnails,
        })
    }

    fn reset(&mut self, transaction: &mut Tx) {
        self.dashboard.reset();
        self.synchronize_editors();
        transaction.scroll_to_y("records", 0.0);
    }

    fn update(
        &mut self,
        frame: FrameState,
        _: &Harness,
        transaction: &mut Tx,
        updated: bool,
    ) -> bool {
        self.dashboard.advance(frame);
        self.synchronize_editors();
        transaction.scroll_to_y("records", frame.scroll_offset);
        updated
    }

    fn applied_updates(&self, _: FrameState) -> u64 {
        self.dashboard.applied_updates()
    }

    fn report(&self, harness: &mut Harness) -> Result<(), Error> {
        harness.report_dashboard(&self.dashboard)?;
        Ok(())
    }

    fn view(state: &Benchmark<Self>) -> El {
        crate::dashboard::view(state)
    }
}

pub fn run() -> Result<(), Error> {
    crate::ui::run::<Memory>()
}
