use floem::{prelude::*, window::WindowConfig};

pub fn main() {
    floem::Application::new()
        .window(
            |_| {
                let notes = RwSignal::new(String::from(""));
                let refresh_count = RwSignal::new(0_u32);
                let enabled = RwSignal::new(true);

                stack((
                    label(|| "Project dashboard").style(|style| {
                        style
                            .absolute()
                            .inset_left(24.0)
                            .inset_top(22.0)
                            .width(400.0)
                            .height(32.0)
                            .font_size(24.0)
                    }),
                    text_input(notes)
                        .placeholder("Project notes")
                        .style(|style| {
                            style
                                .absolute()
                                .inset_left(24.0)
                                .inset_top(70.0)
                                .width(360.0)
                                .height(40.0)
                                .padding_horiz(8.0)
                                .background(Color::WHITE)
                        }),
                    button("Refresh")
                        .action(move || refresh_count.update(|count| *count += 1))
                        .style(|style| {
                            style
                                .absolute()
                                .inset_left(416.0)
                                .inset_top(70.0)
                                .width(160.0)
                                .height(40.0)
                                .items_center()
                                .justify_center()
                        }),
                    h_stack((
                        toggle_button(move || enabled.get())
                            .on_toggle(move |is_enabled| enabled.set(is_enabled)),
                        label(|| "Enabled"),
                    ))
                    .style(|style| {
                        style
                            .absolute()
                            .inset_left(624.0)
                            .inset_top(70.0)
                            .width(160.0)
                            .height(40.0)
                            .items_center()
                            .justify_center()
                            .gap(8.0)
                    }),
                    label(move || {
                        format!(
                            "Status: {} refreshes, {}",
                            refresh_count.get(),
                            if enabled.get() { "enabled" } else { "disabled" }
                        )
                    })
                    .style(|style| {
                        style
                            .absolute()
                            .inset_left(24.0)
                            .inset_top(130.0)
                            .width(952.0)
                            .height(28.0)
                    }),
                    scroll(v_stack_from_iter((1..=1000).map(|number| {
                        h_stack((
                            img(|| {
                                include_bytes!("../../../assets/images/dashboard-0.png").to_vec()
                            })
                            .style(|style| style.width(64.0).height(64.0).flex_shrink(0.0)),
                            v_stack((
                                label(move || format!("Project {number}")),
                                label(|| "Project details and notes for this record"),
                            ))
                            .style(|style| style.items_start().justify_center().gap(6.0)),
                        ))
                        .style(move |style| {
                            style
                                .width(944.0)
                                .min_width(944.0)
                                .height(88.0)
                                .min_height(88.0)
                                .flex_shrink(0.0)
                                .items_center()
                                .gap(16.0)
                                .padding_horiz(12.0)
                                .background(if number % 2 == 1 {
                                    Color::rgb8(245, 245, 245)
                                } else {
                                    Color::rgb8(230, 230, 230)
                                })
                        })
                    })))
                    .style(|style| {
                        style
                            .absolute()
                            .inset_left(20.0)
                            .inset_top(180.0)
                            .width(960.0)
                            .height(500.0)
                    }),
                ))
                .style(|style| {
                    style
                        .size_full()
                        .font_size(14.0)
                        .color(Color::BLACK)
                        .background(Color::WHITE)
                })
            },
            Some(
                WindowConfig::default()
                    .size((1000.0, 700.0))
                    .title("Floem Scroll 2000")
                    .apply_default_theme(false),
            ),
        )
        .run();
}
