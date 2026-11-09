use benchmark_common::{
    Scenario,
    dashboard::{Dashboard, FIELD_COUNT, SECTION_COUNT, THUMBNAILS},
};
use zgui::{
    prelude::*,
    reactive::{LocalStorage, UnsyncCallback, on_cleanup_local},
};
use zgui_image::ImageBytes;
use zgui_ui::prelude::*;

type Model = RwSignal<Dashboard, LocalStorage>;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run(Scenario::Memory, |runtime| {
        let model = RwSignal::new_local(Dashboard::default());
        let query = RwSignal::new_local(String::new());
        let fields: Vec<_> = (0..FIELD_COUNT)
            .map(|index| RwSignal::new_local(format!("Initial value {index:02}")))
            .collect();
        let viewport = NodeRef::new();
        let tick_fields = fields.clone();
        let mut reset_generation = 0;
        runtime.schedule(move |runtime, frame| {
            if reset_generation != frame.reset_generation {
                reset_generation = frame.reset_generation;
                model.update(Dashboard::reset);
                crate::ui::scroll_to(viewport, 0.0);
            }
            if frame.running {
                model.update(|state| state.advance(frame));
                crate::ui::scroll_to(viewport, frame.scroll_offset);
            }
            model.with_untracked(|state| {
                if query.get_untracked() != state.query {
                    query.set(state.query.clone());
                }
                for (index, field) in tick_fields.iter().enumerate() {
                    if field.get_untracked() != state.fields[index] {
                        field.set(state.fields[index].clone());
                    }
                }
                runtime.harness.borrow_mut().report_dashboard(state)?;
                Ok(state.applied_updates())
            })
        });
        let images: Vec<_> = THUMBNAILS
            .iter()
            .map(|bytes| ImageBytes::new(bytes.to_vec()))
            .collect();
        let thumbnail_rows: Vec<_> = images
            .chunks(2)
            .map(|images| {
                let thumbnails: Vec<_> = images
                    .iter()
                    .map(|image| {
                        let source = image.url();
                        view! { image(class = "thumbnail", src = source) }
                    })
                    .collect();
                view! { row(class = "thumbnail-row") {{thumbnails}} }
            })
            .collect();
        on_cleanup_local(move || drop(images));
        let form: Vec<_> = fields
            .into_iter()
            .enumerate()
            .map(|(index, field)| {
                view! {
                    row(class = "form-row") {
                        text(class = "field-label") {{format!("Field {index:02}")}}
                        Input(
                            value = field,
                            class = "form-input",
                            label = format!("Field {index:02}"),
                            on_change = UnsyncCallback::new(move |value: String| {
                                model.update(|state| state.edit(index, value));
                            })
                        )
                    }
                }
            })
            .collect();
        view! {
            column(class = "full") {
                row(class = "dashboard-header") {
                    text {"Dashboard"}
                    Input(
                        value = query,
                        placeholder = "Search records",
                        class = "search",
                        on_change = UnsyncCallback::new(move |value: String| {
                            model.update(|state| state.search(value));
                        })
                    )
                }
                row(class = "dashboard-main") {
                    scroll(class = "sidebar") {
                        column {
                            for section in || 0..SECTION_COUNT, key = |section: &usize| *section {
                                control(
                                    class = "navigation",
                                    tabindex = Focus::Sequential,
                                    on:click = move |_| model.update(|state| state.navigate(section))
                                ) {
                                    {if section == 0 { "All records".to_owned() } else { format!("Section {section:02}") }}
                                }
                            }
                            column(class = "thumbnails") {{thumbnail_rows}}
                        }
                    }
                    scroll(class = "records", node_ref = viewport) {
                        column {
                            for index in move || model.with(Dashboard::visible_records), key = |index: &usize| *index {
                                {record(model, index)}
                            }
                        }
                    }
                    column(class = "properties") {
                        scroll(class = "detail") {
                            column {
                                text(class = "detail-title") {
                                    {move || model.with(|state| state.fixtures.records[state.selected_record].title.clone())}
                                }
                                text(class = "detail-body") {
                                    {move || model.with(|state| state.detail.to_string())}
                                }
                            }
                        }
                        scroll(class = "form") { column {{form}} }
                    }
                }
            }
        }
    })
}

fn record(model: Model, index: usize) -> impl IntoView {
    let record = model.with_untracked(|state| state.fixtures.records[index].clone());
    view! {
        row(
            class = "record",
            class:selected = move || model.with(|state| state.selected_record == index)
        ) {
            column(
                class = "record-copy",
                on:click = move |_| model.update(|state| state.select(index))
            ) {
                text {{record.title}}
                text(class = "record-body") {{record.body}}
            }
            control(
                class = "record-toggle",
                tabindex = Focus::Sequential,
                on:click = move |_| model.update(|state| state.toggle(index))
            ) {
                {move || model.with(|state| if state.active.get(index) { "ON" } else { "OFF" })}
            }
        }
    }
}
