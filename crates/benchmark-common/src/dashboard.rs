use crate::{FrameState, Switches, row_body, row_title};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const RECORD_COUNT: usize = 200;
pub const FIELD_COUNT: usize = 20;
pub const SECTION_COUNT: usize = 20;
pub const THUMBNAIL_COUNT: usize = 10;
pub const DETAIL_CHARACTERS: usize = 10_000;
pub const HEADER_HEIGHT: f32 = 56.0;
pub const SIDEBAR_WIDTH: f32 = 180.0;
pub const RECORDS_WIDTH: f32 = 600.0;
pub const RECORD_HEIGHT: f32 = 64.0;

pub const THUMBNAILS: [&[u8]; THUMBNAIL_COUNT] = [
    include_bytes!("../../../assets/images/dashboard-0.png"),
    include_bytes!("../../../assets/images/dashboard-1.png"),
    include_bytes!("../../../assets/images/dashboard-2.png"),
    include_bytes!("../../../assets/images/dashboard-3.png"),
    include_bytes!("../../../assets/images/dashboard-4.png"),
    include_bytes!("../../../assets/images/dashboard-5.png"),
    include_bytes!("../../../assets/images/dashboard-6.png"),
    include_bytes!("../../../assets/images/dashboard-7.png"),
    include_bytes!("../../../assets/images/dashboard-8.png"),
    include_bytes!("../../../assets/images/dashboard-9.png"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub title: String,
    pub body: String,
    pub section: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fixtures {
    pub records: Vec<Record>,
    pub initial_fields: Vec<String>,
    detail_text: String,
}

impl Default for Fixtures {
    fn default() -> Self {
        Self {
            records: (0..RECORD_COUNT).map(|index| Record {
                title: row_title(index),
                body: format!("{} {}", row_body(index), row_body(index)).chars().take(160).collect(),
                section: index % (SECTION_COUNT - 1) + 1,
            }).collect(),
            initial_fields: (0..FIELD_COUNT).map(|index| format!("Initial value {index:02}")).collect(),
            detail_text: "A shared record detail contains wrapped text, selection state and editable properties. ".repeat(150),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dashboard {
    pub fixtures: Arc<Fixtures>,
    pub selected_record: usize,
    pub section: usize,
    pub query: String,
    pub fields: Vec<String>,
    pub active: Switches,
    pub detail: Arc<str>,
    pub scroll_offset: f32,
    applied_updates: u64,
}

impl Default for Dashboard {
    fn default() -> Self {
        Self::new(Arc::new(Fixtures::default()))
    }
}

impl Dashboard {
    pub fn new(fixtures: Arc<Fixtures>) -> Self {
        let mut dashboard = Self {
            fields: fixtures.initial_fields.clone(),
            fixtures,
            selected_record: 0,
            section: 0,
            query: String::new(),
            active: Switches::default(),
            detail: Arc::from(""),
            scroll_offset: 0.0,
            applied_updates: 0,
        };
        dashboard.refresh_detail();
        dashboard
    }

    pub fn reset(&mut self) {
        *self = Self::new(self.fixtures.clone());
    }

    pub fn visible_records(&self) -> Vec<usize> {
        let query = self.query.to_lowercase();
        self.fixtures
            .records
            .iter()
            .enumerate()
            .filter_map(|(index, record)| {
                ((self.section == 0 || record.section == self.section)
                    && (query.is_empty()
                        || record.title.to_lowercase().contains(&query)
                        || record.body.to_lowercase().contains(&query)))
                .then_some(index)
            })
            .collect()
    }

    pub fn select(&mut self, index: usize) {
        if index < self.fixtures.records.len() && index != self.selected_record {
            self.selected_record = index;
            self.refresh_detail();
        }
    }

    pub fn navigate(&mut self, section: usize) {
        self.section = section.min(SECTION_COUNT - 1);
        let visible = self.visible_records();
        if !visible.contains(&self.selected_record) {
            if let Some(index) = visible.first() {
                self.select(*index);
            }
        }
    }

    pub fn search(&mut self, query: String) {
        self.query = query;
    }

    pub fn edit(&mut self, index: usize, value: String) {
        if let Some(field) = self.fields.get_mut(index) {
            *field = value;
        }
    }

    pub fn toggle(&mut self, index: usize) {
        if index < RECORD_COUNT {
            self.active.toggle(index);
        }
    }

    pub fn advance(&mut self, frame: FrameState) {
        for update in self.applied_updates..frame.update_index {
            let step = update + 1;
            if step % 60 == 0 {
                self.query = if step % 120 == 0 {
                    String::new()
                } else {
                    "record".to_owned()
                };
            }
            if step % 90 == 0 {
                self.navigate(if step % 180 == 0 {
                    0
                } else {
                    (step / 90) as usize % (SECTION_COUNT - 1) + 1
                });
            }
            if step % 6 == 0 {
                let visible = self.visible_records();
                if !visible.is_empty() {
                    self.select(visible[((step / 6) as usize * 7) % visible.len()]);
                }
            }
            if step % 3 == 0 {
                self.toggle((step / 3) as usize % RECORD_COUNT);
            }
            if step % 10 == 0 {
                self.edit(
                    (step / 10) as usize % FIELD_COUNT,
                    format!("Edited value at update {step}"),
                );
            }
        }
        self.applied_updates = frame.update_index;
        self.scroll_offset = frame.scroll_offset;
    }

    pub fn applied_updates(&self) -> u64 {
        self.applied_updates
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            records: self.fixtures.records.len(),
            visible_records: self.visible_records().len(),
            fields: self.fields.len(),
            thumbnails: THUMBNAIL_COUNT,
            detail_characters: self.detail.chars().count(),
            selected_record: self.selected_record,
            section: self.section,
            edited_fields: self
                .fields
                .iter()
                .zip(&self.fixtures.initial_fields)
                .filter(|(current, initial)| current != initial)
                .count(),
            active_records: (0..RECORD_COUNT)
                .filter(|index| self.active.get(*index))
                .count(),
            applied_updates: self.applied_updates,
        }
    }

    fn refresh_detail(&mut self) {
        let record = &self.fixtures.records[self.selected_record];
        let mut detail = format!(
            "{}\n{}\n{}",
            record.title, record.body, self.fixtures.detail_text
        );
        detail.truncate(DETAIL_CHARACTERS);
        self.detail = Arc::from(detail);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub records: usize,
    pub visible_records: usize,
    pub fields: usize,
    pub thumbnails: usize,
    pub detail_characters: usize,
    pub selected_record: usize,
    pub section: usize,
    pub edited_fields: usize,
    pub active_records: usize,
    pub applied_updates: u64,
}

#[cfg(test)]
mod tests {
    use crate::{
        FrameState,
        dashboard::{DETAIL_CHARACTERS, Dashboard, FIELD_COUNT, RECORD_COUNT, THUMBNAILS},
    };
    use std::sync::Arc;

    #[test]
    fn scripted_dashboard_changes_real_state_and_reset_preserves_assets() {
        let mut dashboard = Dashboard::default();
        let fixtures = dashboard.fixtures.clone();
        dashboard.advance(FrameState {
            update_index: 90,
            scroll_offset: 222.0,
            ..FrameState::default()
        });
        let snapshot = dashboard.snapshot();
        assert_eq!(snapshot.records, RECORD_COUNT);
        assert_eq!(snapshot.fields, FIELD_COUNT);
        assert_eq!(snapshot.detail_characters, DETAIL_CHARACTERS);
        assert_eq!(snapshot.edited_fields, 9);
        assert_eq!(snapshot.active_records, 30);
        assert_eq!(snapshot.visible_records, 11);
        assert_eq!(snapshot.section, 2);
        assert_eq!(dashboard.query, "record");
        dashboard.reset();
        assert!(Arc::ptr_eq(&dashboard.fixtures, &fixtures));
        assert_eq!(dashboard.snapshot().edited_fields, 0);
        assert_eq!(dashboard.snapshot().active_records, 0);
        assert_eq!(dashboard.visible_records().len(), RECORD_COUNT);
    }

    #[test]
    fn search_and_fields_are_functional_and_images_are_pngs() {
        let mut dashboard = Dashboard::default();
        dashboard.search("Record 000042".to_owned());
        assert_eq!(dashboard.visible_records(), vec![42]);
        dashboard.select(42);
        assert!(dashboard.detail.starts_with("Record 000042"));
        dashboard.edit(19, "User edited value".to_owned());
        assert_eq!(dashboard.fields[19], "User edited value");
        assert!(
            THUMBNAILS
                .iter()
                .all(|bytes| bytes.starts_with(b"\x89PNG\r\n\x1a\n"))
        );
        assert_ne!(THUMBNAILS[0], THUMBNAILS[1]);
    }
}
