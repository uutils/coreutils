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
    let out_dir = dbg!(env::var("OUT_DIR").unwrap());
    let libstdbuf = dbg!(format!(
        "{}/../../../deps/liblibstdbuf{}",
        out_dir,
        platform::DYLIB_EXT
    ));

    fs::copy(libstdbuf, Path::new(&out_dir).join("libstdbuf.so")).unwrap();
}
