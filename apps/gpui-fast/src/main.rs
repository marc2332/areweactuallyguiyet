#[cfg(any(feature = "startup", feature = "memory"))]
mod text_input;
#[cfg(any(
    feature = "startup",
    feature = "scroll",
    feature = "virtual-scroll",
    feature = "switches",
    feature = "hover",
    feature = "memory"
))]
mod ui;
benchmark_common::scenario_main!();
