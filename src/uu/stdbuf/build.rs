// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) bindeps dylib libstdbuf deps liblibstdbuf

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
mod platform {
    pub const DYLIB_EXT: &str = ".so";
}

#[cfg(target_vendor = "apple")]
mod platform {
    pub const DYLIB_EXT: &str = ".dylib";
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/libstdbuf/src/libstdbuf.rs");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());

    // Create a separate build directory for libstdbuf to avoid conflicts
    let build_dir = Path::new(&out_dir).join("libstdbuf-build");
    fs::create_dir_all(&build_dir).expect("Failed to create build directory");

    // Get the cargo executable
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    // This manual cargo call ensures that libstdbuf is built before stdbuf.rs is compiled, which is necessary
    // for include_bytes!(..."/libstdbuf.so") to work.
    // In the future, "bindeps" should be used to simplify the code and avoid the manual cargo call,
    // however this is available only in cargo nightly at the moment.
    // See the tracking issue: https://github.com/rust-lang/cargo/issues/9096
    let mut cmd = Command::new(&cargo);
    cmd.env_clear().envs(env::vars());
    cmd.current_dir(Path::new("src/libstdbuf")).args([
        "build",
        "--target-dir",
        build_dir.to_str().unwrap(),
    ]);

    // Get the current profile
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());

    // Pass the release flag if we're in release mode
    if profile == "release" || profile == "bench" {
        cmd.arg("--release");
    }

    // Pass the target architecture if we're cross-compiling
    if !target.is_empty() && target != "unknown" {
        cmd.arg("--target").arg(&target);
    }

    let status = cmd.status().expect("Failed to build libstdbuf");
    assert!(status.success(), "Failed to build libstdbuf");

    // Copy the built library to OUT_DIR for include_bytes! to find
    let lib_name = format!("libstdbuf{}", platform::DYLIB_EXT);
    let dest_path = Path::new(&out_dir).join(format!("libstdbuf{}", platform::DYLIB_EXT));

    // Check multiple possible locations for the built library
    let possible_paths = if !target.is_empty() && target != "unknown" {
        vec![
            build_dir.join(&target).join(&profile).join(&lib_name),
            build_dir
                .join(&target)
                .join(&profile)
                .join("deps")
                .join(&lib_name),
        ]
    } else {
        vec![
            build_dir.join(&profile).join(&lib_name),
            build_dir.join(&profile).join("deps").join(&lib_name),
        ]
    };

    // Try to find the library in any of the possible locations
    let mut found = false;
    for source_path in &possible_paths {
        if source_path.exists() {
            fs::copy(source_path, &dest_path).expect("Failed to copy libstdbuf library");
            found = true;
            break;
        }
    }

    assert!(
        found,
        "Could not find built libstdbuf library. Searched in: {:?}.",
        possible_paths
    );
}
