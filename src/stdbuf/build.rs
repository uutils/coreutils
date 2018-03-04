use std::env;
use std::fs;
use std::path::Path;

#[path = "../../mkmain.rs"]
mod mkmain;

#[cfg(target_os = "linux")]
mod platform {
    pub const DYLIB_EXT: &'static str = ".so";
}

#[cfg(target_os = "macos")]
mod platform {
    pub const DYLIB_EXT: &'static str = ".dylib";
}

fn main() {
    mkmain::main();

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("Could not find manifest dir");
    let profile = env::var("PROFILE").expect("Could not determine profile");

    let out_dir = env::var("OUT_DIR").unwrap();
    let libstdbuf = format!("{}/../../{}/{}/deps/liblibstdbuf{}", manifest_dir, env::var("CARGO_TARGET_DIR").unwrap_or("target".to_string()), profile, platform::DYLIB_EXT);
    
    fs::copy(libstdbuf, Path::new(&out_dir).join("libstdbuf.so")).unwrap();
}
