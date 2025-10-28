use std::env;

pub fn main() {
    // Do not rebuild build script unless the script itself or the enabled features are modified
    // See <https://doc.rust-lang.org/cargo/reference/build-scripts.html#change-detection>
    println!("cargo:rerun-if-changed=build.rs");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap();

    // On musl, fts is not part of libc, but in its own library.
    if target_os == "linux" && target_env == "musl" {
        println!("cargo:rustc-link-lib=fts");
    }
}
