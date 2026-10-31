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
    // Basic printenv functionality
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

#[test]
#[cfg(unix)]
#[cfg(not(any(target_os = "freebsd", target_os = "android", target_os = "openbsd")))]
#[cfg_attr(wasi_runner, ignore = "WASI: env values must be valid UTF-8")]
fn test_non_utf8_value() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    // Environment variable values can contain non-UTF-8 bytes on Unix.
    // printenv should output them correctly, matching GNU behavior.
    // Reproduces: LD_PRELOAD=$'/tmp/lib.so\xff' printenv LD_PRELOAD
    let value_with_invalid_utf8 = OsStr::from_bytes(b"/tmp/lib.so\xff");

    let result = new_ucmd!()
        .env("LD_PRELOAD", value_with_invalid_utf8)
        .arg("LD_PRELOAD")
        .run();

    // Use byte-based assertions to avoid UTF-8 conversion issues
    // when the test framework tries to format error messages
    assert!(
        result.succeeded(),
        "Command failed with exit code: {:?}, stderr: {:?}",
        result.code(),
        String::from_utf8_lossy(result.stderr())
    );
    result.stdout_is_bytes(b"/tmp/lib.so\xff\n");
}

#[test]
#[cfg(unix)]
#[cfg_attr(wasi_runner, ignore = "WASI: env values must be valid UTF-8")]
fn test_non_utf8_env_vars() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let non_utf8_value = OsString::from_vec(b"hello\x80world".to_vec());
    new_ucmd!()
        .env("NON_UTF8_VAR", &non_utf8_value)
        .succeeds()
        .stdout_contains_bytes(b"NON_UTF8_VAR=hello\x80world");
}

mod quoting_style {
    use uutests::new_ucmd;

    use rstest::rstest;
    use rstest_reuse::{apply, template};

    #[template]
    #[rstest]
    #[case::literal("literal", "a b", "c d")]
    #[case::escape("escape", "a\\ b", "c\\ d")]
    #[case::shell("shell", "'a b'", "'c d'")]
    #[case::c("c", "\"a b\"", "\"c d\"")]
    fn test_qs(#[case] qs: &str, #[case] key: &str, #[case] val: &str) {}

    #[apply(test_qs)]
    fn test_qs_arg(qs: &str, key: &str, val: &str) {
        new_ucmd!()
            .env("a b", "c d")
            .args(&["--quoting-style", qs])
            .succeeds()
            .stdout_contains(format!("{key}={val}"));

        new_ucmd!()
            .env("a b", "c d")
            .arg("a b")
            .args(&["--quoting-style", qs])
            .succeeds()
            .stdout_is(format!("{val}\n"));
    }

    #[cfg(unix)]
    #[apply(test_qs)]
    fn test_qs_env(qs: &str, key: &str, val: &str) {
        new_ucmd!()
            .env("QUOTING_STYLE", qs)
            .env("a b", "c d")
            .terminal_simulation(true)
            .succeeds()
            .stdout_contains(format!("{key}={val}"));

        new_ucmd!()
            .env("QUOTING_STYLE", qs)
            .env("a b", "c d")
            .arg("a b")
            .terminal_simulation(true)
            .succeeds()
            .stdout_is(format!("{val}\r\n"));
    }

    #[template]
    #[rstest]
    #[case::locale_c("C", "locale", "'a b'", "'c d'")]
    #[case::clocale_c("C", "clocale", "\"a b\"", "\"c d\"")]
    #[case::locale_utf8("en_US.UTF-8", "locale", "\u{2018}a b\u{2019}", "\u{2018}c d\u{2019}")]
    #[case::clocale_utf8("en_US.UTF-8", "clocale", "\u{2018}a b\u{2019}", "\u{2018}c d\u{2019}")]
    fn test_qs_locale(
        #[case] locale: &str,
        #[case] qs: &str,
        #[case] key: &str,
        #[case] val: &str,
    ) {
    }

    #[apply(test_qs_locale)]
    fn test_qs_locale_arg(locale: &str, qs: &str, #[case] key: &str, #[case] val: &str) {
        new_ucmd!()
            .env("LC_ALL", locale)
            .env("a b", "c d")
            .args(&["--quoting-style", qs])
            .succeeds()
            .stdout_contains(format!("{key}={val}"));

        new_ucmd!()
            .env("LC_ALL", locale)
            .env("a b", "c d")
            .arg("a b")
            .args(&["--quoting-style", qs])
            .succeeds()
            .stdout_is(format!("{val}\n"));
    }

    #[cfg(unix)]
    #[apply(test_qs_locale)]
    fn test_qs_locale_env(locale: &str, qs: &str, #[case] key: &str, #[case] val: &str) {
        new_ucmd!()
            .env("QUOTING_STYLE", qs)
            .env("LC_ALL", locale)
            .env("a b", "c d")
            .terminal_simulation(true)
            .succeeds()
            .stdout_contains(format!("{key}={val}"));

        new_ucmd!()
            .env("QUOTING_STYLE", qs)
            .env("LC_ALL", locale)
            .env("a b", "c d")
            .arg("a b")
            .terminal_simulation(true)
            .succeeds()
            .stdout_is(format!("{val}\r\n"));
    }

    // Check that the QUOTING_STYLE environment variable is ignored when
    // stdout is not a terminal (default in test environment)
    #[test]
    fn test_qs_env_no_effect_without_tty() {
        new_ucmd!()
            .env("QUOTING_STYLE", "c")
            .env("a b", "c d")
            .arg("a b")
            .succeeds()
            .stdout_is("c d\n")
            .stdout_does_not_contain("\"c d\"");
    }

    // Check that --quoting-style option takes precedence
    // over the QUOTING_STYLE environment variable
    #[cfg(unix)]
    #[test]
    fn test_qs_arg_precedence() {
        new_ucmd!()
            .env("QUOTING_STYLE", "c")
            .env("a b", "c d")
            .arg("a b")
            .arg("--quoting-style=escape")
            .terminal_simulation(true)
            .succeeds()
            .stdout_is("c\\ d\r\n")
            .stdout_does_not_contain("\"c d\"");
    }

    #[cfg(unix)]
    #[test]
    fn test_qs_env_invalid() {
        new_ucmd!()
            .env("QUOTING_STYLE", "invalid")
            .terminal_simulation(true)
            .succeeds()
            .stderr_is(
                "printenv: ignoring invalid value of environment variable QUOTING_STYLE: 'invalid'\r\n",
            );
    }
}
