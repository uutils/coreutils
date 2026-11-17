// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) bindeps dylib libstdbuf deps liblibstdbuf

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // do not compile libstdbuf for windows target. The windows stdbuf.exe loads libstdbuf.dll compiled for the cygwin target.
    if env::var("CARGO_CFG_UNIX").is_err() {
        println!("cargo:rustc-cfg=feature=\"feat_external_libstdbuf\"");
        return;
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/libstdbuf/src/libstdbuf.rs");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_vendor = env::var("CARGO_CFG_TARGET_VENDOR").unwrap();

    let dylib_ext = if target_vendor == "apple" {
        ".dylib"
    } else if target_os == "cygwin" {
        ".dll"
    } else {
        ".so"
    };

    // Check if we're building from the repository (where src/libstdbuf exists)
    // or from crates.io (where it doesn't)
    let libstdbuf_src = Path::new("src/libstdbuf");
    if !libstdbuf_src.exists() {
        // When building from crates.io, libstdbuf is already available as a dependency
        // We can't build it here, so we'll need to handle this differently
        // For now, we'll create a dummy library file to satisfy the include_bytes! macro
        let lib_name = format!("libstdbuf{dylib_ext}");
        let dest_path = Path::new(&out_dir).join(&lib_name);

        // Create an empty file as a placeholder
        // The actual library will be provided by the dependency
        fs::write(&dest_path, []).expect("Failed to create placeholder libstdbuf");
        return;
    }

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
    cmd.current_dir(libstdbuf_src)
        .args(["build", "--target-dir", build_dir.to_str().unwrap()]);
    // Give the nested build its own build directory too. A build directory configured for the
    // outer build (build.build-dir or CARGO_BUILD_BUILD_DIR) would otherwise be shared, and the
    // nested build would wait forever for the lock which the outer build holds.
    cmd.env("CARGO_BUILD_BUILD_DIR", &build_dir);

    // Determine the actual profile from OUT_DIR since PROFILE env var only gives the base profile
    // for inherited profiles (e.g., PROFILE=release for both release and release-small).
    // OUT_DIR is .../target/[triple/]{profile}/build/{pkg-hash}/out, or with cargo's newer
    // build-dir layout .../{profile}/build/{pkg}/{hash}/out, so the depth varies: look for the
    // enclosing "build" directory and take its parent.
    let profile_dir = Path::new(&out_dir)
        .ancestors()
        .find(|p| p.file_name().is_some_and(|n| n == "build"))
        .and_then(Path::parent);
    let profile = profile_dir
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("debug");

    // Pass the profile to the nested build (for release, release-small, profiling, bench, etc.)
    if profile != "debug" {
        cmd.arg("--profile").arg(profile);
    }

    // Pass the target architecture if we're cross-compiling
    if !target.is_empty() && target != "unknown" {
        cmd.arg("--target").arg(&target);
    }

    let status = cmd.status().expect("Failed to build libstdbuf");
    assert!(status.success(), "Failed to build libstdbuf");

    // Copy the built library to OUT_DIR for include_bytes! to find
    let lib_name = if target_os == "cygwin" {
        // <https://github.com/rust-lang/rust/issues/148913>
        format!("stdbuf{dylib_ext}")
    } else {
        format!("libstdbuf{dylib_ext}")
    };
    let mut source_path = build_dir.clone();
    source_path.push(&target);
    source_path.push(profile);
    source_path.push(&lib_name);
    // align name for simplicity
    let lib_name = format!("libstdbuf{dylib_ext}");
    // This path contains hash
    let dest_path = Path::new(&out_dir).join(&lib_name);
    fs::copy(&source_path, &dest_path).expect("Failed to copy libstdbuf");
    // copy to the main target directory (e.g., target/debug) too.
    let target_dir = profile_dir.expect("Failed to get target directory");
    let easy_dest_path = target_dir.join(lib_name);
    let _ = fs::remove_file(&easy_dest_path);
    fs::copy(&source_path, &easy_dest_path)
        .expect("Failed to copy libstdbuf library to target profile root");
}
