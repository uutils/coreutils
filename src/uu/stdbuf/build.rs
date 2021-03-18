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
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("Could not find manifest dir");
    let profile = env::var("PROFILE").expect("Could not determine profile");

    let out_dir = env::var("OUT_DIR").unwrap();
    let libstdbuf = format!(
        "{}/../../../{}/{}/deps/liblibstdbuf{}",
        manifest_dir,
        env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string()),
        profile,
        platform::DYLIB_EXT
    );

    fs::copy(libstdbuf, Path::new(&out_dir).join("libstdbuf.so")).unwrap();
}
