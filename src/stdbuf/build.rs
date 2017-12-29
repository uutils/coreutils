extern crate cc;
extern crate glob;

use std::env;
use std::process::Command;

use glob::glob;

#[path = "../../mkmain.rs"]
mod mkmain;

#[cfg(target_os = "macos")]
mod platform {
    pub const DYLIB_EXT: &'static str = "dylib";
    pub const DYLIB_FLAGS: [&'static str; 3] = ["-dynamiclib", "-undefined", "dynamic_lookup"];
}

#[cfg(target_os = "linux")]
mod platform {
    pub const DYLIB_EXT: &'static str = "so";
    pub const DYLIB_FLAGS: [&'static str; 1] = ["-shared"];
}

// FIXME: this entire thing is pretty fragile
fn main() {
    mkmain::main();

    let cc = env::var("CC").unwrap_or("gcc".to_string());

    let out_dir = env::var("OUT_DIR").unwrap();

    let entry = glob(&format!("{}/../../../deps/liblibstdbuf-*.a", out_dir)).unwrap()
                                                                            .next()
                                                                            .unwrap()
                                                                            .unwrap();

    cc::Build::new()
        .flag("-Wall")
        .flag("-Werror")
        .pic(true)
        .file("libstdbuf.c")
        .compile("libstdbuf.a");

    // XXX: we have to link manually because apparently cc-rs does not support shared libraries
    let mut link = Command::new(cc);
    for flag in platform::DYLIB_FLAGS.iter() {
        link.arg(flag);
    }
    link.arg("-o")
        .arg(format!("{}/libstdbuf.{}", out_dir, platform::DYLIB_EXT))
        .arg(format!("{}/libstdbuf.a", out_dir))
        .arg(entry);
    if !link.spawn().unwrap().wait().unwrap().success() {
        panic!("linking failed");
    }
}
