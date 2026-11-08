use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use benchmark_common::{Scenario, Switches};
use floem::prelude::*;

struct State {
    switches: RefCell<Switches>,
    cells: Vec<RwSignal<bool>>,
    reset: Cell<u64>,
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let state = Rc::new(State {
        switches: RefCell::new(Switches::default()),
        cells: (0..900).map(|_| create_rw_signal(false)).collect(),
        reset: Cell::new(0),
    });
    let ui_state = Rc::clone(&state);
    crate::ui::run(
        Scenario::Switches,
        move |_| {
            v_stack_from_iter((0..30).map(|row_index| {
                let state = Rc::clone(&ui_state);
                h_stack_from_iter((0..30).map(move |column| {
                    let index = row_index * 30 + column;
                    let value = state.cells[index];
                    let click_state = Rc::clone(&state);
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
                        click_state.switches.borrow_mut().toggle(index);
                        value.set(!value.get_untracked());
                    })
                }))
                .style(|style| style.height(30.0).gap(0.0))
            }))
            .style(|style| style.gap(0.0))
            .into_any()
        },
        move |runtime, frame| {
            if state.reset.replace(frame.reset_generation) != frame.reset_generation {
                *state.switches.borrow_mut() = Switches::default();
                for cell in &state.cells {
                    cell.set(false);
                }
            }
            if frame.running {
                state.switches.borrow_mut().advance_changed(
                    frame.update_index,
                    runtime.harness.borrow().switch_mode(),
                    |index, value| state.cells[index].set(value),
                );
            }
            Ok(state.switches.borrow().applied_updates())
        },
    )
}
