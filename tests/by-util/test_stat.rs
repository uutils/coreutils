//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

extern crate regex;

use crate::common::util::*;

extern crate stat;
pub use self::stat::*;

#[test]
fn test_scanners() {
    assert_eq!(Some((-5, 2)), "-5zxc".scan_num::<i32>());
    assert_eq!(Some((51, 2)), "51zxc".scan_num::<u32>());
    assert_eq!(Some((192, 4)), "+192zxc".scan_num::<i32>());
    assert_eq!(None, "z192zxc".scan_num::<i32>());

    assert_eq!(Some(('a', 3)), "141zxc".scan_char(8));
    assert_eq!(Some(('\n', 2)), "12qzxc".scan_char(8)); // spell-checker:disable-line
    assert_eq!(Some(('\r', 1)), "dqzxc".scan_char(16)); // spell-checker:disable-line
    assert_eq!(None, "z2qzxc".scan_char(8)); // spell-checker:disable-line
}

#[test]
fn test_group_num() {
    assert_eq!("12,379,821,234", group_num("12379821234"));
    assert_eq!("21,234", group_num("21234"));
    assert_eq!("821,234", group_num("821234"));
    assert_eq!("1,821,234", group_num("1821234"));
    assert_eq!("1,234", group_num("1234"));
    assert_eq!("234", group_num("234"));
    assert_eq!("24", group_num("24"));
    assert_eq!("4", group_num("4"));
    assert_eq!("", group_num(""));
    assert_eq!("-5", group_num("-5"));
    assert_eq!("-1,234", group_num("-1234"));
}

#[test]
#[should_panic]
fn test_group_num_panic_if_invalid_numeric_characters() {
    group_num("³³³³³");
}

#[cfg(test)]
mod test_generate_tokens {
    use super::*;

    #[test]
    fn normal_format() {
        let s = "%'010.2ac%-#5.w\n";
        let expected = vec![
            Token::Directive {
                flag: F_GROUP | F_ZERO,
                width: 10,
                precision: 2,
                format: 'a',
            },
            Token::Char('c'),
            Token::Directive {
                flag: F_LEFT | F_ALTER,
                width: 5,
                precision: 0,
                format: 'w',
            },
            Token::Char('\n'),
        ];
        assert_eq!(&expected, &Stater::generate_tokens(s, false).unwrap());
    }

    #[test]
    fn printf_format() {
        let s = "%-# 15a\\t\\r\\\"\\\\\\a\\b\\e\\f\\v%+020.-23w\\x12\\167\\132\\112\\n";
        let expected = vec![
            Token::Directive {
                flag: F_LEFT | F_ALTER | F_SPACE,
                width: 15,
                precision: -1,
                format: 'a',
            },
            Token::Char('\t'),
            Token::Char('\r'),
            Token::Char('"'),
            Token::Char('\\'),
            Token::Char('\x07'),
            Token::Char('\x08'),
            Token::Char('\x1B'),
            Token::Char('\x0C'),
            Token::Char('\x0B'),
            Token::Directive {
                flag: F_SIGN | F_ZERO,
                width: 20,
                precision: -1,
                format: 'w',
            },
            Token::Char('\x12'),
            Token::Char('w'),
            Token::Char('Z'),
            Token::Char('J'),
            Token::Char('\n'),
        ];
        assert_eq!(&expected, &Stater::generate_tokens(s, true).unwrap());
    }
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
    println!("actual: {:?}", actual);
    println!("expect: {:?}", expect);
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
    println!("actual: {:?}", actual);
    println!("expect: {:?}", expect);
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
    println!("actual: {:?}", actual);
    println!("expect: {:?}", expect);
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
    if !tested {
        panic!("No symlink found to test in this environment");
    }
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
        #[cfg(any(target_vendor = "apple"))]
        "%a %A %b %B %d %D %f %F %g %G %h %i %m %n %o %s (/%T) %u %U %W %X %y %Y %z %Z",
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
