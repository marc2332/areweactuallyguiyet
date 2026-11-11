#[cfg(all(feature = "scroll", feature = "dashboard"))]
compile_error!("features `scroll` and `dashboard` cannot be enabled together");
#[cfg(not(any(feature = "scroll", feature = "dashboard")))]
compile_error!("enable exactly one of `scroll` or `dashboard`");

#[cfg(feature = "dashboard")]
mod dashboard;
#[cfg(feature = "scroll")]
mod scroll;

#[cfg(feature = "scroll")]
fn main() -> Result<(), vizia::prelude::ApplicationError> {
    scroll::main()
}
#[cfg(feature = "dashboard")]
fn main() -> Result<(), vizia::prelude::ApplicationError> {
    dashboard::main()
}
