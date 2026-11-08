#[cfg(any(feature = "startup", feature = "memory"))]
mod input;
#[cfg(any(
    feature = "startup",
    feature = "scroll",
    feature = "virtual-scroll",
    feature = "switches",
    feature = "memory"
))]
mod ui;
benchmark_common::scenario_main!();
