// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_unlink_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_unlink_file";

    at.touch(file);

    ucmd.arg(file).succeeds().no_stderr();

    assert!(!at.file_exists(file));
}

#[test]
fn test_unlink_multiple_files() {
    let ts = TestScenario::new(util_name!());

    let (at, mut ucmd) = (ts.fixtures.clone(), ts.ucmd());
    let file_a = "test_unlink_multiple_file_a";
    let file_b = "test_unlink_multiple_file_b";

    at.touch(file_a);
    at.touch(file_b);

    ucmd.arg(file_a)
        .arg(file_b)
        .fails()
        .stderr_contains("Usage");
}

#[test]
fn test_unlink_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir = "dir";

    at.mkdir(dir);

    let res = ucmd.arg(dir).fails();
    let stderr = res.stderr_str();
    assert!(
        stderr == "unlink: cannot unlink 'dir': Is a directory\n"
            || stderr == "unlink: cannot unlink 'dir': Permission denied\n"
    );
}

#[test]
fn test_unlink_nonexistent() {
    let file = "test_unlink_nonexistent";

    new_ucmd!()
        .arg(file)
        .fails()
        .stderr_is("unlink: cannot unlink 'test_unlink_nonexistent': No such file or directory\n");
}

#[test]
fn test_unlink_symlink() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch("foo");
    at.symlink_file("foo", "bar");

    ucmd.arg("bar").succeeds().no_stderr();

    assert!(at.file_exists("foo"));
    assert!(!at.file_exists("bar"));
}
