use blinc_app::{prelude::*, windowed::WindowedApp};

pub fn run() -> Result<()> {
    WindowedApp::run(
        WindowConfig::new("Blinc Scroll 2000").size(1000, 700),
        |context| {
            let refresh_count = context.use_state_keyed("dashboard_refresh_count", || 0_u32);
            let enabled = context.use_state_keyed("dashboard_enabled", || true);
            let notes = context.use_state_keyed("dashboard_notes", || {
                let data = text_input_state();
                if let Ok(mut input_data) = data.lock() {
                    input_data.value = "Project notes".to_string();
                    input_data.cursor = input_data.value.len();
                }
                data
            });

            let refresh_for_click = refresh_count.clone();
            let mut rows = div().flex_col().w_full().h_fit().gap_px(8.0);
            for index in 0..1000 {
                rows = rows.child(
                    div()
                        .w_full()
                        .h(80.0)
                        .flex_shrink_0()
                        .flex_row()
                        .items_center()
                        .gap_px(16.0)
                        .p_px(8.0)
                        .bg(Color::rgb(0.94, 0.95, 0.97))
                        .child(image("assets/images/dashboard-0.png").w(64.0).h(64.0))
                        .child(
                            div()
                                .flex_col()
                                .gap(4.0)
                                .child(
                                    text(format!("Project {}", index + 1))
                                        .size(18.0)
                                        .color(Color::BLACK),
                                )
                                .child(
                                    text("Project overview and current work details")
                                        .size(13.0)
                                        .color(Color::rgb(0.35, 0.37, 0.4)),
                                ),
                        ),
                );
            }

            div()
                .w(context.width)
                .h(context.height)
                .flex_col()
                .p_px(24.0)
                .bg(Color::WHITE)
                .child(h2("Project dashboard").size(26.0).color(Color::BLACK))
                .child(div().h(15.0))
                .child(
                    div()
                        .w_full()
                        .h(40.0)
                        .flex_row()
                        .items_center()
                        .gap_px(32.0)
                        .child(
                            div()
                                .w(360.0)
                                .h(40.0)
                                .child(text_input(&notes.get()).text_size(15.0)),
                        )
                        .child(
                            div()
                                .w(160.0)
                                .h(40.0)
                                .items_center()
                                .justify_center()
                                .bg(Color::rgb(0.18, 0.38, 0.72))
                                .on_click(move |_| {
                                    refresh_for_click.update_rebuild(|count| count.saturating_add(1))
                                })
                                .child(text("Refresh").size(15.0).color(Color::WHITE)),
                        )
                        .child(div().w(16.0))
                        .child(
                            div()
                                .w(160.0)
                                .h(40.0)
                                .items_center()
                                .justify_center()
                                .bg(Color::rgb(0.88, 0.9, 0.93))
                                .on_click({
                                    let enabled = enabled.clone();
                                    move |_| enabled.update_rebuild(|current| !current)
                                })
                                .child(
                                    text(if enabled.get() {
                                        "Enabled: On"
                                    } else {
                                        "Enabled: Off"
                                    })
                                    .size(15.0)
                                    .color(Color::BLACK),
                                ),
                        ),
                )
                .child(div().h(20.0))
                .child(
                    text(format!("Status: {} refreshes", refresh_count.get()))
                        .size(14.0)
                        .color(Color::rgb(0.3, 0.32, 0.36)),
                )
                .child(div().h(33.0))
                .child(scroll().w_full().h_full().child(rows))
        },
    )
}
