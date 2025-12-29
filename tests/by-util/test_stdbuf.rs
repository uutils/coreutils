// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore dyld dylib setvbuf
#[cfg(target_os = "linux")]
use uutests::at_and_ucmd;
use uutests::new_ucmd;
#[cfg(not(target_os = "windows"))]
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn invalid_input() {
    new_ucmd!().arg("-/").fails_with_code(125);
}

#[cfg(not(feature = "feat_external_libstdbuf"))]
#[test]
fn test_permission() {
    new_ucmd!()
        .arg("-o1")
        .arg(".")
        .fails_with_code(126)
        .stderr_contains("Permission denied");
}

// TODO: Tests below are brittle when feat_external_libstdbuf is enabled and libstdbuf is not installed.
// Align stdbuf with GNU search order to enable deterministic testing without installation:
// 1) search for libstdbuf next to the stdbuf binary, 2) then in LIBSTDBUF_DIR, 3) then system locations.
// After implementing this, rework tests to provide a temporary symlink rather than depending on system state.

#[cfg(feature = "feat_external_libstdbuf")]
#[test]
fn test_permission_external_missing_lib() {
    // When built with external libstdbuf, running stdbuf fails early if lib is not installed
    new_ucmd!()
        .arg("-o1")
        .arg(".")
        .fails_with_code(1)
        .stderr_contains("External libstdbuf not found");
}

#[cfg(not(feature = "feat_external_libstdbuf"))]
#[test]
fn test_no_such() {
    new_ucmd!()
        .arg("-o1")
        .arg("no_such")
        .fails_with_code(127)
        .stderr_contains("No such file or directory");
}

#[cfg(feature = "feat_external_libstdbuf")]
#[test]
fn test_no_such_external_missing_lib() {
    // With external lib mode and missing installation, stdbuf fails before spawning the command
    new_ucmd!()
        .arg("-o1")
        .arg("no_such")
        .fails_with_code(1)
        .stderr_contains("External libstdbuf not found");
}

// Disabled on x86_64-unknown-linux-musl because the cross-rs Docker image for this target
// does not provide musl-compiled system utilities (like head), leading to dynamic linker errors
// when preloading musl-compiled libstdbuf.so into glibc-compiled binaries. Same thing for FreeBSD.
#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd"),
    not(all(target_arch = "x86_64", target_env = "musl"))
))]
#[test]
fn test_stdbuf_unbuffered_stdout() {
    // This is a basic smoke test
    // Note: This test only verifies that stdbuf does not crash and that output is passed through as expected
    // for simple, short-lived commands. It does not guarantee that buffering is actually modified or that
    // libstdbuf is loaded and functioning correctly.
    new_ucmd!()
        .args(&["-o0", "head"])
        .pipe_in("The quick brown fox jumps over the lazy dog.")
        .succeeds()
        .stdout_is("The quick brown fox jumps over the lazy dog.");
}

// Disabled on x86_64-unknown-linux-musl because the cross-rs Docker image for this target
// does not provide musl-compiled system utilities (like head), leading to dynamic linker errors
// when preloading musl-compiled libstdbuf.so into glibc-compiled binaries. Same thing for FreeBSD.
#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd"),
    not(all(target_arch = "x86_64", target_env = "musl"))
))]
#[test]
fn test_stdbuf_line_buffered_stdout() {
    // Note: This test only verifies that stdbuf does not crash and that output is passed through as expected
    // for simple, short-lived commands. It does not guarantee that buffering is actually modified or that
    // libstdbuf is loaded and functioning correctly.
    new_ucmd!()
        .args(&["-oL", "head"])
        .pipe_in("The quick brown fox jumps over the lazy dog.")
        .succeeds()
        .stdout_is("The quick brown fox jumps over the lazy dog.");
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_stdbuf_no_buffer_option_fails() {
    let ts = TestScenario::new(util_name!());

    ts.ucmd()
        .args(&["head"])
        .fails()
        .stderr_contains("the following required arguments were not provided:");
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_stdbuf_no_command_fails_with_125() {
    // Test that missing command fails with exit code 125 (stdbuf error)
    // This verifies proper error handling without unwrap panic
    new_ucmd!()
        .args(&["-o1"])
        .fails_with_code(125)
        .stderr_contains("the following required arguments were not provided:");
}

// Disabled on x86_64-unknown-linux-musl because the cross-rs Docker image for this target
// does not provide musl-compiled system utilities (like tail), leading to dynamic linker errors
// when preloading musl-compiled libstdbuf.so into glibc-compiled binaries. Same thing for FreeBSD.
#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd"),
    not(all(target_arch = "x86_64", target_env = "musl"))
))]
#[test]
fn test_stdbuf_trailing_var_arg() {
    new_ucmd!()
        .args(&["-i", "1024", "tail", "-1"])
        .pipe_in("The quick brown fox\njumps over the lazy dog.")
        .succeeds()
        .stdout_is("jumps over the lazy dog.");
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_stdbuf_line_buffering_stdin_fails() {
    new_ucmd!()
        .args(&["-i", "L", "head"])
        .fails()
        .usage_error("line buffering stdin is meaningless");
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_stdbuf_invalid_mode_fails() {
    let options = ["--input", "--output", "--error"];
    for option in &options {
        new_ucmd!()
            .args(&[*option, "1024R", "head"])
            .fails_with_code(125)
            .usage_error("invalid mode '1024R': Value too large for defined data type");
        new_ucmd!()
            .args(&[*option, "1Y", "head"])
            .fails_with_code(125)
            .stderr_contains("stdbuf: invalid mode '1Y': Value too large for defined data type");
        #[cfg(target_pointer_width = "32")]
        {
            new_ucmd!()
                .args(&[*option, "5GB", "head"])
                .fails_with_code(125)
                .stderr_contains(
                    "stdbuf: invalid mode '5GB': Value too large for defined data type",
                );
        }
    }
}

// macos uses DYLD_PRINT_LIBRARIES, not LD_DEBUG, so disable on macos at the moment.
// On modern Android (Bionic, API 37+), LD_DEBUG is supported and behaves similarly to glibc.
// On older Android versions (Bionic, API < 37), LD_DEBUG uses integer values instead of strings
// and is sometimes disabled. Disable test on Android for now.
// musl libc dynamic loader does not support LD_DEBUG, so disable on musl targets as well.
#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "openbsd"),
    not(target_os = "macos"),
    not(target_os = "android"),
    not(target_env = "musl")
))]
#[test]
fn test_libstdbuf_preload() {
    use std::process::Command;

    // Run a simple program with LD_DEBUG=symbols to verify that libstdbuf is loaded correctly
    // and that there are no architecture mismatches when preloading the library.
    // Note: This does not check which setvbuf implementation is used, as our libstdbuf does not override setvbuf.
    // for https://github.com/uutils/coreutils/issues/6591

    let scene = TestScenario::new(util_name!());
    let coreutils_bin = &scene.bin_path;

    // Test with our own echo (should have the correct architecture even when cross-compiled using cross-rs,
    // in which case the "system" echo will be the host architecture)
    let uutils_echo_cmd = format!(
        "LD_DEBUG=symbols {} stdbuf -oL {} echo test 2>&1",
        coreutils_bin.display(),
        coreutils_bin.display()
    );
    let uutils_output = Command::new("sh")
        .arg("-c")
        .arg(&uutils_echo_cmd)
        .output()
        .expect("Failed to run uutils echo test");

    let uutils_debug = String::from_utf8_lossy(&uutils_output.stdout);

    // Check if libstdbuf.so / libstdbuf.dylib is in the lookup path.
    // With GLIBC, the log should contain something like:
    //   "symbol=setvbuf;  lookup in file=/tmp/.tmp0mfmCg/libstdbuf.so [0]"
    // With FreeBSD dynamic loader, the log should contain something like:
    // cspell:disable-next-line
    //   "calling init function for /tmp/.tmpu11rhP/libstdbuf.so at ..."
    let libstdbuf_in_path = if cfg!(target_os = "freebsd") {
        uutils_debug
            .lines()
            .any(|line| line.contains("calling init function") && line.contains("libstdbuf"))
    } else {
        uutils_debug.contains("symbol=setvbuf")
            && uutils_debug.contains("lookup in file=")
            && uutils_debug.contains("libstdbuf")
    };

    // Check for lack of architecture mismatch error. The potential error message with GLIBC is:
    // cspell:disable-next-line
    // "ERROR: ld.so: object '/tmp/.tmpCLq8jl/libstdbuf.so' from LD_PRELOAD cannot be preloaded (cannot open shared object file): ignored."
    let arch_mismatch_line = uutils_debug
        .lines()
        .find(|line| line.contains("cannot be preloaded"));
    println!("LD_DEBUG output: {uutils_debug}");
    let no_arch_mismatch = arch_mismatch_line.is_none();

    println!("libstdbuf in lookup path: {libstdbuf_in_path}");
    println!("No architecture mismatch: {no_arch_mismatch}");
    if let Some(error_line) = arch_mismatch_line {
        println!("Architecture mismatch error: {error_line}");
    }

    assert!(
        libstdbuf_in_path,
        "libstdbuf should be in lookup path with uutils echo"
    );
    assert!(
        no_arch_mismatch,
        "uutils echo should not show architecture mismatch"
    );
}

#[cfg(target_os = "linux")]
#[cfg(not(target_env = "musl"))]
#[test]
fn test_stdbuf_non_utf8_paths() {
    use std::os::unix::ffi::OsStringExt;

    let (at, mut ucmd) = at_and_ucmd!();

    let filename = std::ffi::OsString::from_vec(vec![0xFF, 0xFE]);
    std::fs::write(at.plus(&filename), b"test content for stdbuf\n").unwrap();

    ucmd.arg("-o0")
        .arg("cat")
        .arg(&filename)
        .succeeds()
        .stdout_is("test content for stdbuf\n");
}
