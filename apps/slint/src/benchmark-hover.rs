use benchmark_common::{Harness, Scenario, hover};
use slint::{ModelRc, VecModel};

slint::slint! {
    export component BenchWindow inherits Window {
        width: 1280px; height: 800px; title: "Slint hover benchmark";
        background: white;
        in property <[int]> cells;
        in property <int> columns;
        in property <length> cell-size;
        in property <color> normal-color;
        in property <color> hovered-color;
        callback cell-hovered(int, bool);
        for cell in root.cells: Rectangle {
            x: mod(cell, root.columns) * root.cell-size;
            y: floor(cell / root.columns) * root.cell-size;
            width: root.cell-size; height: root.cell-size;
            background: touch.has-hover ? root.hovered-color : root.normal-color;
            touch := TouchArea {
                changed has-hover => { root.cell-hovered(cell, self.has-hover); }
            }
        }
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    slint::BackendSelector::new()
        .with_winit_window_attributes_hook(|attributes| {
            attributes
                .with_fullscreen(Some(
                    slint::winit_030::winit::window::Fullscreen::Borderless(None),
                ))
                .with_decorations(false)
        })
        .select()?;
    let harness = Harness::connect(Scenario::Hover)?;
    let observer = harness.hover_observer();
    let window = BenchWindow::new()?;
    window.set_cells(ModelRc::new(VecModel::from(
        (0..hover::COUNT as i32).collect::<Vec<_>>(),
    )));
    window.set_columns(hover::COLUMNS as i32);
    window.set_cell_size(hover::CELL_SIZE);
    let (red, green, blue) = hover::NORMAL_COLOR;
    window.set_normal_color(slint::Color::from_rgb_u8(red, green, blue));
    let (red, green, blue) = hover::HOVERED_COLOR;
    window.set_hovered_color(slint::Color::from_rgb_u8(red, green, blue));
    window.on_cell_hovered(move |index, inside| {
        if inside {
            observer.enter(index as usize);
        } else {
            observer.leave(index as usize);
        }
    });
    crate::ui::run_with_harness(window, harness, |_, frame, _, _| Ok(frame.update_index))
}
