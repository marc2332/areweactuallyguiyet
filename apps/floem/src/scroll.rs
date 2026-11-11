#[path = "../../scroll_config.rs"]
mod scroll_config;

use floem::{prelude::*, window::WindowConfig};

pub fn main() {
    floem::Application::new()
        .window(
            |_| {
                scroll(v_stack_from_iter((1..=scroll_config::row_count()).map(
                    |number| {
                        label(move || format!("Item {number}")).style(move |style| {
                            style
                                .width(960.0)
                                .min_width(960.0)
                                .height(48.0)
                                .min_height(48.0)
                                .flex_shrink(0.0)
                                .padding(12.0)
                                .background(if number % 2 == 1 {
                                    Color::rgb8(245, 245, 245)
                                } else {
                                    Color::rgb8(230, 230, 230)
                                })
                        })
                    },
                )))
                .style(|style| style.size_full().font_size(14.0).color(Color::BLACK))
            },
            Some(
                WindowConfig::default()
                    .size((1000.0, 700.0))
                    .title("Floem Scroll")
                    .apply_default_theme(false),
            ),
        )
        .run();
}
