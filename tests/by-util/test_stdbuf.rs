// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore cmdline dyld dylib PDEATHSIG setvbuf ppid

#[cfg(target_os = "linux")]
use uutests::at_and_ucmd;
#[cfg(unix)]
use uutests::{new_ucmd, util::TestScenario, util_name};

#[test]
#[cfg(unix)]
fn invalid_input() {
    new_ucmd!().arg("-/").fails_with_code(125);
}

// linux-gated to match the `at_and_ucmd` import above; the check itself is not
// platform-specific.
#[cfg(all(target_os = "linux", not(feature = "feat_external_libstdbuf")))]
#[test]
fn test_tmpdir_with_colon_is_rejected() {
    // The preload variable is a colon-separated list with no escaping, so a
    // libstdbuf path containing ':' would be split by the loader and its leading
    // component loaded as a library of its own. With $TMPDIR under an attacker's
    // control that component is attacker-chosen, so refuse instead.
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("evil.so:x");

    ucmd.env("TMPDIR", at.plus("evil.so:x"))
        .arg("-o0")
        .arg("true")
        .fails_with_code(125)
        .stderr_contains("contains ':'");
}

#[cfg(all(unix, not(feature = "feat_external_libstdbuf")))]
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
    unix,
    not(target_vendor = "apple"),
    not(target_os = "openbsd"),
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

#[cfg(all(unix, not(feature = "feat_external_libstdbuf")))]
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
    unix,
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
    unix,
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

#[test]
#[cfg(unix)]
fn test_stdbuf_no_buffer_option_fails() {
    let ts = TestScenario::new(util_name!());

    ts.ucmd()
        .args(&["head"])
        .fails()
        .stderr_contains("the following required arguments were not provided:");
}

#[test]
#[cfg(unix)]
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
    unix,
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

#[test]
#[cfg(unix)]
fn test_stdbuf_line_buffering_stdin_fails() {
    new_ucmd!()
        .args(&["-i", "L", "head"])
        .fails()
        .usage_error("line buffering stdin is meaningless");
}

#[test]
#[cfg(unix)]
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
    unix,
    not(target_vendor = "apple"),
    not(target_os = "openbsd"),
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

// stdbuf uses spawn()+wait() (not exec()) so that the TempDir holding
// libstdbuf.so is cleaned up after the child exits.  The stdbuf process
// itself therefore stays in the process table as a thin waiter; the child
// immediately execs the requested command.
// See: https://github.com/uutils/coreutils/issues/13939
#[test]
#[cfg(target_os = "linux")]
fn test_stdbuf_child_execs_command() {
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Duration;

    let scene = TestScenario::new(util_name!());

    // Start stdbuf with a long-running command
    let mut parent = Command::new(&scene.bin_path)
        .args(["stdbuf", "-o0", "sleep", "5"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start stdbuf");

    let parent_pid = parent.id();

    // Poll until the child process appears or timeout
    let timeout = Duration::from_secs(3);
    let poll_interval = Duration::from_millis(10);
    let start_time = std::time::Instant::now();

    let child_comm = loop {
        if start_time.elapsed() > timeout {
            parent.kill().ok();
            panic!("TIMEOUT: child of {parent_pid} did not appear within {timeout:?}");
        }

        // Find children of our stdbuf process
        let found = std::fs::read_dir("/proc").ok().and_then(|entries| {
            entries.flatten().find_map(|entry| {
                let pid: u32 = entry.file_name().to_string_lossy().parse().ok()?;
                let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
                let mut parts = stat.splitn(5, ' ');
                let _pid = parts.next();
                let comm = parts
                    .next()
                    .unwrap_or("")
                    .trim_matches(|c| c == '(' || c == ')')
                    .to_string();
                let _state = parts.next();
                let ppid: u32 = parts.next().unwrap_or("0").parse().unwrap_or(0);
                if ppid == parent_pid && comm.contains("sleep") {
                    Some(comm)
                } else {
                    None
                }
            })
        });
        if found.is_some() {
            break found;
        }
        thread::sleep(poll_interval);

        if start_time.elapsed() > timeout {
            break None;
        }
    };

    parent.kill().ok();
    parent.wait().ok();

    assert!(
        child_comm.is_some(),
        "stdbuf should have spawned a child running 'sleep' (pid={parent_pid})"
    );
    assert!(
        child_comm.as_deref().unwrap_or("").contains("sleep"),
        "Expected child to be 'sleep', got: {child_comm:?}"
    );
}

/// Verify that stdbuf does not leak temporary directories.
/// Each invocation should clean up its own tmpdir.
/// Regression test for https://github.com/uutils/coreutils/issues/13939
#[test]
#[cfg(all(target_os = "linux", not(feature = "feat_external_libstdbuf")))]
fn test_stdbuf_no_tmpdir_leak() {
    use std::process::Command;

    // Use a dedicated TMPDIR so we only count stdbuf-created dirs,
    // not dirs created by the test harness itself.
    let dedicated_tmpdir = tempfile::tempdir().unwrap();
    let scene = TestScenario::new(util_name!());

    for _ in 0..5 {
        Command::new(&scene.bin_path)
            .args(["stdbuf", "-oL", "true"])
            .env("TMPDIR", dedicated_tmpdir.path())
            .status()
            .expect("failed to run stdbuf");
    }

    let leaked: Vec<_> = std::fs::read_dir(dedicated_tmpdir.path())
        .unwrap()
        .flatten()
        .filter(|e| e.metadata().is_ok_and(|m| m.is_dir()))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    assert!(
        leaked.is_empty(),
        "stdbuf leaked {n} temporary director{pl}: {leaked:?}",
        n = leaked.len(),
        pl = if leaked.len() == 1 { "y" } else { "ies" },
    );
}

/// Verify that the temporary directory created for libstdbuf.so is private (0700)
/// and the embedded library file is owner read/write only (0600).
/// Regression test for https://github.com/uutils/coreutils/issues/13939
#[test]
#[cfg(all(target_os = "linux", not(feature = "feat_external_libstdbuf")))]
fn test_stdbuf_tmpdir_is_private() {
    use std::os::unix::fs::PermissionsExt;

    // Use a dedicated TMPDIR so we only observe stdbuf's directory.
    let dedicated_tmpdir = tempfile::tempdir().unwrap();
    let scene = TestScenario::new(util_name!());

    // Use a long-running command so the tmpdir exists while we inspect it.
    let mut child = std::process::Command::new(&scene.bin_path)
        .args(["stdbuf", "-o0", "sleep", "5"])
        .env("TMPDIR", dedicated_tmpdir.path())
        .spawn()
        .expect("failed to spawn stdbuf");

    // Give it time to create the tmpdir
    std::thread::sleep(std::time::Duration::from_millis(300));

    let stdbuf_dirs: Vec<_> = std::fs::read_dir(dedicated_tmpdir.path())
        .unwrap()
        .flatten()
        .filter(|e| e.metadata().is_ok_and(|m| m.is_dir()))
        .map(|e| e.path())
        .collect();

    child.kill().ok();
    child.wait().ok();

    assert!(
        !stdbuf_dirs.is_empty(),
        "expected stdbuf to create a temporary directory in {dedicated_tmpdir:?}"
    );

    for dir in &stdbuf_dirs {
        let mode = std::fs::metadata(dir)
            .expect("metadata")
            .permissions()
            .mode();
        assert_eq!(
            mode & 0o777,
            0o700,
            "tmpdir {dir:?} has unsafe permissions {mode:#o}, expected 0o700"
        );

        for entry in std::fs::read_dir(dir).expect("read_dir").flatten() {
            if !entry.file_type().is_ok_and(|t| t.is_file()) {
                continue;
            }
            let file_mode = entry.metadata().expect("metadata").permissions().mode() & 0o777;
            assert_eq!(
                file_mode,
                0o600,
                "libstdbuf at {} has unsafe permissions {file_mode:#o}, expected 0o600",
                entry.path().display(),
            );
        }
    }
}

/// stdbuf waits on the command instead of exec'ing it, so it has to translate
/// the child's fate back into its own exit status: a command killed by a
/// signal must still be reported as `128 + signal`, not as a clean 0.
#[test]
#[cfg(unix)]
fn test_stdbuf_reports_signalled_command() {
    // SIGQUIT (3) rather than the usual SIGTERM, so the expected 131 cannot be
    // confused with a status the shell would produce on its own.
    new_ucmd!()
        .args(&["-o0", "sh", "-c", "kill -QUIT $$"])
        .fails_with_code(131);
}

#[cfg(unix)]
#[cfg(all(feature = "feat_diagnostics", not(wasi_runner)))]
mod diagnostics {
    use super::*;

    #[test]
    fn test_snippet_points_at_the_unknown_unit() {
        let result = new_ucmd!()
            .terminal_sim_stderr()
            .args(&["-o", "6pq", "head"])
            .fails_with_code(125);

        // The number parsed; only the unit did not.
        let stderr = result.stderr_as_displayed();
        assert!(
            stderr.starts_with(
                "\
stdbuf: invalid mode '6pq'
   ╭─[ stdbuf:1:12 ]
   │
 1 │ stdbuf -o 6pq head
   │            ─┬
   │             ╰── not a known unit
   │
   │ Help: a size is a number and an optional unit: K, M, G and so on for 1024, KB, MB, GB for 1000
───╯"
            ),
            "{stderr}"
        );
        // The caret replaces the message, not the usage hint: a pipe and a
        // terminal must not disagree on whether one was printed.
        assert!(
            stderr.ends_with("stdbuf --help' for more information."),
            "{stderr}"
        );
    }

    #[test]
    fn test_snippet_points_inside_a_long_option_value() {
        let result = new_ucmd!()
            .terminal_sim_stderr()
            .args(&["--error=pq", "head"])
            .fails_with_code(125);
        let stderr = result.stderr_as_displayed();

        // Nothing usable was read, so the whole value is underlined.
        assert!(stderr.contains("stdbuf:1:16"), "{stderr}");
        assert!(!stderr.contains("not a known unit"), "{stderr}");
    }

    #[test]
    fn test_plain_message_when_stderr_is_a_pipe() {
        new_ucmd!()
            .args(&["-o", "6pq", "head"])
            .fails_with_code(125)
            .usage_error("invalid mode '6pq'");
    }
}
