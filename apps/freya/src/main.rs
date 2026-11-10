#[cfg(all(feature = "dashboard", not(feature = "scroll")))]
mod dashboard;
#[cfg(all(feature = "scroll", not(feature = "dashboard")))]
mod scroll;

#[cfg(all(feature = "scroll", not(feature = "dashboard")))]
fn main() {
    scroll::main();
}
#[cfg(all(feature = "dashboard", not(feature = "scroll")))]
fn main() {
    dashboard::main();
}
#[cfg(any(
    all(feature = "scroll", feature = "dashboard"),
    not(any(feature = "scroll", feature = "dashboard"))
))]
compile_error!("enable exactly one of the scroll or dashboard features");
