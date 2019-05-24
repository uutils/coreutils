use std::env;
use std::fs;
use std::path::Path;

#[path = "../../mkmain.rs"]
mod mkmain;

#[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "windows")))]
mod platform {
    pub const DYLIB_EXT: &str = ".so";
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod platform {
    pub const DYLIB_EXT: &str = ".dylib";
}

#[cfg(target_os = "windows")]
mod platform {
    pub const DYLIB_EXT: &str = ".dll";
}

fn main() {
    mkmain::main();

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("Could not find manifest dir");
    let profile = env::var("PROFILE").expect("Could not determine profile");

    let out_dir = env::var("OUT_DIR").unwrap();
    let libstdbuf = format!(
        "{}/../../{}/{}/deps/liblibstdbuf{}",
        manifest_dir,
        env::var("CARGO_TARGET_DIR").unwrap_or("target".to_string()),
        profile,
        platform::DYLIB_EXT
    );

    fs::copy(libstdbuf, Path::new(&out_dir).join("libstdbuf.so")).unwrap();
}
