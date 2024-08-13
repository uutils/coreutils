// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use crate::common::util::{expected_result, TestScenario};

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_invalid_option() {
    new_ucmd!().arg("-w").arg("-q").arg("/").fails();
}

#[cfg(unix)]
const NORMAL_FORMAT_STR: &str =
    "%a %A %b %B %d %D %f %F %g %G %h %i %m %n %o %s %u %U %x %X %y %Y %z %Z"; // avoid "%w %W" (birth/creation) due to `stat` limitations and linux kernel & rust version capability variations
#[cfg(any(target_os = "linux", target_os = "android"))]
const DEV_FORMAT_STR: &str =
    "%a %A %b %B %d %D %f %F %g %G %h %i %m %n %o %s (%t/%T) %u %U %w %W %x %X %y %Y %z %Z";
#[cfg(target_os = "linux")]
const FS_FORMAT_STR: &str = "%b %c %i %l %n %s %S %t %T"; // avoid "%a %d %f" which can cause test failure due to race conditions

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_terse_fs_format() {
    let args = ["-f", "-t", "/proc"];
    let ts = TestScenario::new(util_name!());
    let expected_stdout = unwrap_or_return!(expected_result(&ts, &args)).stdout_move_str();
    ts.ucmd().args(&args).run().stdout_is(expected_stdout);
}

#[test]
#[cfg(target_os = "linux")]
fn test_fs_format() {
    let args = ["-f", "-c", FS_FORMAT_STR, "/dev/shm"];
    let ts = TestScenario::new(util_name!());
    let expected_stdout = unwrap_or_return!(expected_result(&ts, &args)).stdout_move_str();
    ts.ucmd().args(&args).run().stdout_is(expected_stdout);
}

#[cfg(unix)]
#[test]
fn test_terse_normal_format() {
    // note: contains birth/creation date which increases test fragility
    // * results may vary due to built-in `stat` limitations as well as linux kernel and rust version capability variations
    let args = ["-t", "/"];
    let ts = TestScenario::new(util_name!());
    let actual = ts.ucmd().args(&args).succeeds().stdout_move_str();
    let expect = unwrap_or_return!(expected_result(&ts, &args)).stdout_move_str();
    println!("actual: {actual:?}");
    println!("expect: {expect:?}");
    let v_actual: Vec<&str> = actual.trim().split(' ').collect();
    let mut v_expect: Vec<&str> = expect.trim().split(' ').collect();
    assert!(!v_expect.is_empty());

    // uu_stat does not support selinux
    if v_actual.len() == v_expect.len() - 1 && v_expect[v_expect.len() - 1].contains(':') {
        // assume last element contains: `SELinux security context string`
        v_expect.pop();
    }

    // * allow for inequality if `stat` (aka, expect) returns "0" (unknown value)
    assert!(
        expect == "0"
            || expect == "0\n"
            || v_actual
                .iter()
                .zip(v_expect.iter())
                .all(|(a, e)| a == e || *e == "0" || *e == "0\n")
    );
}

#[cfg(unix)]
#[test]
fn test_format_created_time() {
    let args = ["-c", "%w", "/bin"];
    let ts = TestScenario::new(util_name!());
    let actual = ts.ucmd().args(&args).succeeds().stdout_move_str();
    let expect = unwrap_or_return!(expected_result(&ts, &args)).stdout_move_str();
    println!("actual: {actual:?}");
    println!("expect: {expect:?}");
    // note: using a regex instead of `split_whitespace()` in order to detect whitespace differences
    let re = regex::Regex::new(r"\s").unwrap();
    let v_actual: Vec<&str> = re.split(&actual).collect();
    let v_expect: Vec<&str> = re.split(&expect).collect();
    assert!(!v_expect.is_empty());
    // * allow for inequality if `stat` (aka, expect) returns "-" (unknown value)
    assert!(
        expect == "-"
            || expect == "-\n"
            || v_actual
                .iter()
                .zip(v_expect.iter())
                .all(|(a, e)| a == e || *e == "-" || *e == "-\n")
    );
}

#[cfg(unix)]
#[test]
fn test_format_created_seconds() {
    let args = ["-c", "%W", "/bin"];
    let ts = TestScenario::new(util_name!());
    let actual = ts.ucmd().args(&args).succeeds().stdout_move_str();
    let expect = unwrap_or_return!(expected_result(&ts, &args)).stdout_move_str();
    println!("actual: {actual:?}");
    println!("expect: {expect:?}");
    // note: using a regex instead of `split_whitespace()` in order to detect whitespace differences
    let re = regex::Regex::new(r"\s").unwrap();
    let v_actual: Vec<&str> = re.split(&actual).collect();
    let v_expect: Vec<&str> = re.split(&expect).collect();
    assert!(!v_expect.is_empty());
    // * allow for inequality if `stat` (aka, expect) returns "0" (unknown value)
    assert!(
        expect == "0"
            || expect == "0\n"
            || v_actual
                .iter()
                .zip(v_expect.iter())
                .all(|(a, e)| a == e || *e == "0" || *e == "0\n")
    );
}

#[cfg(unix)]
#[test]
fn test_normal_format() {
    let args = ["-c", NORMAL_FORMAT_STR, "/bin"];
    let ts = TestScenario::new(util_name!());
    let expected_stdout = unwrap_or_return!(expected_result(&ts, &args)).stdout_move_str();
    ts.ucmd().args(&args).succeeds().stdout_is(expected_stdout);
}

#[cfg(unix)]
#[cfg(not(target_os = "openbsd"))]
#[test]
fn test_symlinks() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    let mut tested: bool = false;
    // arbitrarily chosen symlinks with hope that the CI environment provides at least one of them
    for file in [
        "/bin/sh",
        "/data/data/com.termux/files/usr/bin/sh", // spell-checker:disable-line
        "/bin/sudoedit",
        "/usr/bin/ex",
        "/etc/localtime",
        "/etc/aliases",
    ] {
        if at.file_exists(file) && at.is_symlink(file) {
            tested = true;
            let args = ["-c", NORMAL_FORMAT_STR, file];
            let expected_stdout = unwrap_or_return!(expected_result(&ts, &args)).stdout_move_str();
            ts.ucmd().args(&args).succeeds().stdout_is(expected_stdout);
            // -L, --dereference    follow links
            let args = ["-L", "-c", NORMAL_FORMAT_STR, file];
            let expected_stdout = unwrap_or_return!(expected_result(&ts, &args)).stdout_move_str();
            ts.ucmd().args(&args).succeeds().stdout_is(expected_stdout);
        }
    }
    assert!(tested, "No symlink found to test in this environment");
}

#[cfg(any(target_os = "linux", target_os = "android", target_vendor = "apple"))]
#[test]
fn test_char() {
    // TODO: "(%t) (%x) (%w)" deviate from GNU stat for `character special file` on macOS
    // Diff < left / right > :
    // <"(f0000) (2021-05-20 23:08:03.442555000 +0200) (1970-01-01 01:00:00.000000000 +0100)\n"
    // >"(f) (2021-05-20 23:08:03.455598000 +0200) (-)\n"
    let args = [
        "-c",
        #[cfg(any(target_os = "linux", target_os = "android"))]
        DEV_FORMAT_STR,
        #[cfg(target_os = "linux")]
        "/dev/pts/ptmx",
        #[cfg(target_vendor = "apple")]
        "%a %A %b %B %d %D %f %F %g %G %h %i %m %n %o %s (/%T) %u %U %W %X %y %Y %z %Z",
        #[cfg(any(target_os = "android", target_vendor = "apple"))]
        "/dev/ptmx",
    ];
    let ts = TestScenario::new(util_name!());
    let expected_stdout = unwrap_or_return!(expected_result(&ts, &args)).stdout_move_str();
    ts.ucmd().args(&args).succeeds().stdout_is(expected_stdout);
}

#[cfg(any(target_os = "linux", target_os = "android", target_vendor = "apple"))]
#[test]
fn test_date() {
    // Just test the date for the time 0.3 change
    let args = [
        "-c",
        #[cfg(any(target_os = "linux", target_os = "android"))]
        "%z",
        #[cfg(target_os = "linux")]
        "/bin/sh",
        #[cfg(target_vendor = "apple")]
        "%z",
        #[cfg(any(target_os = "android", target_vendor = "apple"))]
        "/bin/sh",
    ];
    let ts = TestScenario::new(util_name!());
    let expected_stdout = unwrap_or_return!(expected_result(&ts, &args)).stdout_move_str();
    ts.ucmd().args(&args).succeeds().stdout_is(expected_stdout);
    // Just test the date for the time 0.3 change
    let args = [
        "-c",
        #[cfg(any(target_os = "linux", target_os = "android"))]
        "%z",
        #[cfg(target_os = "linux")]
        "/dev/ptmx",
        #[cfg(target_vendor = "apple")]
        "%z",
        #[cfg(any(target_os = "android", target_vendor = "apple"))]
        "/dev/ptmx",
    ];
    let ts = TestScenario::new(util_name!());
    let expected_stdout = unwrap_or_return!(expected_result(&ts, &args)).stdout_move_str();
    ts.ucmd().args(&args).succeeds().stdout_is(expected_stdout);
}
#[cfg(unix)]
#[test]
fn test_multi_files() {
    let args = [
        "-c",
        NORMAL_FORMAT_STR,
        "/dev",
        "/usr/lib",
        #[cfg(target_os = "linux")]
        "/etc/fstab",
        "/var",
    ];
    let ts = TestScenario::new(util_name!());
    let expected_stdout = unwrap_or_return!(expected_result(&ts, &args)).stdout_move_str();
    ts.ucmd().args(&args).succeeds().stdout_is(expected_stdout);
}

#[cfg(unix)]
#[test]
fn test_printf() {
    let args = [
        "--printf=123%-# 15q\\r\\\"\\\\\\a\\b\\e\\f\\v%+020.23m\\x12\\167\\132\\112\\n",
        "/",
    ];
    let ts = TestScenario::new(util_name!());
    let expected_stdout = unwrap_or_return!(expected_result(&ts, &args)).stdout_move_str();
    ts.ucmd().args(&args).succeeds().stdout_is(expected_stdout);
}

#[test]
#[cfg(unix)]
fn test_pipe_fifo() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkfifo("FIFO");
    ucmd.arg("FIFO")
        .run()
        .no_stderr()
        .stdout_contains("fifo")
        .stdout_contains("File: FIFO")
        .succeeded();
}

#[test]
#[cfg(all(
    unix,
    not(any(target_os = "android", target_os = "freebsd", target_os = "openbsd"))
))]
fn test_stdin_pipe_fifo1() {
    // $ echo | stat -
    // File: -
    // Size: 0               Blocks: 0          IO Block: 4096   fifo
    new_ucmd!()
        .arg("-")
        .set_stdin(std::process::Stdio::piped())
        .run()
        .no_stderr()
        .stdout_contains("fifo")
        .stdout_contains("File: -")
        .succeeded();
    new_ucmd!()
        .args(&["-L", "-"])
        .set_stdin(std::process::Stdio::piped())
        .run()
        .no_stderr()
        .stdout_contains("fifo")
        .stdout_contains("File: -")
        .succeeded();
}

#[test]
#[cfg(all(unix, not(target_os = "android")))]
fn test_stdin_pipe_fifo2() {
    // $ stat -
    // File: -
    // Size: 0               Blocks: 0          IO Block: 1024   character special file
    new_ucmd!()
        .arg("-")
        .set_stdin(std::process::Stdio::null())
        .run()
        .no_stderr()
        .stdout_contains("character special file")
        .stdout_contains("File: -")
        .succeeded();
}

#[test]
#[cfg(all(unix, not(target_os = "android")))]
fn test_stdin_with_fs_option() {
    // $ stat -f -
    new_ucmd!()
        .arg("-f")
        .arg("-")
        .set_stdin(std::process::Stdio::null())
        .fails()
        .code_is(1)
        .stderr_contains("using '-' to denote standard input does not work in file system mode");
}

#[test]
#[cfg(all(
    unix,
    not(any(
        target_os = "android",
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd"
    ))
))]
fn test_stdin_redirect() {
    // $ touch f && stat - < f
    // File: -
    // Size: 0               Blocks: 0          IO Block: 4096   regular empty file
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("f");
    ts.ucmd()
        .arg("-")
        .set_stdin(std::fs::File::open(at.plus("f")).unwrap())
        .run()
        .no_stderr()
        .stdout_contains("regular empty file")
        .stdout_contains("File: -")
        .succeeded();
}

#[test]
fn test_without_argument() {
    new_ucmd!()
        .fails()
        .stderr_contains("missing operand\nTry 'stat --help' for more information.");
}
