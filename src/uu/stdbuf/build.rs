// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) bindeps dylib libstdbuf deps liblibstdbuf

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

#[cfg(not(any(target_vendor = "apple", target_os = "windows")))]
mod platform {
    pub const DYLIB_EXT: &str = ".so";
}

#[cfg(target_vendor = "apple")]
mod platform {
    pub const DYLIB_EXT: &str = ".dylib";
}

#[cfg(target_os = "windows")]
mod platform {
    pub const DYLIB_EXT: &str = ".dll";
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
    let mut cmd = Command::new(&cargo);
    cmd.current_dir(Path::new("src/libstdbuf")).args([
        "build",
        "--target-dir",
        build_dir.to_str().unwrap(),
    ]);

    // Pass the target architecture if we're cross-compiling
    if !target.is_empty() && target != "unknown" {
        cmd.arg("--target").arg(&target);
    }

    let status = cmd.status().expect("Failed to build libstdbuf");

    if !status.success() {
        panic!("Failed to build libstdbuf");
    }

    // Copy the built library to OUT_DIR for include_bytes! to find
    let lib_name = format!("liblibstdbuf{}", platform::DYLIB_EXT);
    let dest_path = Path::new(&out_dir).join(format!("libstdbuf{}", platform::DYLIB_EXT));

    // Check multiple possible locations for the built library
    let possible_paths = if !target.is_empty() && target != "unknown" {
        vec![
            build_dir.join("debug").join(&target).join(&lib_name),
            build_dir.join(&target).join("debug").join(&lib_name),
            build_dir
                .join(&target)
                .join("debug")
                .join("deps")
                .join(&lib_name),
            build_dir
                .join(&target)
                .join("debug")
                .join(format!("lib{}", lib_name)),
        ]
    } else {
        vec![
            build_dir.join("debug").join(&lib_name),
            build_dir.join("debug").join("deps").join(&lib_name),
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

    if !found {
        // Try to find any .so files to help with debugging
        find_libs(&build_dir, &platform::DYLIB_EXT);

        // Fail the build with helpful error message
        panic!(
            "Could not find built libstdbuf library. Searched in: {:?}.",
            possible_paths
        );
    }
}

// Helper function to recursively find library files
fn find_libs(dir: &Path, ext: &str) {
    if !dir.exists() || !dir.is_dir() {
        return;
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                find_libs(&path, ext);
            } else if let Some(extension) = path.extension() {
                if extension == ext.trim_start_matches('.') {
                    println!("Found library: {}", path.display());
                }
            }
        }
    }
}
