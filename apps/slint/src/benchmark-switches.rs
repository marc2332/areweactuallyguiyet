slint::slint! {
    import { Palette } from "std-widgets.slint";
    import "../../../assets/fonts/DejaVuSans.ttf";
    component DummyCell inherits Rectangle {
        in property <bool> active;
        callback clicked();
        width: 30px; height: 30px;
        background: active ? #2f9e44 : #868e96;
        Text { x: 0px; y: 0px; width: parent.width; height: parent.height; text: root.active ? "ON" : "OFF"; color: white; font-size: 12px; font-family: "DejaVu Sans"; horizontal-alignment: center; vertical-alignment: center; }
        TouchArea { clicked => { root.clicked(); } }
    }
    export component BenchWindow inherits Window {
        width: 1280px; height: 800px; title: "Slint benchmark";
        background: white; default-font-family: "DejaVu Sans"; default-font-size: 14px;
        init => { Palette.color-scheme = ColorScheme.light; }
        in property <[bool]> cells;
        callback cell-clicked(int);
        Rectangle {
            width: parent.width; height: parent.height;
            for value[index] in root.cells: DummyCell {
                x: mod(index, 30) * 30px; y: floor(index / 30) * 30px;
                active: value; clicked => { root.cell-clicked(index); }
            }
        }
    }
}

use benchmark_common::{Scenario, Switches};
use slint::{Model, VecModel};
use std::{cell::RefCell, rc::Rc};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let window = BenchWindow::new()?;
    let cells = Rc::new(VecModel::from(vec![false; 900]));
    window.set_cells(cells.clone().into());
    let switches = Rc::new(RefCell::new(Switches::default()));
    let click_cells = cells.clone();
    let click_switches = switches.clone();
    window.on_cell_clicked(move |index| {
        let index = index as usize;
        let mut state = click_switches.borrow_mut();
        state.toggle(index);
        click_cells.set_row_data(index, state.get(index));
    });
    crate::ui::run(
        Scenario::Switches,
        window,
        move |_window, frame, harness, reset| {
            if reset {
                *switches.borrow_mut() = Switches::default();
                for index in 0..cells.row_count() {
                    if cells.row_data(index) == Some(true) {
                        cells.set_row_data(index, false);
                    }
                }
            }
            if frame.running {
                switches.borrow_mut().advance_changed(
                    frame.update_index,
                    harness.switch_mode(),
                    |index, value| cells.set_row_data(index, value),
                );
            }
            Ok(switches.borrow().applied_updates())
        },
    )
}
