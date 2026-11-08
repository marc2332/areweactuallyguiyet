use std::{env, fs, io, path::PathBuf};

fn main() -> io::Result<()> {
    let output =
        PathBuf::from(env::var_os("OUT_DIR").ok_or_else(|| io::Error::other("Missing OUT_DIR"))?);
    let profile = output
        .ancestors()
        .nth(3)
        .ok_or_else(|| io::Error::other("Missing build profile directory"))?;
    let manifest = env::var("CARGO_MANIFEST_DIR").map_err(io::Error::other)?;
    fs::write(profile.join("gui-bench-makepad.path"), manifest)?;
    println!("cargo:rerun-if-changed=../../assets/fonts/DejaVuSans.ttf");
    Ok(())
}
