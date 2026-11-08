use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use benchmark_common::{
    Harness, Scenario, Switches,
    dashboard::{Dashboard, THUMBNAILS},
    row_body, row_title,
};
use floem::{
    AnyView, ViewId, kurbo::Point, prelude::*, reactive::create_effect, window::WindowConfig,
};

pub fn run(scenario: Scenario) -> Result<(), Box<dyn std::error::Error>> {
    floem::text::FONT_SYSTEM
        .lock()
        .db_mut()
        .load_font_data(include_bytes!("../../../assets/fonts/DejaVuSans.ttf").to_vec());
    let bridge = Rc::new(Bridge {
        harness: RefCell::new(Harness::connect(scenario)?),
        failure: RefCell::new(None),
        ready: Cell::new(false),
        scenario,
        switches: RefCell::new(Switches::default()),
        cells: if scenario == Scenario::Switches {
            (0..900).map(|_| create_rw_signal(false)).collect()
        } else {
            Vec::new()
        },
        dashboard: (scenario == Scenario::Memory).then(|| create_rw_signal(Dashboard::default())),
        scroll: create_rw_signal(None),
        reset: Cell::new(0),
    });
    let app_bridge = Rc::clone(&bridge);
    floem::Application::new()
        .window(
            move |_| app(app_bridge),
            Some(
                WindowConfig::default()
                    .size((1280.0, 800.0))
                    .title("Floem benchmark")
                    .apply_default_theme(false),
            ),
        )
        .run();
    if let Some(error) = bridge.failure.borrow_mut().take() {
        return Err(error.into());
    }
    Ok(())
}

struct Bridge {
    harness: RefCell<Harness>,
    failure: RefCell<Option<String>>,
    ready: Cell<bool>,
    scenario: Scenario,
    switches: RefCell<Switches>,
    cells: Vec<RwSignal<bool>>,
    dashboard: Option<RwSignal<Dashboard>>,
    scroll: RwSignal<Option<Point>>,
    reset: Cell<u64>,
}

fn schedule(bridge: Rc<Bridge>) {
    floem::action::exec_after(Duration::from_millis(16), move |_| {
        let result = (|| -> std::io::Result<bool> {
            let mut harness = bridge.harness.borrow_mut();
            let frame = harness.poll()?;
            if frame.should_quit {
                return Ok(false);
            }
            if bridge.reset.replace(frame.reset_generation) != frame.reset_generation {
                *bridge.switches.borrow_mut() = Switches::default();
                for cell in &bridge.cells {
                    cell.set(false);
                }
                if let Some(dashboard) = bridge.dashboard {
                    dashboard.update(|state| state.reset());
                }
                bridge.scroll.set(Some(Point::ZERO));
            }
            let applied = if bridge.scenario == Scenario::Switches {
                if frame.running {
                    bridge.switches.borrow_mut().advance_changed(
                        frame.update_index,
                        harness.switch_mode(),
                        |index, value| bridge.cells[index].set(value),
                    );
                }
                bridge.switches.borrow().applied_updates()
            } else if let Some(dashboard) = bridge.dashboard {
                if frame.running {
                    dashboard.update(|state| state.advance(frame));
                }
                let state = dashboard.get_untracked();
                harness.report_dashboard(&state)?;
                state.applied_updates()
            } else {
                frame.update_index
            };
            if frame.running
                && matches!(
                    bridge.scenario,
                    Scenario::Scroll | Scenario::VirtualScroll | Scenario::Memory
                )
            {
                bridge
                    .scroll
                    .set(Some(Point::new(0.0, f64::from(frame.scroll_offset))));
            }
            harness.report_applied(applied)?;
            Ok(true)
        })();
        match result {
            Ok(true) => schedule(bridge),
            Ok(false) => floem::quit_app(),
            Err(error) => {
                *bridge.failure.borrow_mut() = Some(error.to_string());
                floem::quit_app();
            }
        }
    });
}

type PaintRenderer = <floem::context::PaintCx<'static> as std::ops::Deref>::Target;

struct ReadyProbe {
    id: ViewId,
    bridge: Rc<Bridge>,
}
impl View for ReadyProbe {
    fn id(&self) -> ViewId {
        self.id
    }
    fn paint(&mut self, context: &mut floem::context::PaintCx) {
        let boundary = match &**context {
            PaintRenderer::Uninitialized { .. } => return,
            PaintRenderer::Vger(_) => "vger-first-paint-after-renderer-init-not-presented",
            PaintRenderer::TinySkia(_) => "tiny-skia-first-paint-after-renderer-init-not-presented",
        };
        if !self.bridge.ready.get() {
            self.bridge.ready.set(true);
            if let Err(error) = self.bridge.harness.borrow_mut().mark_ready(boundary) {
                *self.bridge.failure.borrow_mut() = Some(error.to_string());
                floem::quit_app();
            }
        }
    }
}

fn app(bridge: Rc<Bridge>) -> impl IntoView {
    schedule(Rc::clone(&bridge));
    let scrolling = bridge.scroll;
    let content: AnyView = match bridge.scenario {
        Scenario::Startup => {
            let query = create_rw_signal(String::new());
            let applied = create_rw_signal(0usize);
            v_stack((
                label(|| "Benchmark startup".to_owned()),
                label(|| "Prepared initial content".to_owned()),
                text_input(query)
                    .placeholder("Search records")
                    .style(|style| style.width(430.0).height(32.0)),
                button(label(move || format!("Apply ({})", applied.get())))
                    .on_click_stop(move |_| applied.update(|value| *value += 1)),
                scroll(v_stack_from_iter((0..20).map(row)))
                    .style(|style| style.width_full().min_height(0.0).flex_grow(1.0)),
            ))
            .style(|style| style.size_full().padding(16.0).gap(8.0))
            .into_any()
        }
        Scenario::Scroll => scroll(v_stack_from_iter((0..2000).map(row)))
            .scroll_to(move || scrolling.get())
            .style(|style| style.size_full())
            .into_any(),
        Scenario::VirtualScroll => {
            let records: im::Vector<usize> = (0..100_000).collect();
            scroll(
                virtual_stack(
                    VirtualDirection::Vertical,
                    VirtualItemSize::Fixed(Box::new(|| 48.0)),
                    move || records.clone(),
                    |index| *index,
                    row,
                )
                .style(|style| style.width_full().flex_col()),
            )
            .scroll_to(move || scrolling.get())
            .style(|style| style.size_full())
            .into_any()
        }
        Scenario::Switches => v_stack_from_iter((0..30).map(|row_index| {
            h_stack_from_iter((0..30).map(|column| {
                let index = row_index * 30 + column;
                let value = bridge.cells[index];
                let state = Rc::clone(&bridge);
                container(label(move || {
                    if value.get() {
                        "ON".to_owned()
                    } else {
                        "OFF".to_owned()
                    }
                }))
                .style(move |style| {
                    style
                        .width(30.0)
                        .height(30.0)
                        .flex_shrink(0.0)
                        .items_center()
                        .justify_center()
                        .font_size(12.0)
                        .color(Color::WHITE)
                        .background(if value.get() {
                            Color::rgb8(47, 158, 68)
                        } else {
                            Color::rgb8(134, 142, 150)
                        })
                })
                .on_click_stop(move |_| {
                    state.switches.borrow_mut().toggle(index);
                    value.set(!value.get_untracked());
                })
            }))
            .style(|style| style.height(30.0).gap(0.0))
        }))
        .style(|style| style.gap(0.0))
        .into_any(),
        Scenario::Memory => {
            if let Some(model) = bridge.dashboard {
                dashboard_view(model, scrolling).into_any()
            } else {
                empty().into_any()
            }
        }
    };
    stack((
        content,
        ReadyProbe {
            id: ViewId::new(),
            bridge,
        }
        .style(|style| style.absolute().width(1.0).height(1.0)),
    ))
    .style(|style| {
        style
            .size_full()
            .font_family("DejaVu Sans".to_owned())
            .font_size(14.0)
            .color(Color::BLACK)
            .background(Color::WHITE)
    })
}

fn row(index: usize) -> impl IntoView {
    let title = row_title(index);
    let body = row_body(index);
    v_stack((
        label(move || title.clone()),
        label(move || body.clone()).style(|style| style.font_size(12.0)),
    ))
    .style(|style| {
        style
            .width_full()
            .height(48.0)
            .min_height(48.0)
            .padding_horiz(8.0)
            .padding_vert(4.0)
            .flex_shrink(0.0)
    })
}

fn dashboard_view(model: RwSignal<Dashboard>, scrolling: RwSignal<Option<Point>>) -> impl IntoView {
    let query = create_rw_signal(String::new());
    let fields: Vec<_> = (0..20)
        .map(|index| create_rw_signal(format!("Initial value {index:02}")))
        .collect();
    let field_sync = fields.clone();
    create_effect(move |_| {
        let state = model.get();
        if query.get_untracked() != state.query {
            query.set(state.query);
        }
        for (index, field) in field_sync.iter().enumerate() {
            if field.get_untracked() != state.fields[index] {
                field.set(state.fields[index].clone());
            }
        }
    });
    create_effect(move |_| {
        let value = query.get();
        if model.get_untracked().query != value {
            model.update(|state| state.search(value));
        }
    });
    for (index, field) in fields.iter().copied().enumerate() {
        create_effect(move |_| {
            let value = field.get();
            if model.get_untracked().fields[index] != value {
                model.update(|state| state.edit(index, value));
            }
        });
    }
    let sidebar = v_stack((
        v_stack_from_iter((0..20).map(move |section| {
            label(move || {
                if section == 0 {
                    "All records".to_owned()
                } else {
                    format!("Section {section:02}")
                }
            })
            .style(|style| style.width_full().height(22.0).font_size(12.0))
            .on_click_stop(move |_| model.update(|state| state.navigate(section)))
        })),
        v_stack_from_iter((0..5).map(|row| {
            h_stack_from_iter((0..2).map(move |column| {
                let bytes = THUMBNAILS[row * 2 + column];
                img(move || bytes.to_vec()).style(|style| style.width(48.0).height(48.0))
            }))
            .style(|style| style.gap(8.0))
        }))
        .style(|style| style.gap(4.0)),
    ));
    let records = dyn_stack(
        move || model.with(|state| state.visible_records()),
        |index| *index,
        move |index| {
            let record = model.with_untracked(|state| state.fixtures.records[index].clone());
            let title = record.title;
            let body = record.body;
            h_stack((
                v_stack((
                    label(move || title.clone()),
                    label(move || body.clone()).style(|style| style.font_size(12.0)),
                ))
                .style(|style| style.flex_grow(1.0).min_width(0.0).height(56.0))
                .on_click_stop(move |_| model.update(|state| state.select(index))),
                container(label(move || {
                    model.with(|state| {
                        if state.active.get(index) {
                            "ON".to_owned()
                        } else {
                            "OFF".to_owned()
                        }
                    })
                }))
                .style(|style| {
                    style
                        .width(48.0)
                        .flex_shrink(0.0)
                        .height(24.0)
                        .items_center()
                        .justify_center()
                        .background(Color::rgb8(134, 142, 150))
                        .color(Color::WHITE)
                })
                .on_click_stop(move |_| model.update(|state| state.toggle(index))),
            ))
            .style(move |style| {
                style
                    .width_full()
                    .height(64.0)
                    .min_height(64.0)
                    .padding(8.0)
                    .items_center()
                    .background(if model.with(|state| state.selected_record == index) {
                        Color::rgb8(227, 237, 255)
                    } else {
                        Color::WHITE
                    })
            })
        },
    )
    .style(|style| style.flex_col().width_full());
    let detail = scroll(
        v_stack((
            label(move || {
                model.with(|state| state.fixtures.records[state.selected_record].title.clone())
            })
            .style(|style| style.font_size(18.0)),
            label(move || model.with(|state| state.detail.to_string())).style(|style| {
                style
                    .width_full()
                    .text_overflow(floem::style::TextOverflow::Wrap)
            }),
        ))
        .style(|style| style.width_full().min_width(0.0)),
    )
    .style(|style| style.width_full().height(300.0).min_height(300.0));
    let form = scroll(
        v_stack_from_iter(fields.into_iter().enumerate().map(|(index, field)| {
            h_stack((
                label(move || format!("Field {index:02}")).style(|style| style.width(80.0)),
                text_input(field).style(|style| {
                    style
                        .flex_grow(1.0)
                        .min_width(0.0)
                        .height(32.0)
                        .border(1.0)
                        .border_color(Color::rgb8(170, 170, 170))
                }),
            ))
            .style(|style| style.width_full().height(36.0).items_center().gap(8.0))
        }))
        .style(|style| style.width_full()),
    )
    .style(|style| style.width_full().min_height(0.0).flex_grow(1.0));
    v_stack((
        h_stack((
            label(|| "Dashboard".to_owned()),
            text_input(query)
                .placeholder("Search records")
                .style(|style| {
                    style
                        .flex_grow(1.0)
                        .height(32.0)
                        .border(1.0)
                        .border_color(Color::rgb8(170, 170, 170))
                }),
        ))
        .style(|style| {
            style
                .width_full()
                .height(56.0)
                .min_height(56.0)
                .padding(8.0)
                .gap(12.0)
                .items_center()
        }),
        h_stack((
            scroll(sidebar).style(|style| {
                style
                    .width(180.0)
                    .min_width(180.0)
                    .height_full()
                    .padding(8.0)
                    .background(Color::rgb8(241, 243, 245))
            }),
            scroll(records)
                .scroll_to(move || scrolling.get())
                .style(|style| style.width(600.0).min_width(600.0).height_full()),
            v_stack((detail, form)).style(|style| {
                style
                    .flex_grow(1.0)
                    .min_width(0.0)
                    .height_full()
                    .padding(8.0)
            }),
        ))
        .style(|style| style.width_full().flex_grow(1.0).min_height(0.0)),
    ))
    .style(|style| style.size_full())
}
