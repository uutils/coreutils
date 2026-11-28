// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#![cfg(all(unix, not(target_os = "redox")))]

#[cfg(unix)]
use std::fs::File;
#[cfg(unix)]
use std::fs::{Permissions, set_permissions};
#[cfg(unix)]
use std::io::{Read as _, Write as _};
#[cfg(target_os = "linux")]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use uutests::util::pty_path;
#[cfg(unix)]
use uutests::{at_and_ucmd, new_ucmd};

/// Kills `more` when dropped, so a test that panics does not leave it running.
/// A `more` left behind spins once its terminal hangs up.
#[cfg(unix)]
struct MoreChild(uutests::util::UChild);

#[cfg(unix)]
impl std::ops::Deref for MoreChild {
    type Target = uutests::util::UChild;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(unix)]
impl std::ops::DerefMut for MoreChild {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(unix)]
impl Drop for MoreChild {
    fn drop(&mut self) {
        if self.0.is_alive() {
            let _ = self.0.try_kill();
        }
    }
}

#[cfg(unix)]
fn run_more_with_pty(args: &[&str], file: &str, content: &str) -> (MoreChild, File, String) {
    let (path, mut controller, _replica) = pty_path();
    let (at, mut ucmd) = at_and_ucmd!();
    at.write(file, content);

    let mut child = MoreChild(
        ucmd
            // POSIXLY_CORRECT disables the default exit_on_eof so the pager stays alive
            .env("POSIXLY_CORRECT", "1")
            .set_stdin(File::open(&path).unwrap())
            .set_stdout(File::create(&path).unwrap())
            .args(args)
            .arg(file)
            .run_no_wait(),
    );

    child.delay(200);
    let mut output = vec![0u8; 1024];
    let n = controller.read(&mut output).unwrap();
    let output_str = String::from_utf8_lossy(&output[..n]).to_string();

    (child, controller, output_str)
}

#[cfg(unix)]
fn quit_more(controller: &mut File, mut child: MoreChild) {
    controller.write_all(b"q").unwrap();
    child.delay(50);
    // Not all options quit on `q` here, and a `more` left behind spins once
    // its terminal hangs up.
    child.kill();
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
        &["-e"],
        &["--exit-on-eof"],
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
        &["-f"],
        &["--logical"],
        &["-l"],
        &["--no-pause"],
    ];
    for args in args_list {
        test_alive(args);
    }
}

#[cfg(unix)]
fn test_alive(args: &[&str]) {
    let (at, mut ucmd) = at_and_ucmd!();
    let (path, mut controller, _replica) = pty_path();

    // Enough content to avoid hitting EOF on the initial page so the pager stays alive
    let content = (0..50)
        .map(|i| i.to_string())
        .collect::<Vec<String>>()
        .join("\n");
    let file = "test_file";
    at.write(file, &content);

    let mut child = MoreChild(
        ucmd
            // POSIXLY_CORRECT disables the default exit_on_eof so the pager stays alive
            .env("POSIXLY_CORRECT", "1")
            .set_stdin(File::open(&path).unwrap())
            .set_stdout(File::create(&path).unwrap())
            .args(args)
            .arg(file)
            .run_no_wait(),
    );

    // wait for more to start and display the file
    child.delay(100);

    assert!(child.is_alive(), "Command should still be alive");

    quit_more(&mut controller, child);
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
        .stderr_contains("'.': Is a directory");

    // Single argument errors
    let (path, _controller, _replica) = pty_path();
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir_all("folder");
    ucmd.set_stdin(File::open(&path).unwrap())
        .set_stdout(File::create(&path).unwrap())
        .arg("folder")
        .succeeds()
        .stderr_contains("Is a directory");

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
        .stderr_contains("cannot open 'invalid-perms.txt': Permission denied");
}

#[test]
#[cfg(target_os = "linux")]
fn test_more_non_utf8_paths() {
    let (path, _controller, _replica) = pty_path();
    let (at, mut ucmd) = at_and_ucmd!();
    let file_name = std::ffi::OsStr::from_bytes(b"test_\xFF\xFE.txt");
    // Create test file with normal name first
    at.write(
        file_name.to_string_lossy().to_string(),
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
    let (child, mut controller, output) =
        run_more_with_pty(&[], "test.txt", "line1\nline2\nline3\n");
    assert!(output.contains("line1"));
    quit_more(&mut controller, child);
}

#[test]
#[cfg(unix)]
fn test_space_at_eof_does_not_exit_without_exit_on_eof() {
    let (mut child, mut controller, _output) = run_more_with_pty(&[], "test.txt", "line1\n");
    assert!(
        child.is_alive(),
        "more should be alive at EOF when exit_on_eof is disabled"
    );
    controller.write_all(b" ").unwrap();
    child.delay(100);
    assert!(
        child.is_alive(),
        "more should remain alive after space at EOF when exit_on_eof is disabled"
    );
    quit_more(&mut controller, child);
}

/// Spawn `more` on a PTY. The PTY has no window size, so pass `-n` to get
/// enough rows for a short file to fit on the first page.
#[cfg(unix)]
fn spawn_more_with_pty(
    posixly_correct: bool,
    args: &[&str],
    files: &[(&str, &str)],
) -> (MoreChild, File, File) {
    let (path, controller, replica) = pty_path();
    let (at, mut ucmd) = at_and_ucmd!();
    for (name, content) in files {
        at.write(name, content);
    }
    // UCommand starts from an empty environment, so POSIXLY_CORRECT is unset by default
    if posixly_correct {
        ucmd.env("POSIXLY_CORRECT", "1");
    }
    let child = MoreChild(
        ucmd.set_stdin(File::open(&path).unwrap())
            .set_stdout(File::create(&path).unwrap())
            .args(args)
            .args(&files.iter().map(|(name, _)| *name).collect::<Vec<_>>())
            .run_no_wait(),
    );
    (child, controller, replica)
}

#[test]
#[cfg(unix)]
fn test_exit_on_eof_by_default() {
    let (mut child, mut controller, _replica) =
        spawn_more_with_pty(false, &["-n", "10"], &[("test.txt", "line1\nline2\n")]);
    child.delay(200);
    assert!(
        !child.is_alive(),
        "more should exit when the file fits on the first page"
    );
    let mut output = vec![0u8; 4096];
    let n = controller.read(&mut output).unwrap();
    let output = String::from_utf8_lossy(&output[..n]);
    // The content is printed again after leaving the alternate screen
    let after_leave = output.split_once("\x1b[?1049l").unwrap().1;
    assert!(after_leave.contains("line1") && after_leave.contains("line2"));
}

#[test]
#[cfg(unix)]
fn test_exit_on_eof_flag_overrides_posixly_correct() {
    for flag in ["-e", "--exit-on-eof"] {
        let (mut child, _controller, _replica) =
            spawn_more_with_pty(true, &[flag, "-n", "10"], &[("test.txt", "line1\nline2\n")]);
        child.delay(200);
        assert!(
            !child.is_alive(),
            "more {flag} should exit at EOF even with POSIXLY_CORRECT set"
        );
    }
}

#[test]
#[cfg(unix)]
fn test_space_at_eof_stays_on_file_without_exit_on_eof() {
    let (mut child, mut controller, _replica) = spawn_more_with_pty(
        true,
        &["-n", "10"],
        &[("a.txt", "content_a\n"), ("b.txt", "content_b\n")],
    );
    child.delay(200);
    let mut output = vec![0u8; 4096];
    let n = controller.read(&mut output).unwrap();
    assert!(String::from_utf8_lossy(&output[..n]).contains("content_a"));
    controller.write_all(b" ").unwrap();
    child.delay(200);
    assert!(child.is_alive(), "more should stay on the file at EOF");
    let n = controller.read(&mut output).unwrap();
    assert!(
        !String::from_utf8_lossy(&output[..n]).contains("content_b"),
        "space at EOF should not move to the next file without exit_on_eof"
    );
    // `:n` is the way to the next file
    controller.write_all(b":n").unwrap();
    child.delay(200);
    let n = controller.read(&mut output).unwrap();
    assert!(String::from_utf8_lossy(&output[..n]).contains("content_b"));
    quit_more(&mut controller, child);
}

#[test]
#[cfg(unix)]
fn test_colon_n_skips_to_next_file() {
    let content_a = (1..=50)
        .map(|i| format!("a{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let (mut child, mut controller, _replica) = spawn_more_with_pty(
        true,
        &["-n", "10"],
        &[("a.txt", &content_a), ("b.txt", "content_b\n")],
    );
    child.delay(200);
    let mut output = vec![0u8; 4096];
    assert!(controller.read(&mut output).unwrap() > 0);
    controller.write_all(b":n").unwrap();
    child.delay(200);
    let n = controller.read(&mut output).unwrap();
    let output_str = String::from_utf8_lossy(&output[..n]);
    assert!(
        output_str.contains("content_b"),
        "`:n` should skip the rest of a.txt"
    );
    assert!(!output_str.contains("a50"));
    // `:n` on the last file exits
    controller.write_all(b":n").unwrap();
    child.delay(200);
    assert!(!child.is_alive(), "`:n` on the last file should exit");
}

#[test]
#[cfg(unix)]
fn test_colon_then_other_key_is_dropped() {
    let (mut child, mut controller, _replica) = spawn_more_with_pty(
        true,
        &["-n", "10"],
        &[("a.txt", "content_a\n"), ("b.txt", "content_b\n")],
    );
    child.delay(200);
    let mut output = vec![0u8; 4096];
    assert!(controller.read(&mut output).unwrap() > 0);
    // `n` alone is not `:n`, a key between `:` and `n` cancels the colon command,
    // and keys after `:` are dropped like in util-linux, so `:q` does not quit
    controller.write_all(b"n:xn:q").unwrap();
    child.delay(200);
    let n = controller.read(&mut output).unwrap();
    assert!(!String::from_utf8_lossy(&output[..n]).contains("content_b"));
    assert!(child.is_alive());
    quit_more(&mut controller, child);
}

#[test]
#[cfg(unix)]
fn test_exit_on_eof_while_paging() {
    let content = (1..=15)
        .map(|i| format!("line{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let (mut child, mut controller, _replica) =
        spawn_more_with_pty(false, &["-n", "10"], &[("test.txt", &content)]);
    child.delay(200);
    assert!(child.is_alive(), "more should wait on the first page");
    let mut output = vec![0u8; 4096];
    assert!(controller.read(&mut output).unwrap() > 0);
    controller.write_all(b" ").unwrap();
    child.delay(200);
    assert!(
        !child.is_alive(),
        "more should exit as soon as paging reaches EOF"
    );
    let n = controller.read(&mut output).unwrap();
    let output = String::from_utf8_lossy(&output[..n]);
    let after_leave = output.split_once("\x1b[?1049l").unwrap().1;
    assert!(after_leave.contains("line15"));
}

#[test]
#[cfg(unix)]
fn test_exit_on_eof_pauses_before_next_file() {
    let (mut child, mut controller, _replica) = spawn_more_with_pty(
        false,
        &["-n", "10"],
        &[("a.txt", "content_a\n"), ("b.txt", "content_b\n")],
    );
    child.delay(200);
    assert!(
        child.is_alive(),
        "more should wait before showing the next file"
    );
    let mut output = vec![0u8; 4096];
    let n = controller.read(&mut output).unwrap();
    let output_str = String::from_utf8_lossy(&output[..n]);
    assert!(output_str.contains("content_a"));
    assert!(output_str.contains("(Next file: b.txt)"));
    assert!(!output_str.contains("content_b"));
    controller.write_all(b" ").unwrap();
    child.delay(200);
    assert!(
        !child.is_alive(),
        "more should show the last file and exit at EOF"
    );
    let n = controller.read(&mut output).unwrap();
    assert!(String::from_utf8_lossy(&output[..n]).contains("content_b"));
}

#[test]
#[cfg(unix)]
fn test_squeeze_blank_lines() {
    let (child, mut controller, output) =
        run_more_with_pty(&["-s"], "test.txt", "line1\n\n\n\nline2\n");
    assert!(output.contains("line1"));
    quit_more(&mut controller, child);
}

#[test]
#[cfg(unix)]
fn test_pattern_search() {
    let (child, mut controller, output) = run_more_with_pty(
        &["-P", "target"],
        "test.txt",
        "foo\nbar\nbaz\ntarget\nend\n",
    );
    assert!(output.contains("target"));
    assert!(!output.contains("foo"));
    quit_more(&mut controller, child);
}

#[test]
#[cfg(unix)]
fn test_from_line_option() {
    let (child, mut controller, output) =
        run_more_with_pty(&["-F", "2"], "test.txt", "line1\nline2\nline3\nline4\n");
    assert!(output.contains("line2"));
    assert!(!output.contains("line1"));
    quit_more(&mut controller, child);
}

#[test]
#[cfg(unix)]
fn test_lines_option_max_u16_no_overflow() {
    // Regression test for https://github.com/uutils/coreutils/pull/12970
    // When -n is u16::MAX (65535), adding 1 for the banner line used to
    // overflow. Verify that more starts and displays content without crashing.
    let (child, mut controller, output) =
        run_more_with_pty(&["-n", "65535"], "test.txt", "line1\nline2\nline3\n");
    assert!(output.contains("line1"));
    quit_more(&mut controller, child);
}
