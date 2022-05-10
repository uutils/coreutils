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
    // - cargo publish --dry-run
    let mut name = target_dir.file_name().unwrap().to_string_lossy();
    while name != "target" && !name.starts_with("cargo-install") {
        target_dir = target_dir.parent().unwrap();
        name = target_dir.file_name().unwrap().to_string_lossy();
    }
    let mut dir = target_dir.to_path_buf();
    dir.push(env::var("PROFILE").unwrap());
    dir.push("deps");
    let mut path = None;

    // When running cargo publish, cargo appends hashes to the filenames of the compiled artifacts.
    // Therefore, it won't work to just get liblibstdbuf.so. Instead, we look for files with the
    // glob pattern "liblibstdbuf*.so" (i.e. starts with liblibstdbuf and ends with the extension).
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("liblibstdbuf") && name.ends_with(platform::DYLIB_EXT) {
            path = Some(entry.path());
        }
    }
    fs::copy(
        path.expect("liblibstdbuf was not found"),
        Path::new(&out_dir).join("libstdbuf.so"),
    )
    .unwrap();
}
