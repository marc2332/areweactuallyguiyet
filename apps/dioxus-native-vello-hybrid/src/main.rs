#[cfg(all(feature = "dashboard", not(feature = "scroll")))]
mod dashboard;
#[cfg(all(feature = "scroll", not(feature = "dashboard")))]
mod scroll;
#[cfg(all(feature = "scroll", not(feature = "dashboard")))]
fn main() {
    scroll::run("Dioxus Native Vello Hybrid Scroll 2000");
}
#[cfg(all(feature = "dashboard", not(feature = "scroll")))]
fn main() {
    dashboard::run();
}
#[cfg(any(
    all(feature = "scroll", feature = "dashboard"),
    not(any(feature = "scroll", feature = "dashboard"))
))]
compile_error!("enable exactly one of the scroll or dashboard features");
