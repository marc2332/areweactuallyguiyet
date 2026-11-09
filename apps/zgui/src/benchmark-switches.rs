use std::{cell::RefCell, rc::Rc};

use benchmark_common::{Scenario, Switches};
use zgui::prelude::*;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run(Scenario::Switches, |runtime| {
        let switches = Rc::new(RefCell::new(Switches::default()));
        let cells: Vec<_> = (0..benchmark_common::SWITCH_COUNT)
            .map(|_| RwSignal::new(false))
            .collect();
        let tick_switches = Rc::clone(&switches);
        let tick_cells = cells.clone();
        let mut reset_generation = 0;
        runtime.schedule(move |runtime, frame| {
            if reset_generation != frame.reset_generation {
                reset_generation = frame.reset_generation;
                *tick_switches.borrow_mut() = Switches::default();
                for cell in &tick_cells {
                    cell.set(false);
                }
            }
            if frame.running {
                tick_switches.borrow_mut().advance_changed(
                    frame.update_index,
                    runtime.harness.borrow().switch_mode(),
                    |index, value| tick_cells[index].set(value),
                );
            }
            Ok(tick_switches.borrow().applied_updates())
        });
        let rows: Vec<_> = cells
            .chunks(benchmark_common::SWITCH_COLUMNS)
            .enumerate()
            .map(|(row_index, row)| {
                let controls: Vec<_> = row
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(column, value)| {
                        let switches = Rc::clone(&switches);
                        let index = row_index * benchmark_common::SWITCH_COLUMNS + column;
                        view! {
                            control(
                                class = "switch-cell",
                                class:on = move || value.get(),
                                on:click = move |_| {
                                    switches.borrow_mut().toggle(index);
                                    value.set(!value.get_untracked());
                                }
                            ) {
                                {move || if value.get() { "ON" } else { "OFF" }}
                            }
                        }
                    })
                    .collect();
                view! { row(class = "switch-row") {{controls}} }
            })
            .collect();
        view! { column(class = "switch-grid") {{rows}} }
    })
}
