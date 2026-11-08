pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run(benchmark_common::Scenario::Scroll)
}
