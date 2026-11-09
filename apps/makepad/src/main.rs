pub use makepad_widgets;

benchmark_common::scenario_main!();

#[cfg(not(feature = "virtual-scroll"))]
mod items;
mod ui;
mod widgets;

#[cfg(feature = "memory")]
use selected_memory as scenario;
#[cfg(feature = "scroll")]
use selected_scroll as scenario;
#[cfg(feature = "startup")]
use selected_startup as scenario;
#[cfg(feature = "switches")]
use selected_switches as scenario;
#[cfg(feature = "virtual-scroll")]
use selected_virtual_scroll as scenario;

#[cfg(feature = "hover")]
use selected_hover as scenario;
