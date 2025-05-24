// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) libstdbuf

use std::env;

fn main() {
    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    println!("Building libstdbuf for target triple: {}", target);

    if let Ok(target_arch) = env::var("CARGO_CFG_TARGET_ARCH") {
        println!("Building for target architecture: {}", target_arch);
    }

    // Make sure we're building position-independent code for use with LD_PRELOAD
    println!("cargo:rustc-link-arg=-fPIC");

    // Ensure the library doesn't have any undefined symbols (-z flag not supported on macOS)
    if !target.contains("apple-darwin") {
        println!("cargo:rustc-link-arg=-z");
        println!("cargo:rustc-link-arg=defs");
    }

    println!("cargo:rerun-if-changed=src/libstdbuf.rs");
}
