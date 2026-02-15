// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore cmdline dyld dylib PDEATHSIG setvbuf
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

// LD_DEBUG is not available on macOS, OpenBSD, Android, or musl
#[cfg(all(
    feature = "feat_external_libstdbuf",
    not(target_os = "windows"),
    not(target_os = "openbsd"),
    not(target_os = "macos"),
    not(target_os = "android"),
    not(target_env = "musl")
))]
#[test]
fn test_stdbuf_search_order_exe_dir_first() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    // Test that stdbuf searches for libstdbuf in its own directory first,
    // before checking LIBSTDBUF_DIR.
    let ts = TestScenario::new(util_name!());
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();

    // Determine the correct library extension for this platform
    let lib_extension = if cfg!(target_vendor = "apple") {
        "dylib"
    } else {
        "so"
    };
    let lib_name = format!("libstdbuf.{lib_extension}");

    // Look for libstdbuf in the build directory deps folder
    // During build, libstdbuf.so is in target/debug/deps/ or target/release/deps/
    // This allows running tests without requiring installation to a root-owned path
    // ts.bin_path is the path to the binary file, so we get its parent directory first
    let source_lib = ts
        .bin_path
        .parent()
        .expect("Binary should have a parent directory")
        .join("deps")
        .join(&lib_name);

    // Fail test if the library doesn't exist - it should have been built
    assert!(
        source_lib.exists(),
        "libstdbuf not found at {}. It should have been built.",
        source_lib.display()
    );

    // Copy stdbuf binary to temp directory
    // ts.bin_path is the full path to the coreutils binary
    let stdbuf_copy = temp_path.join("stdbuf");
    fs::copy(&ts.bin_path, &stdbuf_copy).unwrap();

    // Make the copied binary executable
    let mut perms = fs::metadata(&stdbuf_copy).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&stdbuf_copy, perms).unwrap();

    // Copy libstdbuf to the same directory as stdbuf
    let lib_copy = temp_path.join(&lib_name);
    fs::copy(&source_lib, &lib_copy).unwrap();

    // Run the copied stdbuf with LD_DEBUG to verify it loads the local libstdbuf
    // This proves the exe-dir search happens first, before checking LIBSTDBUF_DIR
    let output = std::process::Command::new(&stdbuf_copy)
        .env("LD_DEBUG", "libs")
        .args(["-o0", "echo", "test_output"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify the library was loaded from the temp directory (same dir as exe)
    // LD_DEBUG output will show something like:
    //   "     trying file=/tmp/.../libstdbuf.so"
    let temp_dir_str = temp_path.to_string_lossy();
    let loaded_from_exe_dir = stderr
        .lines()
        .any(|line| line.contains(&*lib_name) && line.contains(&*temp_dir_str));

    assert!(
        loaded_from_exe_dir,
        "libstdbuf should be loaded from exe directory ({}), not from LIBSTDBUF_DIR. LD_DEBUG output:\n{stderr}",
        temp_path.display()
    );

    // The command should succeed and produce the expected output
    assert!(
        output.status.success(),
        "stdbuf should succeed when libstdbuf is in the same directory. stderr: {stderr}"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "test_output",
        "stdbuf should execute echo successfully"
    );
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

#[test]
#[cfg(target_os = "linux")]
fn test_stdbuf_no_fork_regression() {
    // Regression test for issue #9066: https://github.com/uutils/coreutils/issues/9066
    // The original stdbuf implementation used fork+spawn which broke signal handling
    // and PR_SET_PDEATHSIG. This test verifies that stdbuf uses exec() instead.
    // With fork: stdbuf process would remain visible in process list
    // With exec: stdbuf process is replaced by target command (GNU compatible)

    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Duration;

    let scene = TestScenario::new(util_name!());

    // Start stdbuf with a long-running command
    let mut child = Command::new(&scene.bin_path)
        .args(["stdbuf", "-o0", "sleep", "3"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start stdbuf");

    let child_pid = child.id();

    // Poll until exec happens or timeout
    let cmdline_path = format!("/proc/{child_pid}/cmdline");
    let timeout = Duration::from_secs(2);
    let poll_interval = Duration::from_millis(10);
    let start_time = std::time::Instant::now();

    let command_name = loop {
        if start_time.elapsed() > timeout {
            child.kill().ok();
            panic!("TIMEOUT: Process {child_pid} did not respond within {timeout:?}");
        }

        if let Ok(cmdline) = std::fs::read_to_string(&cmdline_path) {
            let cmd_parts: Vec<&str> = cmdline.split('\0').collect();
            let name = cmd_parts.first().map_or("", |v| v);

            // Wait for exec to complete (process name changes from original binary to target)
            // Handle both multicall binary (coreutils) and individual utilities (stdbuf)
            if !name.contains("coreutils") && !name.contains("stdbuf") && !name.is_empty() {
                break name.to_owned();
            }
        }

        thread::sleep(poll_interval);
    };

    // The loop already waited for exec (no longer original binary), so this should always pass
    // But keep the assertion as a safety check and clear documentation
    assert!(
        !command_name.contains("coreutils") && !command_name.contains("stdbuf"),
        "REGRESSION: Process {child_pid} is still original binary (coreutils or stdbuf) - fork() used instead of exec()"
    );

    // Ensure we're running the expected target command
    assert!(
        command_name.contains("sleep"),
        "Expected 'sleep' command at PID {child_pid}, got: {command_name}"
    );

    // Cleanup
    child.kill().ok();
    child.wait().ok();
}
