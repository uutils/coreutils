// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (words) reallylongexecutable nbaz

#[cfg(any(unix, target_os = "redox"))]
use std::ffi::OsStr;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_help() {
    for help_flg in ["-h", "--help"] {
        new_ucmd!()
            .arg(help_flg)
            .succeeds()
            .no_stderr()
            .stdout_contains("Usage:");
    }
}

#[test]
fn test_version() {
    for version_flg in ["-V", "--version"] {
        assert!(new_ucmd!()
            .arg(version_flg)
            .succeeds()
            .no_stderr()
            .stdout_str()
            .starts_with("basename"));
    }
}

#[test]
fn test_directory() {
    new_ucmd!()
        .args(&["/root/alpha/beta/gamma/delta/epsilon/omega/"])
        .succeeds()
        .stdout_only("omega\n");
}

#[test]
fn test_file() {
    new_ucmd!()
        .args(&["/etc/passwd"])
        .succeeds()
        .stdout_only("passwd\n");
}

#[test]
fn test_remove_suffix() {
    new_ucmd!()
        .args(&["/usr/local/bin/reallylongexecutable.exe", ".exe"])
        .succeeds()
        .stdout_only("reallylongexecutable\n");
}

#[test]
fn test_do_not_remove_suffix() {
    new_ucmd!()
        .args(&["/foo/bar/baz", "baz"])
        .succeeds()
        .stdout_only("baz\n");
}

#[test]
fn test_multiple_param() {
    for multiple_param in ["-a", "--multiple", "--mul"] {
        let path = "/foo/bar/baz";
        new_ucmd!()
            .args(&[multiple_param, path, path])
            .succeeds()
            .stdout_only("baz\nbaz\n");
    }
}

#[test]
fn test_suffix_param() {
    for suffix_param in ["-s", "--suffix", "--suf"] {
        let path = "/foo/bar/baz.exe";
        new_ucmd!()
            .args(&[suffix_param, ".exe", path, path])
            .succeeds()
            .stdout_only("baz\nbaz\n");
    }
}

#[test]
fn test_zero_param() {
    for zero_param in ["-z", "--zero", "--ze"] {
        let path = "/foo/bar/baz";
        new_ucmd!()
            .args(&[zero_param, "-a", path, path])
            .succeeds()
            .stdout_only("baz\0baz\0");
    }
}

fn expect_error(input: &[&str]) {
    assert!(!new_ucmd!()
        .args(input)
        .fails()
        .no_stdout()
        .stderr_str()
        .is_empty());
}

#[test]
fn test_invalid_option() {
    let path = "/foo/bar/baz";
    expect_error(&["-q", path]);
}

#[test]
fn test_no_args() {
    expect_error(&[]);
}

#[test]
fn test_no_args_output() {
    new_ucmd!().fails().usage_error("missing operand");
}

#[test]
fn test_too_many_args() {
    expect_error(&["a", "b", "c"]);
}

#[test]
fn test_too_many_args_output() {
    new_ucmd!()
        .args(&["a", "b", "c"])
        .fails()
        .usage_error("extra operand 'c'");
}

#[cfg(any(unix, target_os = "redox"))]
fn test_invalid_utf8_args(os_str: &OsStr) {
    let test_vec = vec![os_str.to_os_string()];
    new_ucmd!().args(&test_vec).succeeds().stdout_is("fo�o\n");
}

#[cfg(any(unix, target_os = "redox"))]
#[test]
fn invalid_utf8_args_unix() {
    use std::os::unix::ffi::OsStrExt;

    let source = [0x66, 0x6f, 0x80, 0x6f];
    let os_str = OsStr::from_bytes(&source[..]);
    test_invalid_utf8_args(os_str);
}

#[test]
fn test_root() {
    let expected = if cfg!(windows) { "\\\n" } else { "/\n" };
    new_ucmd!().arg("/").succeeds().stdout_is(expected);
}

#[test]
fn test_double_slash() {
    // TODO The GNU tests seem to suggest that some systems treat "//"
    // as the same directory as "/" directory but not all systems. We
    // should extend this test to account for that possibility.
    let expected = if cfg!(windows) { "\\\n" } else { "/\n" };
    new_ucmd!().arg("//").succeeds().stdout_is(expected);
    new_ucmd!()
        .args(&["//", "/"])
        .succeeds()
        .stdout_is(expected);
    new_ucmd!()
        .args(&["//", "//"])
        .succeeds()
        .stdout_is(expected);
}

#[test]
fn test_triple_slash() {
    let expected = if cfg!(windows) { "\\\n" } else { "/\n" };
    new_ucmd!().arg("///").succeeds().stdout_is(expected);
}

#[test]
fn test_simple_format() {
    new_ucmd!().args(&["a-a", "-a"]).succeeds().stdout_is("a\n");
    new_ucmd!()
        .args(&["a--help", "--help"])
        .succeeds()
        .stdout_is("a\n");
    new_ucmd!().args(&["a-h", "-h"]).succeeds().stdout_is("a\n");
    new_ucmd!().args(&["f.s", ".s"]).succeeds().stdout_is("f\n");
    new_ucmd!().args(&["a-s", "-s"]).succeeds().stdout_is("a\n");
    new_ucmd!().args(&["a-z", "-z"]).succeeds().stdout_is("a\n");
    new_ucmd!()
        .args(&["a", "b", "c"])
        .fails()
        .code_is(1)
        .stderr_contains("extra operand 'c'");
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_repeated_multiple() {
    new_ucmd!()
        .args(&["-aa", "-a", "foo"])
        .succeeds()
        .stdout_is("foo\n");
}

#[test]
fn test_repeated_multiple_many() {
    new_ucmd!()
        .args(&["-aa", "-a", "1/foo", "q/bar", "x/y/baz"])
        .succeeds()
        .stdout_is("foo\nbar\nbaz\n");
}

#[test]
fn test_repeated_suffix_last() {
    new_ucmd!()
        .args(&["-s", ".h", "-s", ".c", "foo.c"])
        .succeeds()
        .stdout_is("foo\n");
}

#[test]
fn test_repeated_suffix_not_first() {
    new_ucmd!()
        .args(&["-s", ".h", "-s", ".c", "foo.h"])
        .succeeds()
        .stdout_is("foo.h\n");
}

#[test]
fn test_repeated_suffix_multiple() {
    new_ucmd!()
        .args(&["-as", ".h", "-a", "-s", ".c", "foo.c", "bar.c", "bar.h"])
        .succeeds()
        .stdout_is("foo\nbar\nbar.h\n");
}

#[test]
fn test_repeated_zero() {
    new_ucmd!()
        .args(&["-zz", "-z", "foo/bar"])
        .succeeds()
        .stdout_is("bar\0");
}

#[test]
fn test_zero_does_not_imply_multiple() {
    new_ucmd!()
        .args(&["-z", "foo.c", "c"])
        .succeeds()
        .stdout_is("foo.\0");
}

#[test]
fn test_suffix_implies_multiple() {
    new_ucmd!()
        .args(&["-s", ".c", "foo.c", "o.c"])
        .succeeds()
        .stdout_is("foo\no\n");
}
