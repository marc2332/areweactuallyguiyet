use std::{env, fs, path::PathBuf, process::Command};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("linux")
        || env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("x86_64")
        || env::var("HOST")? != env::var("TARGET")?
    {
        return Ok(());
    }
    #[cfg(unix)]
    {
        let Ok(output) = Command::new("ldconfig").arg("-p").output() else {
            return Ok(());
        };
        if !output.status.success() {
            return Ok(());
        }
        let library = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| line.trim().split_once(" => "))
            .find_map(|(name, path)| {
                (name.starts_with("libxkbcommon-x11.so.0 ") && name.contains("x86-64"))
                    .then(|| PathBuf::from(path))
            });
        let Some(library) = library.filter(|path| path.is_file()) else {
            return Ok(());
        };
        let directory = PathBuf::from(env::var("OUT_DIR")?).join("native-libs");
        fs::create_dir_all(&directory)?;
        let alias = directory.join("libxkbcommon-x11.so");
        if fs::symlink_metadata(&alias).is_ok() {
            fs::remove_file(&alias)?;
        }
        fs::copy(&library, &alias)?;
        println!("cargo:rustc-link-search=native={}", directory.display());
    }
    Ok(())
}
