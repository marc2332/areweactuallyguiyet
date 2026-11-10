#[cfg(all(feature = "scroll", feature = "dashboard"))]
compile_error!("enable either the scroll or dashboard feature, not both");
#[cfg(not(any(feature = "scroll", feature = "dashboard")))]
compile_error!("enable either the scroll or dashboard feature");

#[cfg(feature = "dashboard")]
mod dashboard;
#[cfg(feature = "scroll")]
mod scroll;

#[cfg(feature = "scroll")]
fn main() -> blinc_app::prelude::Result<()> {
    scroll::run()
}
#[cfg(all(feature = "dashboard", not(feature = "scroll")))]
fn main() -> blinc_app::prelude::Result<()> {
    dashboard::run()
}
