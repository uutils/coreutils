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
        .env("FOO", "BAR")
        .arg("KEY")
        .succeeds()
        .stdout_is("VALUE\n");
}

#[test]
fn test_ignore_equal_var() {
    // tested by gnu/tests/misc/printenv.sh
    new_ucmd!().env("a=b", "c").arg("a=b").fails().no_stdout();
}

#[test]
fn test_silent_error_equal_var() {
    // printenv should ignore variables with equal signs e.g. a=b=c
    new_ucmd!()
        .env("KEY", "VALUE")
        .env("a=b", "c")
        .arg("KEY")
        .arg("a=b")
        .fails_with_code(1)
        .stdout_is("VALUE\n")
        .no_stderr();
}

#[test]
fn test_silent_error_not_present() {
    // printenv should ignore unspecified variables, not panic on them
    new_ucmd!()
        .env("KEY", "VALUE")
        .arg("FOO")
        .arg("KEY")
        .fails_with_code(1)
        .stdout_is("VALUE\n")
        .no_stderr();
}

#[test]
fn test_invalid_option_exit_code() {
    // printenv should return exit code 2 for invalid options
    // This matches GNU printenv behavior and the GNU tests expectation
    new_ucmd!()
        .arg("-/")
        .fails()
        .code_is(2)
        .stderr_contains("unexpected argument")
        .stderr_contains("For more information, try '--help'");
}

#[test]
fn test_null_separator() {
    // printenv should use \x00 as separator if null option is provided
    for null_opt in ["-0", "--null"] {
        new_ucmd!()
            .env("HOME", "FOO")
            .env("KEY", "VALUE")
            .arg(null_opt)
            .succeeds()
            .stdout_contains("HOME=FOO\x00")
            .stdout_contains("KEY=VALUE\x00");

        new_ucmd!()
            .env("HOME", "FOO")
            .env("KEY", "VALUE")
            .env("FOO", "BAR")
            .arg(null_opt)
            .arg("HOME")
            .arg("KEY")
            .succeeds()
            .stdout_is("FOO\x00VALUE\x00");
    }
}
