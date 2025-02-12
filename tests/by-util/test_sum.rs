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
fn test_bsd_single_file() {
    new_ucmd!()
        .arg("lorem_ipsum.txt")
        .succeeds()
        .stdout_only_fixture("bsd_single_file.expected");
}

#[test]
fn test_bsd_multiple_files() {
    new_ucmd!()
        .arg("lorem_ipsum.txt")
        .arg("alice_in_wonderland.txt")
        .succeeds()
        .stdout_only_fixture("bsd_multiple_files.expected");
}

#[test]
fn test_bsd_stdin() {
    new_ucmd!()
        .pipe_in_fixture("lorem_ipsum.txt")
        .succeeds()
        .stdout_only_fixture("bsd_stdin.expected");
}

#[test]
fn test_sysv_single_file() {
    new_ucmd!()
        .arg("-s")
        .arg("lorem_ipsum.txt")
        .succeeds()
        .stdout_only_fixture("sysv_single_file.expected");
}

#[test]
fn test_sysv_multiple_files() {
    new_ucmd!()
        .arg("-s")
        .arg("lorem_ipsum.txt")
        .arg("alice_in_wonderland.txt")
        .succeeds()
        .stdout_only_fixture("sysv_multiple_files.expected");
}

#[test]
fn test_sysv_stdin() {
    new_ucmd!()
        .arg("-s")
        .pipe_in_fixture("lorem_ipsum.txt")
        .succeeds()
        .stdout_only_fixture("sysv_stdin.expected");
}

#[test]
fn test_invalid_file() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("a");

    ucmd.arg("a").fails().stderr_is("sum: a: Is a directory\n");
}

#[test]
fn test_invalid_metadata() {
    let (_, mut ucmd) = at_and_ucmd!();

    ucmd.arg("b")
        .fails()
        .stderr_is("sum: b: No such file or directory\n");
}
