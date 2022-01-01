// spell-checker:ignore (ToDO) dylib libstdbuf deps liblibstdbuf

use std::env;
use std::fs;
use std::path::Path;

#[cfg(not(any(target_vendor = "apple", target_os = "windows")))]
mod platform {
    pub const DYLIB_EXT: &str = ".so";
}

#[cfg(any(target_vendor = "apple"))]
mod platform {
    pub const DYLIB_EXT: &str = ".dylib";
}

#[cfg(target_os = "windows")]
mod platform {
    pub const DYLIB_EXT: &str = ".dll";
}

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let mut target_dir = Path::new(&out_dir);

    // Depending on how this is util is built, the directory structure. This seems to work for now.
    // Here are three cases to test when changing this:
    // - cargo run
    // - cross run
    // - cargo install --git
    let mut name = target_dir.file_name().unwrap().to_string_lossy();
    while name != "target" && !name.starts_with("cargo-install") {
        target_dir = target_dir.parent().unwrap();
        name = target_dir.file_name().unwrap().to_string_lossy();
    }
    let mut libstdbuf = target_dir.to_path_buf();
    libstdbuf.push(env::var("PROFILE").unwrap());
    libstdbuf.push("deps");
    libstdbuf.push(format!("liblibstdbuf{}", platform::DYLIB_EXT));

    fs::copy(libstdbuf, Path::new(&out_dir).join("libstdbuf.so")).unwrap();
}
