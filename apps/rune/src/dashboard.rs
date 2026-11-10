use rune::{
    App, AppSpec, Color, Cx, El, Image, ImageCache, TextEdit, TextStyle, Tx, col, el, row, text,
    text_input,
};

struct State {
    notes: TextEdit,
    refresh_count: usize,
    enabled: bool,
    image_cache: Option<ImageCache>,
    images: Vec<Image>,
}

fn view(state: &State, _: &mut Cx<State>) -> El {
    let field = text_input(&state.notes, TextStyle::new(14.0, 400.0))
        .key("notes")
        .w(360.0)
        .h(40.0)
        .single_line()
        .color(Color::BLACK)
        .fill(Color {
            r: 0.94,
            g: 0.94,
            b: 0.94,
            a: 1.0,
        })
        .padding(8.0)
        .on_edit(|state: &mut State| &mut state.notes);
    let refresh_button = row()
        .size(160.0, 40.0)
        .center()
        .fill(Color {
            r: 0.88,
            g: 0.88,
            b: 0.88,
            a: 1.0,
        })
        .on_press_tx(|state: &mut State, _: &mut Tx| state.refresh_count += 1)
        .child(
            text("Refresh")
                .font(TextStyle::new(14.0, 400.0))
                .color(Color::BLACK),
        );
    let toggle = row()
        .size(160.0, 40.0)
        .center()
        .fill(if state.enabled {
            Color {
                r: 0.4,
                g: 0.75,
                b: 0.4,
                a: 1.0,
            }
        } else {
            Color {
                r: 0.7,
                g: 0.7,
                b: 0.7,
                a: 1.0,
            }
        })
        .on_press_tx(|state: &mut State, _: &mut Tx| state.enabled = !state.enabled)
        .child(
            text(if state.enabled { "Enabled" } else { "Disabled" })
                .font(TextStyle::new(14.0, 400.0))
                .color(Color::BLACK),
        );
    let toolbar = col()
        .w_full()
        .h(180.0)
        .padding(24.0)
        .child(text("Project dashboard").font(TextStyle::new(24.0, 400.0)))
        .child(row().h(17.0))
        .child(
            row()
                .h(40.0)
                .gap(32.0)
                .center_y()
                .child(field)
                .child(row().h(40.0).gap(48.0).child(refresh_button).child(toggle)),
        )
        .child(
            text(format!("Refresh count: {}", state.refresh_count))
                .font(TextStyle::new(14.0, 400.0)),
        );
    let records = col()
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll(rune::ScrollSpec {
            stick_bottom: false,
        })
        .children((0..100).map(|index| {
            let mut record = row().w_full().h(88.0).padding(12.0).center_y().gap(16.0);
            if let Some(image) = state.images.get(index % state.images.len().max(1)) {
                record = record.child(el().size(64.0, 64.0).pixels(image.pixels()));
            }
            record.child(
                col()
                    .gap(4.0)
                    .child(text(format!("Project {}", index + 1)).font(TextStyle::new(14.0, 400.0)))
                    .child(
                        text("A project record with descriptive dashboard information.")
                            .font(TextStyle::new(12.0, 400.0)),
                    ),
            )
        }));
    col().w_full().h_full().child(toolbar).child(records)
}

fn setup(state: &mut State, waker: rune::Waker) {
    let cache = ImageCache::new(waker);
    state.images = (0..10)
        .map(|index| {
            cache.file(
                format!(
                    "{}/../../assets/images/dashboard-{index}.png",
                    env!("CARGO_MANIFEST_DIR")
                ),
                64.0,
                1.0,
            )
        })
        .collect();
    state.image_cache = Some(cache);
}

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new(AppSpec {
        setup: Some(setup),
        ..AppSpec::new(
            "Rune Scroll 2000",
            (1000.0, 700.0),
            || State {
                notes: TextEdit::new("Project notes"),
                refresh_count: 0,
                enabled: false,
                image_cache: None,
                images: Vec::new(),
            },
            view,
        )
    })
    .try_run()?;
    Ok(())
}
