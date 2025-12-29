// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) libstdbuf

use std::env;

fn main() {
    // Make sure we're building position-independent code for use with LD_PRELOAD
    println!("cargo:rustc-link-arg=-fPIC");

    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    // Ensure the library doesn't have any undefined symbols (-z flag not supported on macOS and Cygwin)
    if !target.contains("apple-darwin") && !target.contains("cygwin") {
        println!("cargo:rustc-link-arg=-z");
        println!("cargo:rustc-link-arg=defs");
    }

    println!("cargo:rerun-if-changed=src/libstdbuf.rs");
}
