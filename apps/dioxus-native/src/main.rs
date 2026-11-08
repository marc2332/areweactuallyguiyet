benchmark_common::scenario_main!();

mod host;
#[cfg(any(feature = "scroll", feature = "virtual-scroll", feature = "memory"))]
mod scroll;
mod ui;
