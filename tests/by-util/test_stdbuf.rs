// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore dyld dylib setvbuf
use uutests::new_ucmd;
#[cfg(not(target_os = "windows"))]
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn invalid_input() {
    new_ucmd!().arg("-/").fails_with_code(125);
}

#[test]
fn test_permission() {
    new_ucmd!()
        .arg("-o1")
        .arg(".")
        .fails_with_code(126)
        .stderr_contains("Permission denied");
}

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
    println!("LD_DEBUG output: {}", uutils_debug);
    let no_arch_mismatch = arch_mismatch_line.is_none();

    println!("libstdbuf in lookup path: {}", libstdbuf_in_path);
    println!("No architecture mismatch: {}", no_arch_mismatch);
    if let Some(error_line) = arch_mismatch_line {
        println!("Architecture mismatch error: {}", error_line);
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

#[cfg(all(not(target_os = "windows"), not(target_os = "openbsd")))]
#[test]
fn test_stdbuf_shell_scripts() {
    // Note: This is an actual functional test of line buffering. It verifies that stdbuf modifies buffering
    // behavior as expected by checking that output appears in the output file when new lines are added to the log
    // file. This test will fail if libstdbuf is missing, broken, or not loaded, and thus provides real coverage
    // of stdbuf's intended functionality.

    // TODO: the test works with GNU coreutils utilities, but fails with uutils coreutils utilities
    use std::fs::File;
    use std::io::Write;
    use std::process::Command;

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Create a work directory for our test
    let work_dir = "stdbuf_work_dir";
    at.mkdir(work_dir);

    // Paths for files in the work directory
    let work_dir_path = at.plus(work_dir);
    let log_file_path = work_dir_path.join("log_file");
    let output_path = work_dir_path.join("stdbuf_output");

    // Path to the coreutils binary we want to test
    let coreutils_bin = &scene.bin_path;

    // Ensure the log file exists before starting tail
    File::create(&log_file_path).expect("Failed to touch log_file");

    // Dynamically generate the shell command for the tail pipeline
    let tail_cmd = format!(
        "cd '{}' && tail -f log_file | '{}' stdbuf -oL cut -d ' ' -f 1",
        work_dir_path.display(),
        coreutils_bin.display()
    );

    // 1. Start the tail pipeline in the background, capturing its output
    let mut tail_process = Command::new("sh")
        .arg("-c")
        .arg(&tail_cmd)
        .stdout(File::create(&output_path).unwrap())
        .spawn()
        .expect("Failed to start tail pipeline");

    // Give the tail script time to start up
    std::thread::sleep(std::time::Duration::from_secs(1));

    // 2. Write the first line to the log file (simulating echo_script)
    {
        let mut log_file = File::create(&log_file_path).expect("Failed to create log_file");
        writeln!(log_file, "A 1").expect("Failed to write to log_file");
    }

    // Verify the log file was created with expected content
    let log_content = std::fs::read_to_string(&log_file_path).expect("Failed to read log file");
    assert!(log_content.contains("A 1"), "Log file should contain 'A 1'");

    // Give the tail process time to react
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Kill the process
    tail_process.kill().expect("Failed to kill tail process");
    tail_process
        .wait()
        .expect("Failed to wait for tail process");

    // Read the captured output
    let stdbuf_output = std::fs::read_to_string(&output_path).expect("Failed to read output file");

    // If stdbuf is working correctly, we should see at least one "A" in the output
    assert!(
        stdbuf_output.contains('A'),
        "stdbuf did not correctly line-buffer the output. Expected to see 'A' in the output"
    );

    // --- Test with a large buffer: output should NOT appear immediately ---

    // Clean up output file and log file
    std::fs::remove_file(&output_path).ok();
    std::fs::remove_file(&log_file_path).ok();
    File::create(&log_file_path).expect("Failed to touch log_file");

    // Use a large buffer (1MB)
    let tail_cmd_bigbuf = format!(
        "cd '{}' && tail -f log_file | '{}' stdbuf -o1048576 cut -d ' ' -f 1",
        work_dir_path.display(),
        coreutils_bin.display()
    );

    let mut tail_process_bigbuf = Command::new("sh")
        .arg("-c")
        .arg(&tail_cmd_bigbuf)
        .stdout(File::create(&output_path).unwrap())
        .spawn()
        .expect("Failed to start tail pipeline with big buffer");

    std::thread::sleep(std::time::Duration::from_secs(1));

    // Write a line to the log file
    {
        let mut log_file = File::create(&log_file_path).expect("Failed to create log_file");
        writeln!(log_file, "A 1").expect("Failed to write to log_file");
    }

    std::thread::sleep(std::time::Duration::from_secs(1));

    // Kill the process (since output would only be flushed on buffer full or process exit)
    tail_process_bigbuf
        .kill()
        .expect("Failed to kill tail process with big buffer");
    tail_process_bigbuf
        .wait()
        .expect("Failed to wait for tail process with big buffer");

    // Read the captured output
    let stdbuf_output_bigbuf =
        std::fs::read_to_string(&output_path).expect("Failed to read output file (big buffer)");

    // With a big buffer, we should NOT see 'A' in the output (unless the process is killed, which may flush)
    assert!(
        !stdbuf_output_bigbuf.contains('A'),
        "stdbuf with a large buffer should not flush output immediately. Did not expect to see 'A' in the output"
    );
}
