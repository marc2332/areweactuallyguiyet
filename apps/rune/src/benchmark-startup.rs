use benchmark_common::Scenario;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    crate::ui::run(Scenario::Startup)
}
