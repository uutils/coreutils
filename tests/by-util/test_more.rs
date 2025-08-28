// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::io::IsTerminal;

use uutests::{at_and_ucmd, new_ucmd};

#[cfg(unix)]
#[test]
fn test_no_arg() {
    if std::io::stdout().is_terminal() {
        new_ucmd!()
            .terminal_simulation(true)
            .fails()
            .stderr_contains("more: bad usage");
    }
}

#[test]
fn test_valid_arg() {
    if std::io::stdout().is_terminal() {
        let args_list: Vec<&[&str]> = vec![
            &["-c"],
            &["--clean-print"],
            &["-p"],
            &["--print-over"],
            &["-s"],
            &["--squeeze"],
            &["-u"],
            &["--plain"],
            &["-n", "10"],
            &["--lines", "0"],
            &["--number", "0"],
            &["-F", "10"],
            &["--from-line", "0"],
            &["-P", "something"],
            &["--pattern", "-1"],
        ];
        for args in args_list {
            test_alive(args);
        }
    }
}

fn test_alive(args: &[&str]) {
    let (at, mut ucmd) = at_and_ucmd!();

    let content = "test content";
    let file = "test_file";
    at.write(file, content);

    let mut cmd = ucmd.args(args).arg(file).run_no_wait();

    // wait for more to start and display the file
    while cmd.is_alive() && !cmd.stdout_all().contains(content) {
        cmd.delay(50);
    }

    assert!(cmd.is_alive(), "Command should still be alive");

    // cleanup
    cmd.kill();
}

#[test]
fn test_invalid_arg() {
    if std::io::stdout().is_terminal() {
        new_ucmd!().arg("--invalid").fails();

        new_ucmd!().arg("--lines").arg("-10").fails();
        new_ucmd!().arg("--number").arg("-10").fails();

        new_ucmd!().arg("--from-line").arg("-10").fails();
    }
}

#[test]
fn test_file_arg() {
    // Run the test only if there's a valid terminal, else do nothing
    // Maybe we could capture the error, i.e. "Device not found" in that case
    // but I am leaving this for later
    if std::io::stdout().is_terminal() {
        // Directory as argument
        new_ucmd!()
            .arg(".")
            .succeeds()
            .stderr_contains("'.' is a directory.");

        // Single argument errors
        let (at, mut ucmd) = at_and_ucmd!();
        at.mkdir_all("folder");
        ucmd.arg("folder")
            .succeeds()
            .stderr_contains("is a directory");

        new_ucmd!()
            .arg("nonexistent_file")
            .succeeds()
            .stderr_contains("No such file or directory");

        // Multiple nonexistent files
        new_ucmd!()
            .arg("file2")
            .arg("file3")
            .succeeds()
            .stderr_contains("file2")
            .stderr_contains("file3");
    }
}

#[test]
#[cfg(target_family = "unix")]
fn test_invalid_file_perms() {
    if std::io::stdout().is_terminal() {
        use std::fs::{Permissions, set_permissions};
        use std::os::unix::fs::PermissionsExt;

        let (at, mut ucmd) = at_and_ucmd!();
        let permissions = Permissions::from_mode(0o244);
        at.make_file("invalid-perms.txt");
        set_permissions(at.plus("invalid-perms.txt"), permissions).unwrap();
        ucmd.arg("invalid-perms.txt")
            .succeeds()
            .stderr_contains("permission denied");
    }
}

#[test]
#[cfg(target_os = "linux")]
fn test_more_non_utf8_paths() {
    use std::os::unix::ffi::OsStrExt;
    if std::io::stdout().is_terminal() {
        let (at, mut ucmd) = at_and_ucmd!();
        let file_name = std::ffi::OsStr::from_bytes(b"test_\xFF\xFE.txt");
        // Create test file with normal name first
        at.write(
            &file_name.to_string_lossy(),
            "test content for non-UTF-8 file",
        );

        // Test that more can handle non-UTF-8 filenames without crashing
        ucmd.arg(file_name).succeeds();
    }
}
