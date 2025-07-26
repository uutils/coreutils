// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use uutests::new_ucmd;

#[test]
fn test_get_all() {
    new_ucmd!()
        .env("HOME", "FOO")
        .env("KEY", "VALUE")
        .succeeds()
        .stdout_contains("HOME=FOO")
        .stdout_contains("KEY=VALUE");
}

#[test]
fn test_get_var() {
    new_ucmd!()
        .env("KEY", "VALUE")
        .arg("KEY")
        .succeeds()
        .stdout_contains("VALUE\n");
}

#[test]
fn test_ignore_equal_var() {
    // tested by gnu/tests/misc/printenv.sh
    new_ucmd!().env("a=b", "c").arg("a=b").fails().no_stdout();
}
