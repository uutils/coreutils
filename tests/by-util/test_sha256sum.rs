// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use crate::common_checksum_tests::test_digest;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;
// spell-checker:ignore checkfile, testf, ntestf

test_digest! {sha256}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_conflicting_arg() {
    new_ucmd!().arg("--tag").arg("--check").fails_with_code(1);
    new_ucmd!().arg("--tag").arg("--text").fails_with_code(1);
}

#[test]
fn test_tag() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.write("foobar", "foo bar\n");
    scene
        .ccmd("sha256sum")
        .arg("--tag")
        .arg("foobar")
        .succeeds()
        .stdout_is(
            "SHA256 (foobar) = 1f2ec52b774368781bed1d1fb140a92e0eb6348090619c9291f9a5a3c8e8d151\n",
        );
}

#[test]
fn test_sha256_binary() {
    let ts = TestScenario::new(util_name!());
    assert_eq!(
        ts.fixtures.read("binary.sha256.expected"),
        ts.ucmd()
            .arg("binary.png")
            .succeeds()
            .no_stderr()
            .stdout_str()
            .split(' ')
            .next()
            .unwrap()
    );
}

#[test]
fn test_sha256_stdin_binary() {
    let ts = TestScenario::new(util_name!());
    assert_eq!(
        ts.fixtures.read("binary.sha256.expected"),
        ts.ucmd()
            .pipe_in_fixture("binary.png")
            .succeeds()
            .no_stderr()
            .stdout_str()
            .split(' ')
            .next()
            .unwrap()
    );
}

// This test is currently disabled on windows
#[test]
#[cfg_attr(windows, ignore = "Discussion is in #9168")]
fn test_check_sha256_binary() {
    new_ucmd!()
        .args(&["--check", "binary.sha256.checkfile"])
        .succeeds()
        .no_stderr()
        .stdout_is("binary.png: OK\n");
}
