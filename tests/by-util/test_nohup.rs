// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore winsize Openpty openpty xpixel ypixel ptyprocess
use std::thread::sleep;
use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

// General observation: nohup.out will not be created in tests run by cargo test
// because stdin/stdout is not attached to a TTY.
// All that can be tested is the side-effects.

#[test]
fn test_nohup_exit_codes() {
    // No args: 125 default, 127 with POSIXLY_CORRECT
    new_ucmd!().fails_with_code(125);
    new_ucmd!().env("POSIXLY_CORRECT", "1").fails_with_code(127);

    // Invalid arg: 125 default, 127 with POSIXLY_CORRECT
    new_ucmd!().arg("--invalid").fails_with_code(125);
    new_ucmd!()
        .env("POSIXLY_CORRECT", "1")
        .arg("--invalid")
        .fails_with_code(127);
}

#[test]
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd",
    target_os = "openbsd",
    target_vendor = "apple"
))]
fn test_nohup_multiple_args_and_flags() {
    let (at, mut ucmd) = at_and_ucmd!();

    ucmd.args(&["touch", "-t", "1006161200", "file1", "file2"])
        .succeeds();
    sleep(std::time::Duration::from_millis(10));

    assert!(at.file_exists("file1"));
    assert!(at.file_exists("file2"));
}

#[test]
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd",
    target_os = "openbsd",
    target_vendor = "apple"
))]
fn test_nohup_with_pseudo_terminal_emulation_on_stdin_stdout_stderr_get_replaced() {
    let ts = TestScenario::new(util_name!());
    let result = ts
        .ucmd()
        .terminal_simulation(true)
        .args(&["sh", "is_a_tty.sh"])
        .succeeds();

    assert_eq!(
        String::from_utf8_lossy(result.stderr()).trim(),
        "nohup: ignoring input and appending output to 'nohup.out'"
    );

    sleep(std::time::Duration::from_millis(10));

    // this proves that nohup was exchanging the stdio file descriptors
    assert_eq!(
        std::fs::read_to_string(ts.fixtures.plus_as_string("nohup.out")).unwrap(),
        "stdin is not a tty\nstdout is not a tty\nstderr is not a tty\n"
    );
}

// Note: Testing stdin preservation is complex because nohup's behavior depends on
// whether stdin is a TTY. When stdin is a TTY, nohup redirects it to /dev/null.
// When stdin is not a TTY (e.g., a pipe), nohup preserves it.
// This behavior is already tested indirectly through other tests.

// Test that nohup creates nohup.out in current directory
#[test]
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd",
    target_os = "openbsd",
    target_vendor = "apple"
))]
fn test_nohup_creates_output_in_cwd() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    ts.ucmd()
        .terminal_simulation(true)
        .args(&["echo", "test output"])
        .succeeds()
        .stderr_contains("nohup: ignoring input and appending output to 'nohup.out'");

    sleep(std::time::Duration::from_millis(10));

    // Check that nohup.out was created in cwd
    assert!(at.file_exists("nohup.out"));
    let content = std::fs::read_to_string(at.plus_as_string("nohup.out")).unwrap();
    assert!(content.contains("test output"));
}

// Test that nohup appends to existing nohup.out
#[test]
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd",
    target_os = "openbsd",
    target_vendor = "apple"
))]
fn test_nohup_appends_to_existing_file() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    // Create existing nohup.out with content
    at.write("nohup.out", "existing content\n");

    ts.ucmd()
        .terminal_simulation(true)
        .args(&["echo", "new output"])
        .succeeds();

    sleep(std::time::Duration::from_millis(10));

    // Check that new output was appended
    let content = std::fs::read_to_string(at.plus_as_string("nohup.out")).unwrap();
    assert!(content.contains("existing content"));
    assert!(content.contains("new output"));
}

// Test that nohup falls back to $HOME/nohup.out when cwd is not writable
// Skipped on macOS as the permissions test is unreliable
#[test]
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd",
    target_os = "openbsd"
))]
fn test_nohup_fallback_to_home() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    // Skip test when running as root (permissions bypassed via CAP_DAC_OVERRIDE)
    // This is common in Docker/Podman containers but won't happen in CI
    if unsafe { libc::geteuid() } == 0 {
        println!("Skipping test when running as root (file permissions bypassed)");
        return;
    }

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    // Create a temporary HOME directory
    at.mkdir("home");
    let home_dir = at.plus_as_string("home");

    // Create a read-only directory as working directory
    at.mkdir("readonly_dir");
    let readonly_path = at.plus("readonly_dir");

    // Make readonly_dir actually read-only
    let mut perms = fs::metadata(&readonly_path).unwrap().permissions();
    perms.set_mode(0o555); // Changed from 0o444 to 0o555 (r-xr-xr-x)
    fs::set_permissions(&readonly_path, perms).unwrap();

    // Run nohup with the readonly directory as cwd and custom HOME
    let result = ts
        .ucmd()
        .env("HOME", &home_dir)
        .current_dir(&readonly_path)
        .terminal_simulation(true)
        .args(&["echo", "fallback test"])
        .run(); // Use run() instead of succeeds() since it might fail

    // Restore permissions for cleanup before any assertions
    let mut perms = fs::metadata(&readonly_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&readonly_path, perms).unwrap();

    // Should mention HOME/nohup.out in stderr if it fell back
    let stderr_str = String::from_utf8_lossy(result.stderr());
    let home_nohup = format!("{home_dir}/nohup.out");

    // Check either stderr mentions the HOME path or the file was created in HOME
    sleep(std::time::Duration::from_millis(50));
    assert!(
        stderr_str.contains(&home_nohup) || std::path::Path::new(&home_nohup).exists(),
        "nohup should fall back to HOME when cwd is not writable. stderr: {stderr_str}"
    );
}

// Test that nohup exits with 127 when command is not found
// or 126 when command exists but is not executable
#[test]
fn test_nohup_command_not_found() {
    let result = new_ucmd!()
        .arg("this-command-definitely-does-not-exist-anywhere")
        .fails();

    // Accept either 126 (cannot execute) or 127 (command not found)
    let code = result.try_exit_status().and_then(|s| s.code());
    assert!(
        code == Some(126) || code == Some(127),
        "Expected exit code 126 or 127, got: {code:?}"
    );
}

// Test stderr is redirected to stdout
#[test]
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd",
    target_os = "openbsd",
    target_vendor = "apple"
))]
fn test_nohup_stderr_to_stdout() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    // Create a script that outputs to both stdout and stderr
    at.write(
        "both_streams.sh",
        "#!/bin/bash\necho 'stdout message'\necho 'stderr message' >&2",
    );
    at.set_mode("both_streams.sh", 0o755);

    ts.ucmd()
        .terminal_simulation(true)
        .args(&["sh", "both_streams.sh"])
        .succeeds();

    sleep(std::time::Duration::from_millis(10));

    // Both stdout and stderr should be in nohup.out
    let content = std::fs::read_to_string(at.plus_as_string("nohup.out")).unwrap();
    assert!(content.contains("stdout message"));
    assert!(content.contains("stderr message"));
}

// Test nohup.out has 0600 permissions
#[test]
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd",
    target_os = "openbsd",
    target_vendor = "apple"
))]
fn test_nohup_output_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    ts.ucmd()
        .terminal_simulation(true)
        .args(&["echo", "perms"])
        .succeeds();

    sleep(std::time::Duration::from_millis(10));

    let metadata = std::fs::metadata(at.plus("nohup.out")).unwrap();
    let mode = metadata.permissions().mode();

    assert_eq!(
        mode & 0o777,
        0o600,
        "nohup.out should have 0600 permissions"
    );
}

// Test that the fallback nohup.out (in $HOME) also has 0600 permissions
#[test]
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd",
    target_os = "openbsd"
))]
fn test_nohup_fallback_output_permissions() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    // Skip if root
    if unsafe { libc::geteuid() } == 0 {
        println!("Skipping test when running as root");
        return;
    }

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    // Create a fake HOME directory
    at.mkdir("home");
    let home_dir_str = at.plus_as_string("home");

    // Create a read-only directory
    at.mkdir("readonly_dir");
    let readonly_path = at.plus("readonly_dir");

    // Make directory read-only
    let mut perms = fs::metadata(&readonly_path).unwrap().permissions();
    perms.set_mode(0o555);
    fs::set_permissions(&readonly_path, perms).unwrap();

    // Run nohup inside the read-only dir
    // This forces it to fail writing to CWD and fall back to custom HOME
    ts.ucmd()
        .env("HOME", &home_dir_str)
        .current_dir(&readonly_path)
        .terminal_simulation(true)
        .arg("true")
        .run();

    // Restore permissions so the test runner can delete the folder later!
    let mut perms = fs::metadata(&readonly_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&readonly_path, perms).unwrap();

    sleep(std::time::Duration::from_millis(50));

    // Verify the file exists in HOME and has 0600 permissions
    let home_nohup = at.plus("home/nohup.out");
    let metadata = fs::metadata(home_nohup).expect("nohup.out should have been created in HOME");
    let mode = metadata.permissions().mode();

    assert_eq!(
        mode & 0o777,
        0o600,
        "Fallback nohup.out should have 0600 permissions"
    );
}
