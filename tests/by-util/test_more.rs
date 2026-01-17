// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[cfg(unix)]
use nix::unistd::{read, write};
#[cfg(unix)]
use std::fs::File;
#[cfg(unix)]
use std::fs::{Permissions, set_permissions};
#[cfg(target_os = "linux")]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use uutests::util::pty_path;
#[cfg(unix)]
use uutests::{at_and_ucmd, new_ucmd};

#[cfg(unix)]
fn run_more_with_pty(
    args: &[&str],
    file: &str,
    content: &str,
) -> (uutests::util::UChild, std::os::fd::OwnedFd, String) {
    let (path, controller, _replica) = pty_path();
    let (at, mut ucmd) = at_and_ucmd!();
    at.write(file, content);

    let mut child = ucmd
        .set_stdin(File::open(&path).unwrap())
        .set_stdout(File::create(&path).unwrap())
        .args(args)
        .arg(file)
        .run_no_wait();

    child.delay(200);
    let mut output = vec![0u8; 1024];
    let n = read(&controller, &mut output).unwrap();
    let output_str = String::from_utf8_lossy(&output[..n]).to_string();

    (child, controller, output_str)
}

#[cfg(unix)]
fn quit_more(controller: &std::os::fd::OwnedFd, mut child: uutests::util::UChild) {
    write(controller, b"q").unwrap();
    child.delay(50);
}

#[cfg(unix)]
#[test]
fn test_no_arg() {
    let (path, _controller, _replica) = pty_path();
    new_ucmd!()
        .set_stdin(File::open(&path).unwrap())
        .set_stdout(File::create(&path).unwrap())
        .fails()
        .stderr_contains("more: bad usage");
}

#[test]
#[cfg(unix)]
fn test_valid_arg() {
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

#[cfg(unix)]
fn test_alive(args: &[&str]) {
    let (at, mut ucmd) = at_and_ucmd!();
    let (path, controller, _replica) = pty_path();

    let content = "test content";
    let file = "test_file";
    at.write(file, content);

    let mut child = ucmd
        .set_stdin(File::open(&path).unwrap())
        .set_stdout(File::create(&path).unwrap())
        .args(args)
        .arg(file)
        .run_no_wait();

    // wait for more to start and display the file
    child.delay(100);

    assert!(child.is_alive(), "Command should still be alive");

    // cleanup
    write(&controller, b"q").unwrap();
    child.delay(50);
}

#[test]
#[cfg(unix)]
fn test_invalid_arg() {
    let (path, _controller, _replica) = pty_path();
    new_ucmd!()
        .set_stdin(File::open(&path).unwrap())
        .set_stdout(File::create(&path).unwrap())
        .arg("--invalid")
        .fails();

    let (path, _controller, _replica) = pty_path();
    new_ucmd!()
        .set_stdin(File::open(&path).unwrap())
        .set_stdout(File::create(&path).unwrap())
        .arg("--lines")
        .arg("-10")
        .fails();

    let (path, _controller, _replica) = pty_path();
    new_ucmd!()
        .set_stdin(File::open(&path).unwrap())
        .set_stdout(File::create(&path).unwrap())
        .arg("--from-line")
        .arg("-10")
        .fails();
}

#[test]
#[cfg(unix)]
fn test_file_arg() {
    // Directory as argument
    let (path, _controller, _replica) = pty_path();
    new_ucmd!()
        .set_stdin(File::open(&path).unwrap())
        .set_stdout(File::create(&path).unwrap())
        .arg(".")
        .succeeds()
        .stderr_contains("'.' is a directory.");

    // Single argument errors
    let (path, _controller, _replica) = pty_path();
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir_all("folder");
    ucmd.set_stdin(File::open(&path).unwrap())
        .set_stdout(File::create(&path).unwrap())
        .arg("folder")
        .succeeds()
        .stderr_contains("is a directory");

    let (path, _controller, _replica) = pty_path();
    new_ucmd!()
        .set_stdin(File::open(&path).unwrap())
        .set_stdout(File::create(&path).unwrap())
        .arg("nonexistent_file")
        .succeeds()
        .stderr_contains("No such file or directory");

    // Multiple nonexistent files
    let (path, _controller, _replica) = pty_path();
    new_ucmd!()
        .set_stdin(File::open(&path).unwrap())
        .set_stdout(File::create(&path).unwrap())
        .arg("file2")
        .arg("file3")
        .succeeds()
        .stderr_contains("file2")
        .stderr_contains("file3");
}

#[test]
#[cfg(unix)]
fn test_invalid_file_perms() {
    let (path, _controller, _replica) = pty_path();
    let (at, mut ucmd) = at_and_ucmd!();
    let permissions = Permissions::from_mode(0o244);
    at.make_file("invalid-perms.txt");
    set_permissions(at.plus("invalid-perms.txt"), permissions).unwrap();
    ucmd.set_stdin(File::open(&path).unwrap())
        .set_stdout(File::create(&path).unwrap())
        .arg("invalid-perms.txt")
        .succeeds()
        .stderr_contains("permission denied");
}

#[test]
#[cfg(target_os = "linux")]
fn test_more_non_utf8_paths() {
    let (path, _controller, _replica) = pty_path();
    let (at, mut ucmd) = at_and_ucmd!();
    let file_name = std::ffi::OsStr::from_bytes(b"test_\xFF\xFE.txt");
    // Create test file with normal name first
    at.write(
        &file_name.to_string_lossy(),
        "test content for non-UTF-8 file",
    );

    // Test that more can handle non-UTF-8 filenames without crashing
    ucmd.set_stdin(File::open(&path).unwrap())
        .set_stdout(File::create(&path).unwrap())
        .arg(file_name)
        .succeeds();
}

#[test]
#[cfg(unix)]
fn test_basic_display() {
    let (child, controller, output) = run_more_with_pty(&[], "test.txt", "line1\nline2\nline3\n");
    assert!(output.contains("line1"));
    quit_more(&controller, child);
}

#[test]
#[cfg(unix)]
fn test_squeeze_blank_lines() {
    let (child, controller, output) =
        run_more_with_pty(&["-s"], "test.txt", "line1\n\n\n\nline2\n");
    assert!(output.contains("line1"));
    quit_more(&controller, child);
}

#[test]
#[cfg(unix)]
fn test_pattern_search() {
    let (child, controller, output) = run_more_with_pty(
        &["-P", "target"],
        "test.txt",
        "foo\nbar\nbaz\ntarget\nend\n",
    );
    assert!(output.contains("target"));
    assert!(!output.contains("foo"));
    quit_more(&controller, child);
}

#[test]
#[cfg(unix)]
fn test_from_line_option() {
    let (child, controller, output) =
        run_more_with_pty(&["-F", "2"], "test.txt", "line1\nline2\nline3\nline4\n");
    assert!(output.contains("line2"));
    assert!(!output.contains("line1"));
    quit_more(&controller, child);
}
