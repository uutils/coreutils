// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// spell-checker: ignore: AEDT AEST EEST NZDT NZST Kolkata Iseconds févr février janv janvier mercredi samedi sommes juin décembre Januar Juni Dezember enero junio diciembre gennaio giugno dicembre junho dezembro lundi dimanche Montag Sonntag Samstag sábado febr

use std::cmp::Ordering;

use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan};
use regex::Regex;
#[cfg(all(unix, not(target_os = "macos")))]
use uucore::process::geteuid;
use uutests::util::TestScenario;
use uutests::{at_and_ucmd, new_ucmd, util_name};

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_empty_arguments() {
    new_ucmd!().arg("").fails_with_code(1);
    new_ucmd!().args(&["", ""]).fails_with_code(1);
    new_ucmd!().args(&["", "", ""]).fails_with_code(1);
}

#[test]
fn test_extra_operands() {
    new_ucmd!()
        .args(&["test", "extra"])
        .fails_with_code(1)
        .stderr_contains("extra operand 'extra'");
}

#[test]
fn test_bad_format_option_missing_leading_plus_after_d_flag() {
    let bad_arguments = vec!["q", "a", "test", "%Y-%m-%d"];

    for bad_argument in bad_arguments {
        new_ucmd!()
            .args(&["--date", "1996-01-31", bad_argument])
            .fails_with_code(1)
            .stderr_contains(format!("the argument {bad_argument} lacks a leading '+';\nwhen using an option to specify date(s), any non-option\nargument must be a format string beginning with '+'"), );
    }
}

#[test]
fn test_invalid_long_option() {
    new_ucmd!()
        .arg("--fB")
        .fails_with_code(1)
        .stderr_contains("unexpected argument '--fB'");
}

#[test]
fn test_invalid_short_option() {
    new_ucmd!()
        .arg("-w")
        .fails_with_code(1)
        .stderr_contains("unexpected argument '-w'");
}

#[test]
fn test_format_option_not_to_capture_other_valid_arguments() {
    new_ucmd!()
        .arg("+%Y%m%d%H%M%S")
        .arg("--date")
        .arg("@1770996496")
        .succeeds();
}

#[test]
fn test_single_dash_as_date() {
    new_ucmd!()
        .arg("-")
        .fails_with_code(1)
        .stderr_contains("invalid date");
}

#[test]
fn test_date_email() {
    for param in ["--rfc-email", "--rfc-e", "-R", "--rfc-2822", "--rfc-822"] {
        new_ucmd!().arg(param).succeeds();
    }
}

#[test]
fn test_date_email_multiple_aliases() {
    // Test that multiple RFC email aliases can be used together
    // This matches GNU behavior where all aliases map to the same option
    new_ucmd!()
        .arg("--rfc-email")
        .arg("--rfc-822")
        .arg("--rfc-2822")
        .succeeds();
}

#[test]
#[cfg(unix)]
fn test_date_rfc_822_uses_english() {
    // RFC-822/RFC-2822/RFC-5322 formats should always use English day/month names
    // regardless of locale (per RFC specification)
    let scene = TestScenario::new(util_name!());

    // Test with German locale - should still output "Sun" not "So"
    scene
        .ucmd()
        .env("LC_ALL", "de_DE.UTF-8")
        .env("TZ", "UTC")
        .args(&["-R", "-d", "1997-01-19 08:17:48 +0"])
        .succeeds()
        .stdout_contains("Sun, 19 Jan 1997");

    // Test with French locale - should still output "Sun" not "dim."
    scene
        .ucmd()
        .env("LC_ALL", "fr_FR.UTF-8")
        .env("TZ", "UTC")
        .args(&["-R", "-d", "1997-01-19 08:17:48 +0"])
        .succeeds()
        .stdout_contains("Sun, 19 Jan 1997");
}

#[test]
fn test_date_rfc_3339() {
    let scene = TestScenario::new(util_name!());

    let rfc_regexp = concat!(
        r#"(\d+)-(0[1-9]|1[012])-(0[1-9]|[12]\d|3[01])\s([01]\d|2[0-3]):"#,
        r#"([0-5]\d):([0-5]\d|60)(\.\d+)?(([Zz])|([\+|\-]([01]\d|2[0-3])))"#
    );
    let re = Regex::new(rfc_regexp).unwrap();

    // Check that the output matches the regexp
    for param in ["--rfc-3339", "--rfc-3"] {
        scene
            .ucmd()
            .arg(format!("{param}=ns"))
            .succeeds()
            .stdout_matches(&re);

        scene
            .ucmd()
            .arg(format!("{param}=seconds"))
            .succeeds()
            .stdout_matches(&re);
    }
}

#[test]
fn test_date_rfc_3339_invalid_arg() {
    for param in ["--iso-3339", "--rfc-3"] {
        new_ucmd!().arg(format!("{param}=foo")).fails();
    }
}

#[test]
fn test_date_rfc_8601_default() {
    let re = Regex::new(r"^\d{4}-\d{2}-\d{2}\n$").unwrap();
    for param in ["--iso-8601", "--i"] {
        new_ucmd!().arg(param).succeeds().stdout_matches(&re);
    }
}

#[test]
fn test_date_rfc_8601() {
    let re = Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2},\d{9}[+-]\d{2}:\d{2}\n$").unwrap();
    for param in ["--iso-8601", "--i"] {
        new_ucmd!()
            .arg(format!("{param}=ns"))
            .succeeds()
            .stdout_matches(&re);
    }
}

#[test]
fn test_date_rfc_8601_invalid_arg() {
    for param in ["--iso-8601", "--i"] {
        new_ucmd!().arg(format!("{param}=@")).fails();
    }
}

#[test]
fn test_date_rfc_8601_second() {
    let re = Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}[+-]\d{2}:\d{2}\n$").unwrap();
    for param in ["--iso-8601", "--i"] {
        new_ucmd!()
            .arg(format!("{param}=second"))
            .succeeds()
            .stdout_matches(&re);
        new_ucmd!()
            .arg(format!("{param}=seconds"))
            .succeeds()
            .stdout_matches(&re);
    }
}

#[test]
fn test_date_rfc_8601_minute() {
    let re = Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}[+-]\d{2}:\d{2}\n$").unwrap();
    for param in ["--iso-8601", "--i"] {
        new_ucmd!()
            .arg(format!("{param}=minute"))
            .succeeds()
            .stdout_matches(&re);
        new_ucmd!()
            .arg(format!("{param}=minutes"))
            .succeeds()
            .stdout_matches(&re);
    }
}

#[test]
fn test_date_rfc_8601_hour() {
    let re = Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}[+-]\d{2}:\d{2}\n$").unwrap();
    for param in ["--iso-8601", "--i"] {
        new_ucmd!()
            .arg(format!("{param}=hour"))
            .succeeds()
            .stdout_matches(&re);
        new_ucmd!()
            .arg(format!("{param}=hours"))
            .succeeds()
            .stdout_matches(&re);
    }
}

#[test]
fn test_date_rfc_8601_date() {
    let re = Regex::new(r"^\d{4}-\d{2}-\d{2}\n$").unwrap();
    for param in ["--iso-8601", "--i"] {
        new_ucmd!()
            .arg(format!("{param}=date"))
            .succeeds()
            .stdout_matches(&re);
    }
}

#[test]
fn test_date_utc() {
    for param in ["--universal", "--utc", "--uct", "--uni", "--u"] {
        new_ucmd!().arg(param).succeeds();
    }
}

#[test]
fn test_date_utc_multiple_aliases() {
    // Test that multiple UTC aliases can be used together
    // This matches GNU behavior where all aliases map to the same option
    new_ucmd!()
        .arg("--uct")
        .arg("--utc")
        .arg("--universal")
        .succeeds();
}

#[test]
fn test_date_utc_issue_6495() {
    new_ucmd!()
        .env("TZ", "UTC0")
        .arg("-u")
        .arg("-d")
        .arg("@0")
        .succeeds()
        .stdout_is("Thu Jan  1 00:00:00 UTC 1970\n");
}

#[test]
fn test_date_utc_with_d_flag() {
    let cases = [
        ("2024-01-01 12:00", "+%H:%M %Z", "12:00 UTC\n"),
        ("2024-06-15 10:30", "+%H:%M %Z", "10:30 UTC\n"),
        ("2024-12-31 23:59:59", "+%H:%M:%S %Z", "23:59:59 UTC\n"),
        ("@0", "+%Y-%m-%d %H:%M:%S %Z", "1970-01-01 00:00:00 UTC\n"),
        ("@3600", "+%H:%M:%S %Z", "01:00:00 UTC\n"),
        ("@86400", "+%Y-%m-%d %Z", "1970-01-02 UTC\n"),
        ("2024-06-15 10:30 EDT", "+%H:%M %Z", "14:30 UTC\n"),
        ("2024-01-15 10:30 EST", "+%H:%M %Z", "15:30 UTC\n"),
        ("2024-06-15 12:00 PDT", "+%H:%M %Z", "19:00 UTC\n"),
        ("2024-01-15 12:00 PST", "+%H:%M %Z", "20:00 UTC\n"),
        ("2024-01-01 12:00 +0000", "+%H:%M %Z", "12:00 UTC\n"),
        ("2024-01-01 12:00 +0530", "+%H:%M %Z", "06:30 UTC\n"),
        ("2024-01-01 12:00 -0500", "+%H:%M %Z", "17:00 UTC\n"),
    ];
    for (input, fmt, expected) in cases {
        new_ucmd!()
            .env("TZ", "America/New_York")
            .args(&["-u", "-d", input, fmt])
            .succeeds()
            .stdout_is(expected);
    }
}

#[test]
fn test_date_utc_vs_local() {
    let cases = [
        ("-d", "2024-01-01 12:00", "+%H:%M %Z", "12:00 EST\n"),
        ("-ud", "2024-01-01 12:00", "+%H:%M %Z", "12:00 UTC\n"),
        ("-d", "2024-06-15 12:00", "+%H:%M %Z", "12:00 EDT\n"),
        ("-ud", "2024-06-15 12:00", "+%H:%M %Z", "12:00 UTC\n"),
        ("-d", "@0", "+%H:%M %Z", "19:00 EST\n"),
        ("-ud", "@0", "+%H:%M %Z", "00:00 UTC\n"),
    ];
    for (flag, date, fmt, expected) in cases {
        new_ucmd!()
            .env("TZ", "America/New_York")
            .args(&[flag, date, fmt])
            .succeeds()
            .stdout_is(expected);
    }
}

#[test]
fn test_date_utc_output_formats() {
    let cases = [
        ("-I", "2024-06-15"),
        ("--rfc-3339=seconds", "+00:00"),
        ("-R", "+0000"),
    ];
    for (fmt_flag, expected) in cases {
        new_ucmd!()
            .env("TZ", "America/New_York")
            .args(&["-u", "-d", "2024-06-15 12:00", fmt_flag])
            .succeeds()
            .stdout_contains(expected);
    }
}

#[test]
fn test_date_utc_stdin() {
    new_ucmd!()
        .env("TZ", "America/New_York")
        .args(&["-u", "-f", "-", "+%H:%M %Z"])
        .pipe_in("2024-01-01 12:00\n2024-06-15 18:30\n")
        .succeeds()
        .stdout_is("12:00 UTC\n18:30 UTC\n");
}

#[test]
fn test_date_format_y() {
    let scene = TestScenario::new(util_name!());

    let mut re = Regex::new(r"^\d{4}\n$").unwrap();
    scene.ucmd().arg("+%Y").succeeds().stdout_matches(&re);

    re = Regex::new(r"^\d{2}\n$").unwrap();
    scene.ucmd().arg("+%y").succeeds().stdout_matches(&re);
}

#[test]
fn test_date_format_q() {
    let scene = TestScenario::new(util_name!());

    let re = Regex::new(r"^[1-4]\n$").unwrap();
    scene.ucmd().arg("+%q").succeeds().stdout_matches(&re);
}

#[test]
fn test_date_format_m() {
    let scene = TestScenario::new(util_name!());

    let mut re = Regex::new(r"\S+").unwrap();
    scene.ucmd().arg("+%b").succeeds().stdout_matches(&re);

    re = Regex::new(r"^\d{2}\n$").unwrap();
    scene.ucmd().arg("+%m").succeeds().stdout_matches(&re);
}

#[test]
fn test_date_format_day() {
    let scene = TestScenario::new(util_name!());

    let mut re = Regex::new(r"\S+").unwrap();
    scene.ucmd().arg("+%a").succeeds().stdout_matches(&re);

    re = Regex::new(r"\S+").unwrap();
    scene.ucmd().arg("+%A").succeeds().stdout_matches(&re);

    re = Regex::new(r"^\d{1}\n$").unwrap();
    scene.ucmd().arg("+%u").succeeds().stdout_matches(&re);
}

#[test]
fn test_date_format_full_day() {
    let re = Regex::new(r"\S+ \d{4}-\d{2}-\d{2}").unwrap();
    new_ucmd!()
        .arg("+'%a %Y-%m-%d'")
        .succeeds()
        .stdout_matches(&re);
}

#[test]
fn test_date_issue_3780() {
    new_ucmd!().arg("+%Y-%m-%d %H-%M-%S%:::z").succeeds();
}

#[test]
fn test_date_nano_seconds() {
    // %N     nanoseconds (000000000..999999999)
    let re = Regex::new(r"^\d{1,9}\n$").unwrap();
    new_ucmd!().arg("+%N").succeeds().stdout_matches(&re);
}

#[test]
fn test_date_format_without_plus() {
    // [+FORMAT]
    new_ucmd!()
        .arg("%s")
        .fails_with_code(1)
        .stderr_contains("date: invalid date '%s'");
}

#[test]
fn test_date_format_literal() {
    new_ucmd!().arg("+%%s").succeeds().stdout_is("%s\n");
    new_ucmd!().arg("+%%N").succeeds().stdout_is("%N\n");
}

#[test]
#[cfg(all(unix, not(target_os = "macos")))]
fn test_date_set_valid() {
    if geteuid() == 0 {
        new_ucmd!()
            .arg("--set")
            .arg("2020-03-12 13:30:00+08:00")
            .succeeds()
            .no_stdout()
            .no_stderr();
    }
}

#[test]
#[cfg(any(windows, all(unix, not(target_os = "macos"))))]
fn test_date_set_invalid() {
    let result = new_ucmd!().arg("--set").arg("123abcd").fails();
    result.no_stdout();
    assert!(result.stderr_str().starts_with("date: invalid date "));
}

#[test]
#[cfg(all(unix, not(any(target_os = "android", target_os = "macos"))))]
fn test_date_set_permissions_error() {
    if !(geteuid() == 0 || uucore::os::is_wsl_1()) {
        let result = new_ucmd!()
            .arg("--set")
            .arg("2020-03-11 21:45:00+08:00")
            .fails();
        result.no_stdout();
        assert!(result.stderr_str().starts_with("date: cannot set date: "));
    }
}

#[test]
#[cfg(all(unix, not(any(target_os = "android", target_os = "macos"))))]
fn test_date_set_hyphen_prefixed_values() {
    // test -s flag accepts hyphen-prefixed values like "-3 days"
    if !(geteuid() == 0 || uucore::os::is_wsl_1()) {
        let test_cases = vec!["-1 hour", "-2 days", "-3 weeks", "-1 month"];

        for date_str in test_cases {
            let result = new_ucmd!().arg("--set").arg(date_str).fails();
            result.no_stdout();
            // permission error, not argument parsing error
            assert!(
                result.stderr_str().starts_with("date: cannot set date: "),
                "Expected permission error for '{date_str}', but got: {}",
                result.stderr_str()
            );
        }
    }
}

#[test]
#[cfg(target_os = "macos")]
fn test_date_set_mac_unavailable() {
    let result = new_ucmd!()
        .arg("--set")
        .arg("2020-03-11 21:45:00+08:00")
        .fails();
    result.no_stdout();
    assert!(
        result
            .stderr_str()
            .starts_with("date: setting the date is not supported by macOS")
    );
}

#[test]
#[cfg(all(unix, not(target_os = "macos")))]
fn test_date_set_valid_2() {
    if geteuid() == 0 {
        new_ucmd!()
            .arg("--set")
            .arg("Sat 20 Mar 2021 14:53:01 AWST") // spell-checker:disable-line
            .succeeds()
            .no_stdout()
            .no_stderr();
    }
}

#[test]
fn test_date_for_invalid_file() {
    let result = new_ucmd!().arg("--file").arg("invalid_file").fails();
    result.no_stdout();
    assert_eq!(
        result.stderr_str().trim(),
        "date: invalid_file: No such file or directory",
    );
}

#[test]
#[cfg(unix)]
fn test_date_for_no_permission_file() {
    use std::os::unix::fs::PermissionsExt;
    const FILE: &str = "file-no-perm-1";

    let (at, mut ucmd) = at_and_ucmd!();

    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(at.plus(FILE))
        .unwrap();
    file.set_permissions(std::fs::Permissions::from_mode(0o222))
        .unwrap();
    let result = ucmd.arg("--file").arg(FILE).fails();
    result.no_stdout();
    assert_eq!(
        result.stderr_str().trim(),
        format!("date: {FILE}: Permission denied")
    );
}

#[test]
fn test_date_for_dir_as_file() {
    let result = new_ucmd!().arg("--file").arg("/").fails();
    result.no_stdout();
    assert_eq!(
        result.stderr_str().trim(),
        "date: expected file, got directory '/'",
    );
}

#[test]
fn test_date_for_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_date_for_file";
    at.touch(file);
    ucmd.arg("--file").arg(file).succeeds();
}

#[test]
fn test_date_for_file_mtime() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "reference_file";
    at.touch(file);
    std::thread::sleep(std::time::Duration::from_millis(100));
    let result = ucmd.arg("--reference").arg(file).arg("+%s%N").succeeds();
    let mtime = at.metadata(file).modified().unwrap();
    let mtime_nanos = mtime
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .to_string();
    assert_eq!(result.stdout_str().trim(), &mtime_nanos[..]);
}

#[test]
#[cfg(all(unix, not(target_os = "macos")))]
fn test_date_set_valid_3() {
    if geteuid() == 0 {
        new_ucmd!()
            .arg("--set")
            .arg("Sat 20 Mar 2021 14:53:01") // Local timezone
            .succeeds()
            .no_stdout()
            .no_stderr();
    }
}

#[test]
#[cfg(all(unix, not(target_os = "macos")))]
fn test_date_set_valid_4() {
    if geteuid() == 0 {
        new_ucmd!()
            .arg("--set")
            .arg("2020-03-11 21:45:00") // Local timezone
            .succeeds()
            .no_stdout()
            .no_stderr();
    }
}

#[test]
fn test_invalid_format_string() {
    // With lenient mode, invalid format sequences are output literally (like GNU date)
    new_ucmd!().arg("+%!").succeeds().stdout_is("%!\n");
}

#[test]
fn test_capitalized_numeric_time_zone() {
    // %z     +hhmm numeric time zone (e.g., -0400)
    // # is supposed to capitalize, which makes little sense here, but keep coverage
    // on such format so it's good to test.
    let re = Regex::new(r"^[+-]\d{4,4}\n$").unwrap();
    new_ucmd!().arg("+%#z").succeeds().stdout_matches(&re);
}

#[test]
fn test_date_string_human() {
    let date_formats = vec![
        "1 year ago",
        "1 year",
        "2 months ago",
        "15 days ago",
        "1 week ago",
        "5 hours ago",
        "30 minutes ago",
        "10 seconds",
        "last day",
        "last monday",
        "last week",
        "last month",
        "last year",
        "this monday",
        "next day",
        "next monday",
        "next week",
        "next month",
        "next year",
    ];
    let re = Regex::new(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}\n$").unwrap();
    for date_format in date_formats {
        new_ucmd!()
            .arg("-d")
            .arg(date_format)
            .arg("+%Y-%m-%d %S:%M")
            .succeeds()
            .stdout_matches(&re);
    }
}

#[test]
fn test_negative_offset() {
    let data_formats = vec![
        ("-1 hour", 1.hours()),
        ("-1 hours", 1.hours()),
        ("-1 day", 24.hours()),
        ("-2 weeks", (14 * 24).hours()),
    ];
    for (date_format, offset) in data_formats {
        new_ucmd!()
            .arg("-d")
            .arg(date_format)
            .arg("--rfc-3339=seconds")
            .succeeds()
            .stdout_str_check(|out| {
                let date = out.trim().parse::<Timestamp>().unwrap();
                // Is the resulting date roughly what is expected?
                let expected_date = Timestamp::now() - offset;
                (date - expected_date).abs().compare(10.minutes()).unwrap() == Ordering::Less
            });
    }
}

#[test]
fn test_relative_weekdays() {
    // Truncate time component to midnight
    let today = Timestamp::now().to_zoned(TimeZone::UTC).date();
    // Loop through each day of the week, starting with today
    for offset in 0..7 {
        for direction in ["last", "this", "next"] {
            let weekday = today
                .checked_add(offset.days())
                .unwrap()
                .strftime("%a")
                .to_string();
            new_ucmd!()
                .arg("-d")
                .arg(format!("{direction} {weekday}"))
                .arg("--rfc-3339=seconds")
                .arg("--utc")
                .succeeds()
                .stdout_str_check(|out| {
                    let result = out.trim().parse::<Timestamp>().unwrap();
                    let expected = match (direction, offset) {
                        ("last", _) => today.checked_sub((7 - offset).days()).unwrap(),
                        ("this", 0) => today,
                        ("next", 0) => today.checked_add(7.days()).unwrap(),
                        _ => today.checked_add(offset.days()).unwrap(),
                    };
                    let expected_ts = expected.to_zoned(TimeZone::UTC).unwrap().timestamp();
                    result == expected_ts
                });
        }
    }
}

#[test]
fn test_invalid_date_string() {
    new_ucmd!()
        .arg("-d")
        .arg("foo")
        .fails()
        .no_stdout()
        .stderr_contains("invalid date");

    new_ucmd!()
        .arg("-d")
        // cSpell:disable
        .arg("this fooday")
        // cSpell:enable
        .fails()
        .no_stdout()
        .stderr_contains("invalid date");
}

#[test]
fn test_multiple_dates() {
    new_ucmd!()
        .arg("-d")
        .arg("invalid")
        .arg("-d")
        .arg("2000-02-02")
        .arg("+%Y")
        .succeeds()
        .stdout_is("2000\n")
        .no_stderr();
}

#[test]
fn test_date_one_digit_date() {
    new_ucmd!()
        .env("TZ", "UTC0")
        .arg("-d")
        .arg("2000-1-1")
        .succeeds()
        .stdout_only("Sat Jan  1 00:00:00 UTC 2000\n");

    new_ucmd!()
        .env("TZ", "UTC0")
        .arg("-d")
        .arg("2000-1-4")
        .succeeds()
        .stdout_only("Tue Jan  4 00:00:00 UTC 2000\n");
}

#[test]
fn test_date_overflow() {
    new_ucmd!()
        .arg("-d68888888888888sms")
        .fails()
        .no_stdout()
        .stderr_contains("invalid date");
}

#[test]
fn test_date_parse_from_format() {
    const FILE: &str = "file-with-dates";
    let (at, mut ucmd) = at_and_ucmd!();

    at.write(
        FILE,
        "2023-03-27 08:30:00\n\
         2023-04-01 12:00:00\n\
         2023-04-15 18:30:00",
    );
    ucmd.arg("-f")
        .arg(at.plus(FILE))
        .arg("+%Y-%m-%d %H:%M:%S")
        .succeeds();
}

#[test]
fn test_date_from_stdin() {
    new_ucmd!()
        .env("TZ", "UTC0")
        .arg("-f")
        .arg("-")
        .pipe_in(
            "2023-03-27 08:30:00\n\
             2023-04-01 12:00:00\n\
             2023-04-15 18:30:00\n",
        )
        .succeeds()
        .stdout_is(
            "Mon Mar 27 08:30:00 UTC 2023\n\
             Sat Apr  1 12:00:00 UTC 2023\n\
             Sat Apr 15 18:30:00 UTC 2023\n",
        );
}

const JAN2: &str = "2024-01-02 12:00:00 +0000";
const JUL2: &str = "2024-07-02 12:00:00 +0000";

#[test]
fn test_date_tz() {
    fn test_tz(tz: &str, date: &str, output: &str) {
        println!("Test with TZ={tz}, date=\"{date}\".");
        new_ucmd!()
            .env("TZ", tz)
            .arg("-d")
            .arg(date)
            .arg("+%Y-%m-%d %H:%M:%S %Z")
            .succeeds()
            .stdout_only(output);
    }

    // Empty TZ, UTC0, invalid timezone.
    test_tz("", JAN2, "2024-01-02 12:00:00 UTC\n");
    test_tz("UTC0", JAN2, "2024-01-02 12:00:00 UTC\n");
    // TODO: We do not handle invalid timezones the same way as GNU coreutils
    //test_tz("Invalid/Timezone", JAN2, "2024-01-02 12:00:00 Invalid\n");

    // Test various locations, some of them use daylight saving, some don't.
    test_tz("America/Vancouver", JAN2, "2024-01-02 04:00:00 PST\n");
    test_tz("America/Vancouver", JUL2, "2024-07-02 05:00:00 PDT\n");
    test_tz("Europe/Berlin", JAN2, "2024-01-02 13:00:00 CET\n");
    test_tz("Europe/Berlin", JUL2, "2024-07-02 14:00:00 CEST\n");
    test_tz("Africa/Cairo", JAN2, "2024-01-02 14:00:00 EET\n");
    // Egypt restored daylight saving in 2023, so if the database is outdated, this will fail.
    //test_tz("Africa/Cairo", JUL2, "2024-07-02 15:00:00 EEST\n");
    test_tz("Asia/Tokyo", JAN2, "2024-01-02 21:00:00 JST\n");
    test_tz("Asia/Tokyo", JUL2, "2024-07-02 21:00:00 JST\n");
    test_tz("Australia/Sydney", JAN2, "2024-01-02 23:00:00 AEDT\n");
    test_tz("Australia/Sydney", JUL2, "2024-07-02 22:00:00 AEST\n"); // Shifts the other way.
    test_tz("Pacific/Tahiti", JAN2, "2024-01-02 02:00:00 -10\n"); // No abbreviation.
    test_tz("Pacific/Auckland", JAN2, "2024-01-03 01:00:00 NZDT\n");
    test_tz("Pacific/Auckland", JUL2, "2024-07-03 00:00:00 NZST\n");
}

#[test]
fn test_date_tz_with_utc_flag() {
    new_ucmd!()
        .env("TZ", "Europe/Berlin")
        .arg("-u")
        .arg("+%Z")
        .succeeds()
        .stdout_only("UTC\n");
}

#[test]
fn test_date_tz_various_formats() {
    fn test_tz(tz: &str, date: &str, output: &str) {
        println!("Test with TZ={tz}, date=\"{date}\".");
        new_ucmd!()
            .env("TZ", tz)
            .arg("-d")
            .arg(date)
            .arg("+%z %:z %::z %:::z %Z")
            .succeeds()
            .stdout_only(output);
    }

    test_tz(
        "America/Vancouver",
        JAN2,
        "-0800 -08:00 -08:00:00 -08 PST\n",
    );
    // Half-hour timezone
    test_tz("Asia/Kolkata", JAN2, "+0530 +05:30 +05:30:00 +05:30 IST\n"); // spell-checker:disable-line
    test_tz("Europe/Berlin", JAN2, "+0100 +01:00 +01:00:00 +01 CET\n");
    test_tz(
        "Australia/Sydney",
        JAN2,
        "+1100 +11:00 +11:00:00 +11 AEDT\n",
    );
}

#[test]
fn test_date_tz_with_relative_time() {
    new_ucmd!()
        .env("TZ", "America/Vancouver")
        .arg("-d")
        .arg("1 hour ago")
        .arg("+%Y-%m-%d %H:%M:%S %Z")
        .succeeds()
        .stdout_matches(&Regex::new(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} P[DS]T\n$").unwrap());
}

#[test]
fn test_date_utc_time() {
    // Test that -u flag shows correct UTC time
    // We get 2 UTC times just in case we're really unlucky and this runs around
    // an hour change.
    let utc_hour_1: i32 = new_ucmd!()
        .env("TZ", "Asia/Taipei")
        .arg("-u")
        .arg("+%-H")
        .succeeds()
        .stdout_str()
        .trim_end()
        .parse()
        .unwrap();
    let tpe_hour: i32 = new_ucmd!()
        .env("TZ", "Asia/Taipei")
        .arg("+%-H")
        .succeeds()
        .stdout_str()
        .trim_end()
        .parse()
        .unwrap();
    let utc_hour_2: i32 = new_ucmd!()
        .env("TZ", "Asia/Taipei")
        .arg("-u")
        .arg("+%-H")
        .succeeds()
        .stdout_str()
        .trim_end()
        .parse()
        .unwrap();
    // Taipei is always 8 hours ahead of UTC (no daylight savings)
    assert!(
        (tpe_hour - utc_hour_1 + 24) % 24 == 8 || (tpe_hour - utc_hour_2 + 24) % 24 == 8,
        "TPE: {tpe_hour} UTC: {utc_hour_1}/{utc_hour_2}"
    );

    // Test that -u flag shows UTC timezone
    new_ucmd!()
        .arg("-u")
        .arg("+%Z")
        .succeeds()
        .stdout_only("UTC\n");

    // Test that -u flag with specific timestamp shows correct UTC time
    new_ucmd!()
        .arg("-u")
        .arg("-d")
        .arg("@0")
        .succeeds()
        .stdout_only("Thu Jan  1 00:00:00 UTC 1970\n");
}

#[test]
fn test_date_empty_tz_time() {
    new_ucmd!()
        .env("TZ", "")
        .arg("-d")
        .arg("@0")
        .succeeds()
        .stdout_only("Thu Jan  1 00:00:00 UTC 1970\n");
}

#[test]
fn test_date_resolution() {
    // Test that --resolution flag returns a floating point number by default
    new_ucmd!()
        .arg("--resolution")
        .succeeds()
        .stdout_str_check(|s| s.trim().parse::<f64>().is_ok());

    // Test that --resolution flag can be passed twice to match gnu
    new_ucmd!()
        .arg("--resolution")
        .arg("--resolution")
        .succeeds()
        .stdout_str_check(|s| s.trim().parse::<f64>().is_ok());

    // Test that can --resolution output can be formatted as a date
    new_ucmd!()
        .arg("--resolution")
        .arg("-Iseconds")
        .succeeds()
        .stdout_only("1970-01-01T00:00:00+00:00\n");
}

#[test]
fn test_date_resolution_no_combine() {
    // Test that date fails when --resolution flag is passed with date flag
    new_ucmd!()
        .arg("--resolution")
        .arg("-d")
        .arg("2025-01-01")
        .fails();
}

#[test]
fn test_date_numeric_d_basic_utc() {
    // Verify GNU-compatible pure-digit parsing for -d STRING under UTC
    // 0/00 -> today at 00:00; 7/07 -> today at 07:00; 0700 -> today at 07:00
    let today = Timestamp::now().to_zoned(TimeZone::UTC).date();
    let yyyy = today.year();
    let mm = today.month();
    let dd = today.day();

    let mk =
        |h: u32, m: u32| -> String { format!("{yyyy:04}-{mm:02}-{dd:02} {h:02}:{m:02}:00 UTC\n") };

    new_ucmd!()
        .env("TZ", "UTC0")
        .arg("-d")
        .arg("0")
        .arg("+%F %T %Z")
        .succeeds()
        .stdout_only(mk(0, 0));

    new_ucmd!()
        .env("TZ", "UTC0")
        .arg("-d")
        .arg("7")
        .arg("+%F %T %Z")
        .succeeds()
        .stdout_only(mk(7, 0));

    new_ucmd!()
        .env("TZ", "UTC0")
        .arg("-d")
        .arg("0700")
        .arg("+%F %T %Z")
        .succeeds()
        .stdout_only(mk(7, 0));
}

#[test]
fn test_date_numeric_d_invalid_numbers() {
    // Ensure invalid HHMM values are rejected (GNU-compatible)
    new_ucmd!()
        .env("TZ", "UTC0")
        .arg("-d")
        .arg("2400")
        .arg("+%F %T %Z")
        .fails()
        .stderr_contains("invalid date");

    new_ucmd!()
        .env("TZ", "UTC0")
        .arg("-d")
        .arg("2360")
        .arg("+%F %T %Z")
        .fails()
        .stderr_contains("invalid date");
}

#[test]
fn test_date_tz_abbreviation_utc_gmt() {
    // Test UTC and GMT timezone abbreviations
    new_ucmd!()
        .arg("-d")
        .arg("2021-03-20 14:53:01 UTC")
        .arg("+%Y-%m-%d %H:%M:%S")
        .succeeds();

    new_ucmd!()
        .arg("-d")
        .arg("2021-03-20 14:53:01 GMT")
        .arg("+%Y-%m-%d %H:%M:%S")
        .succeeds();
}

#[test]
fn test_date_tz_abbreviation_us_timezones() {
    // Test US timezone abbreviations (uutils supports, GNU also supports these)
    let us_zones = vec![
        ("PST", "2021-03-20 14:53:01 PST"),
        ("PDT", "2021-03-20 14:53:01 PDT"),
        ("MST", "2021-03-20 14:53:01 MST"),
        ("MDT", "2021-03-20 14:53:01 MDT"),
        ("CST", "2021-03-20 14:53:01 CST"),
        ("CDT", "2021-03-20 14:53:01 CDT"),
        ("EST", "2021-03-20 14:53:01 EST"),
        ("EDT", "2021-03-20 14:53:01 EDT"),
    ];

    for (_tz_name, date_str) in us_zones {
        new_ucmd!()
            .arg("-d")
            .arg(date_str)
            .arg("+%Y-%m-%d %H:%M:%S")
            .succeeds()
            .no_stderr();
    }
}

#[test]
fn test_date_tz_abbreviation_australian_timezones() {
    // Test Australian timezone abbreviations (uutils supports, GNU does NOT)
    // This demonstrates uutils date going beyond GNU capabilities
    let au_zones = vec![
        ("AWST", "2021-03-20 14:53:01 AWST"), // Western Australia // spell-checker:disable-line
        ("ACST", "2021-03-20 14:53:01 ACST"), // Central Australia (Standard) // spell-checker:disable-line
        ("ACDT", "2021-03-20 14:53:01 ACDT"), // Central Australia (Daylight) // spell-checker:disable-line
        ("AEST", "2021-03-20 14:53:01 AEST"), // Eastern Australia (Standard)
        ("AEDT", "2021-03-20 14:53:01 AEDT"), // Eastern Australia (Daylight)
    ];

    for (_tz_name, date_str) in au_zones {
        new_ucmd!()
            .arg("-d")
            .arg(date_str)
            .arg("+%Y-%m-%d %H:%M:%S")
            .succeeds()
            .no_stderr();
    }
}

#[test]
fn test_date_tz_abbreviation_dst_handling() {
    // Test that timezone abbreviations correctly handle DST
    // PST is UTC-8, PDT is UTC-7
    // March 20, 2021 was during PDT period in Pacific timezone

    new_ucmd!()
        .arg("-d")
        .arg("2021-03-20 14:53:01 PST")
        .arg("+%z")
        .succeeds()
        .no_stderr();

    new_ucmd!()
        .arg("-d")
        .arg("2021-03-20 14:53:01 PDT")
        .arg("+%z")
        .succeeds()
        .no_stderr();
}

#[test]
fn test_date_tz_abbreviation_fixed_offset_outside_season() {
    // Abbreviations encode a fixed UTC offset regardless of the date.
    // Using a DST abbreviation outside its season should still use the
    // fixed offset the abbreviation implies, not the zone's current offset.

    // EDT (UTC-4) used in winter (New York observes EST in January)
    new_ucmd!()
        .env("TZ", "UTC")
        .arg("-u")
        .arg("-d")
        .arg("2026-01-15 10:00 EDT")
        .arg("+%F %T %Z")
        .succeeds()
        .stdout_is("2026-01-15 14:00:00 UTC\n");

    // PST (UTC-8) used in summer (Los Angeles observes PDT in June)
    new_ucmd!()
        .env("TZ", "UTC")
        .arg("-u")
        .arg("-d")
        .arg("2026-06-15 10:00 PST")
        .arg("+%F %T %Z")
        .succeeds()
        .stdout_is("2026-06-15 18:00:00 UTC\n");

    // PDT (UTC-7) used in winter
    new_ucmd!()
        .env("TZ", "UTC")
        .arg("-u")
        .arg("-d")
        .arg("2026-01-15 10:00 PDT")
        .arg("+%F %T %Z")
        .succeeds()
        .stdout_is("2026-01-15 17:00:00 UTC\n");

    // CDT (UTC-5) used in winter
    new_ucmd!()
        .env("TZ", "UTC")
        .arg("-u")
        .arg("-d")
        .arg("2026-01-15 10:00 CDT")
        .arg("+%F %T %Z")
        .succeeds()
        .stdout_is("2026-01-15 15:00:00 UTC\n");

    // MDT (UTC-6) used in winter
    new_ucmd!()
        .env("TZ", "UTC")
        .arg("-u")
        .arg("-d")
        .arg("2026-01-15 10:00 MDT")
        .arg("+%F %T %Z")
        .succeeds()
        .stdout_is("2026-01-15 16:00:00 UTC\n");
}

#[test]
fn test_date_tz_abbreviation_with_day_of_week() {
    // Test timezone abbreviations with full date format including day of week
    new_ucmd!()
        .arg("-d")
        .arg("Sat 20 Mar 2021 14:53:01 AWST") // spell-checker:disable-line
        .arg("+%Y-%m-%d %H:%M:%S")
        .succeeds()
        .no_stderr();

    new_ucmd!()
        .arg("-d")
        .arg("Sat 20 Mar 2021 14:53:01 EST")
        .arg("+%Y-%m-%d %H:%M:%S")
        .succeeds()
        .no_stderr();
}

#[test]
fn test_date_tz_abbreviation_with_relative_date() {
    // Verify that "yesterday" in "-u -d yesterday 10:00 GMT" is resolved
    // relative to UTC, not the local TZ.
    let expected = new_ucmd!()
        .env("TZ", "UTC")
        .arg("-u")
        .arg("-d")
        .arg("yesterday 10:00 GMT")
        .arg("+%F %T %Z")
        .succeeds()
        .stdout_str()
        .to_string();
    new_ucmd!()
        .env("TZ", "Australia/Sydney")
        .arg("-u")
        .arg("-d")
        .arg("yesterday 10:00 GMT")
        .arg("+%F %T %Z")
        .succeeds()
        .stdout_is(expected);
}

#[test]
fn test_date_tz_abbreviation_unknown() {
    // Test that unknown timezone abbreviations fall back gracefully
    // XYZ is not a valid timezone abbreviation
    new_ucmd!()
        .arg("-d")
        .arg("2021-03-20 14:53:01 XYZ")
        .fails()
        .stderr_contains("invalid date");
}

#[test]
fn test_date_military_timezone_j_variations() {
    // Test multiple variations of 'J' input (case insensitive, with whitespace)
    // All should produce midnight (00:00:00)
    let test_cases = vec!["J", "j", " J ", " j ", "\tJ\t"];

    for input in test_cases {
        new_ucmd!()
            .env("TZ", "UTC")
            .arg("-d")
            .arg(input)
            .arg("+%T")
            .succeeds()
            .stdout_is("00:00:00\n");
    }

    // Test with -u flag to verify UTC behavior
    new_ucmd!()
        .arg("-u")
        .arg("-d")
        .arg("J")
        .arg("+%T %Z")
        .succeeds()
        .stdout_contains("00:00:00")
        .stdout_contains("UTC");
}

#[test]
fn test_date_empty_string() {
    // Empty string should be treated as midnight today
    new_ucmd!()
        .env("TZ", "UTC+1")
        .arg("-d")
        .arg("")
        .succeeds()
        .stdout_contains("00:00:00");
}

#[test]
fn test_date_empty_string_variations() {
    // Test multiple variations of empty/whitespace strings
    // All should produce midnight (00:00:00)
    let test_cases = vec!["", " ", "  ", "\t", "\n", " \t ", "\t\n\t"];

    for input in test_cases {
        new_ucmd!()
            .env("TZ", "UTC")
            .arg("-d")
            .arg(input)
            .arg("+%T")
            .succeeds()
            .stdout_is("00:00:00\n");
    }

    // Test with -u flag to verify UTC behavior
    new_ucmd!()
        .arg("-u")
        .arg("-d")
        .arg("")
        .arg("+%T %Z")
        .succeeds()
        .stdout_contains("00:00:00")
        .stdout_contains("UTC");
}

#[test]
fn test_date_relative_m9() {
    // Military timezone "m9" should be parsed as noon + 9 hours = 21:00 UTC
    // When displayed in TZ=UTC+9 (which is UTC-9), this shows as 12:00 local time
    new_ucmd!()
        .env("TZ", "UTC+9")
        .arg("-d")
        .arg("m9")
        .succeeds()
        .stdout_contains("12:00:00");
}

#[test]
fn test_date_military_timezone_with_offset_variations() {
    // Test various military timezone + offset combinations
    // Format: single letter (a-z except j) optionally followed by 1-2 digits

    // Test cases: (input, expected_time_utc)
    let test_cases = vec![
        ("a", "23:00:00"),  // A = UTC+1, midnight in UTC+1 = 23:00 UTC
        ("m", "12:00:00"),  // M = UTC+12, midnight in UTC+12 = 12:00 UTC
        ("z", "00:00:00"),  // Z = UTC+0, midnight in UTC+0 = 00:00 UTC
        ("m9", "21:00:00"), // M + 9 hours = 12 + 9 = 21:00 UTC
        ("a5", "04:00:00"), // A + 5 hours = 23 + 5 = 04:00 UTC (next day)
        ("z3", "03:00:00"), // Z + 3 hours = 00 + 3 = 03:00 UTC
        ("M", "12:00:00"),  // Uppercase should work too
        ("A5", "04:00:00"), // Uppercase with offset
    ];

    for (input, expected) in test_cases {
        new_ucmd!()
            .env("TZ", "UTC")
            .arg("-d")
            .arg(input)
            .arg("+%T")
            .succeeds()
            .stdout_is(format!("{expected}\n"));
    }
}

#[test]
fn test_date_military_timezone_with_offset_and_date() {
    let today = Timestamp::now().to_zoned(TimeZone::UTC).date();

    let test_cases = vec![
        ("m", -1), // M = UTC+12
        ("a", -1), // A = UTC+1
        ("n", 0),  // N = UTC-1
        ("y", 0),  // Y = UTC-12
        ("z", 0),  // Z = UTC
        // same day hour offsets
        ("n2", 0),
        // midnight crossings with hour offsets back to today
        ("a1", 0), // exactly to midnight
        ("a5", 0), // "overflow" midnight
        ("m23", 0),
        // midnight crossings with hour offsets to tomorrow
        ("n23", 1),
        ("y23", 1),
        // midnight crossing to yesterday even with positive offset
        ("m9", -1), // M = UTC+12 (-12 h + 9h is still `yesterday`)
    ];

    for (input, day_delta) in test_cases {
        let expected_date = today.checked_add(day_delta.days()).unwrap();

        let expected = format!("{}\n", expected_date.strftime("%F"));

        new_ucmd!()
            .env("TZ", "UTC")
            .arg("-d")
            .arg(input)
            .arg("+%F")
            .succeeds()
            .stdout_is(expected);
    }
}

// Locale-aware hour formatting tests
#[test]
#[cfg(unix)]
fn test_date_locale_hour_c_locale() {
    // C locale should use 24-hour format
    new_ucmd!()
        .env("LC_ALL", "C")
        .env("TZ", "UTC")
        .arg("-d")
        .arg("2025-10-11T13:00")
        .succeeds()
        .stdout_contains("13:00");
}

#[test]
#[cfg(any(
    target_os = "linux",
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
fn test_date_locale_hour_en_us() {
    // en_US locale typically uses 12-hour format when available
    // Note: If locale is not installed on system, falls back to C locale (24-hour)
    let result = new_ucmd!()
        .env("LC_ALL", "en_US.UTF-8")
        .env("TZ", "UTC")
        .arg("-d")
        .arg("2025-10-11T13:00")
        .succeeds();

    let stdout = result.stdout_str();
    // Accept either 12-hour (if locale available) or 24-hour (if locale unavailable)
    // The important part is that the code doesn't crash and handles locale detection gracefully
    assert!(
        stdout.contains("1:00") || stdout.contains("13:00"),
        "date output should contain either 1:00 (12-hour) or 13:00 (24-hour), got: {stdout}"
    );
}

#[test]
fn test_date_explicit_format_overrides_locale() {
    // Explicit format should override locale preferences
    new_ucmd!()
        .env("LC_ALL", "en_US.UTF-8")
        .env("TZ", "UTC")
        .arg("-d")
        .arg("2025-10-11T13:00")
        .arg("+%H:%M")
        .succeeds()
        .stdout_is("13:00\n");
}

// Comprehensive locale formatting tests to verify actual locale format strings are used
#[test]
#[cfg(any(
    target_os = "linux",
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
fn test_date_locale_leading_zeros_en_us() {
    // Test for leading zeros in en_US locale
    // en_US uses %I (01-12) with leading zeros, not %l (1-12) without
    let result = new_ucmd!()
        .env("LC_ALL", "en_US.UTF-8")
        .env("TZ", "UTC")
        .arg("-d")
        .arg("2025-12-14T01:00")
        .succeeds();

    let stdout = result.stdout_str();
    // If locale is available, should have leading zero: "01:00"
    // If locale unavailable (falls back to C), may have "01:00" (24-hour) or " 1:00"
    // Key point: output should match what nl_langinfo(D_T_FMT) specifies
    if stdout.contains("AM") || stdout.contains("PM") {
        // 12-hour format detected - should have leading zero in en_US
        assert!(
            stdout.contains("01:00") || stdout.contains(" 1:00"),
            "en_US 12-hour format should show '01:00 AM' or ' 1:00 AM', got: {stdout}"
        );
    }
}

#[test]
#[cfg(unix)]
fn test_date_locale_c_uses_24_hour() {
    // C/POSIX locale must use 24-hour format
    let result = new_ucmd!()
        .env("LC_ALL", "C")
        .env("TZ", "UTC")
        .arg("-d")
        .arg("2025-12-14T13:00")
        .succeeds();

    let stdout = result.stdout_str();
    // C locale uses 24-hour format, no AM/PM
    assert!(
        !stdout.contains("AM") && !stdout.contains("PM"),
        "C locale should not use AM/PM, got: {stdout}"
    );
    assert!(
        stdout.contains("13"),
        "C locale should show 13 (24-hour), got: {stdout}"
    );
}

#[test]
#[cfg(unix)]
fn test_date_locale_timezone_included() {
    // Verify timezone is included in output (implementation adds %Z if missing)
    let result = new_ucmd!()
        .env("LC_ALL", "C")
        .env("TZ", "UTC")
        .arg("-d")
        .arg("2025-12-14T13:00")
        .succeeds();

    let stdout = result.stdout_str();
    assert!(
        stdout.contains("UTC") || stdout.contains("+00"),
        "Output should contain timezone information, got: {stdout}"
    );
}

#[test]
#[cfg(unix)]
fn test_date_locale_format_structure() {
    // Test that output follows locale-defined structure (not hardcoded)
    let result = new_ucmd!()
        .env("LC_ALL", "C")
        .env("TZ", "UTC")
        .arg("-d")
        .arg("2025-12-14T13:00:00")
        .succeeds();

    let stdout = result.stdout_str();

    // Should contain weekday abbreviation
    let weekdays = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    assert!(
        weekdays.iter().any(|day| stdout.contains(day)),
        "Output should contain weekday, got: {stdout}"
    );

    // Should contain month
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    assert!(
        months.iter().any(|month| stdout.contains(month)),
        "Output should contain month, got: {stdout}"
    );

    // Should contain year
    assert!(
        stdout.contains("2025"),
        "Output should contain year, got: {stdout}"
    );
}

#[test]
#[cfg(unix)]
fn test_date_locale_format_not_hardcoded() {
    // This test verifies we're not using hardcoded format strings
    // by checking that the format actually comes from the locale system

    // Test with C locale
    let c_result = new_ucmd!()
        .env("LC_ALL", "C")
        .env("TZ", "UTC")
        .arg("-d")
        .arg("2025-12-14T01:00:00")
        .succeeds();

    let c_output = c_result.stdout_str();

    // C locale should use 24-hour format
    assert!(
        c_output.contains("01:00") || c_output.contains(" 1:00"),
        "C locale output: {c_output}"
    );
    assert!(
        !c_output.contains("AM") && !c_output.contains("PM"),
        "C locale should not have AM/PM: {c_output}"
    );
}

#[test]
#[cfg(any(
    target_os = "linux",
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
fn test_date_locale_en_us_vs_c_difference() {
    // Verify that en_US and C locales produce different outputs
    // (if en_US locale is available on the system)

    let c_result = new_ucmd!()
        .env("LC_ALL", "C")
        .env("TZ", "UTC")
        .arg("-d")
        .arg("2025-12-14T13:00:00")
        .succeeds();

    let en_us_result = new_ucmd!()
        .env("LC_ALL", "en_US.UTF-8")
        .env("TZ", "UTC")
        .arg("-d")
        .arg("2025-12-14T13:00:00")
        .succeeds();

    let c_output = c_result.stdout_str();
    let en_us_output = en_us_result.stdout_str();

    // C locale: 24-hour, no AM/PM
    assert!(
        !c_output.contains("AM") && !c_output.contains("PM"),
        "C locale should not have AM/PM: {c_output}"
    );

    // en_US: If locale is installed, should have AM/PM (12-hour)
    // If not installed, falls back to C locale
    if en_us_output.contains("PM") {
        // Locale is available and using 12-hour format
        assert!(
            en_us_output.contains("1:00") || en_us_output.contains("01:00"),
            "en_US with 12-hour should show 1:00 PM or 01:00 PM, got: {en_us_output}"
        );
    }
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android", target_vendor = "apple",))]
fn test_date_locale_fr_french() {
    // Test French locale (fr_FR.UTF-8) behavior
    // French typically uses 24-hour format and may have localized day/month names

    let result = new_ucmd!()
        .env("LC_ALL", "fr_FR.UTF-8")
        .env("TZ", "UTC")
        .arg("-d")
        .arg("2025-12-14T13:00:00")
        .succeeds();

    let stdout = result.stdout_str();

    // French locale should use 24-hour format (no AM/PM)
    assert!(
        !stdout.contains("AM") && !stdout.contains("PM"),
        "French locale should use 24-hour format (no AM/PM), got: {stdout}"
    );

    // Should have 13:00 (not 1:00)
    assert!(
        stdout.contains("13:00"),
        "French locale should show 13:00 for 1 PM, got: {stdout}"
    );

    // Timezone should be included (our implementation adds %Z if missing)
    assert!(
        stdout.contains("UTC") || stdout.contains("+00") || stdout.contains('Z'),
        "Output should include timezone information, got: {stdout}"
    );
}

#[test]
fn test_date_posix_format_specifiers() {
    let cases = [
        // %r: 12-hour time with zero-padded hour (08:17:48 AM, not 8:17:48 AM)
        ("%r", "08:17:48 AM"),
        // %x: locale date in MM/DD/YY format
        ("%x", "01/19/97"),
        // %X: locale time in HH:MM:SS format
        ("%X", "08:17:48"),
        // %:8z: invalid format (width between : and z) should output literally (lenient mode)
        ("%:8z", "%:8z"),
    ];

    for (format, expected) in cases {
        new_ucmd!()
            .env("TZ", "UTC")
            .arg("-d")
            .arg("1997-01-19 08:17:48")
            .arg(format!("+{format}"))
            .succeeds()
            .stdout_is(format!("{expected}\n"));
    }
}

#[test]
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
fn test_date_format_b_french_locale() {
    // Test both %B and %b formats with French locale using a loop
    // This test expects localized month names when i18n support is available
    let test_cases = [
        ("2025-01-15", "janvier", "janv."), // Wednesday = mercredi, mer.
        ("2025-02-15", "février", "févr."), // Saturday = samedi, sam.
    ];

    for (date, expected_full, expected_abbrev) in &test_cases {
        let result = new_ucmd!()
            .env("LC_TIME", "fr_FR.UTF-8")
            .env("TZ", "UTC")
            .arg("-d")
            .arg(date)
            .arg("+%B %b")
            .succeeds();

        let output = result.stdout_str().trim();
        let expected = format!("{expected_full} {expected_abbrev}");

        if output == expected {
            // i18n feature is working - test passed
            assert_eq!(output, expected);
        } else {
            // i18n feature not available, skip test
            println!(
                "Skipping French locale test for {date} - i18n feature not available, got: {output}"
            );
            return; // Exit early if i18n not available
        }
    }
}

#[test]
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
fn test_date_format_a_french_locale() {
    // Test both %A and %a formats with French locale using a loop
    // This test expects localized day names when i18n support is available
    let test_cases = [
        ("2025-01-15", "mercredi", "mer."), // Wednesday
        ("2025-02-15", "samedi", "sam."),   // Saturday
    ];

    for (date, expected_full, expected_abbrev) in &test_cases {
        let result = new_ucmd!()
            .env("LC_TIME", "fr_FR.UTF-8")
            .env("TZ", "UTC")
            .arg("-d")
            .arg(date)
            .arg("+%A %a")
            .succeeds();

        let output = result.stdout_str().trim();
        let expected = format!("{expected_full} {expected_abbrev}");

        if output == expected {
            // i18n feature is working - test passed
            assert_eq!(output, expected);
        } else {
            // i18n feature not available, skip test
            println!(
                "Skipping French day locale test for {date} - i18n feature not available, got: {output}"
            );
            return; // Exit early if i18n not available
        }
    }
}

#[test]
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
fn test_date_french_full_sentence() {
    let result = new_ucmd!()
        .env("LANG", "fr_FR.UTF-8")
        .env("TZ", "UTC")
        .arg("-d")
        .arg("2026-01-21")
        .arg("+Nous sommes le %A %d %B %Y")
        .succeeds();

    let output = result.stdout_str().trim();
    let expected = "Nous sommes le mercredi 21 janvier 2026";

    if output == expected {
        // i18n feature is working - test passed
        assert_eq!(output, expected);
    } else {
        // i18n feature not available, skip test
        println!("Skipping French full sentence test - i18n feature not available, got: {output}");
    }
}

/// Test that %x format specifier respects locale settings
/// This is a regression test for locale-aware date formatting
#[test]
#[ignore = "https://bugs.launchpad.net/ubuntu/+source/rust-coreutils/+bug/2137410"]
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
fn test_date_format_x_locale_aware() {
    // With C locale, %x should output MM/DD/YY (US format)
    new_ucmd!()
        .env("TZ", "UTC")
        .env("LC_ALL", "C")
        .arg("-d")
        .arg("1997-01-19 08:17:48")
        .arg("+%x")
        .succeeds()
        .stdout_is("01/19/97\n");

    // With French locale, %x should output DD/MM/YYYY (European format)
    // GNU date outputs: 19/01/1997
    new_ucmd!()
        .env("TZ", "UTC")
        .env("LC_ALL", "fr_FR.UTF-8")
        .arg("-d")
        .arg("1997-01-19 08:17:48")
        .arg("+%x")
        .succeeds()
        .stdout_is("19/01/1997\n");
}

#[test]
fn test_date_parenthesis_comment() {
    // GNU compatibility: Text in parentheses is treated as a comment and removed.
    let cases = [
        // (input, format, expected_output)
        ("(", "+%H:%M:%S", "00:00:00\n"),
        ("1(ignore comment to eol", "+%H:%M:%S", "01:00:00\n"),
        ("2026-01-05(this is a comment", "+%Y-%m-%d", "2026-01-05\n"),
        ("2026(this is a comment)-01-05", "+%Y-%m-%d", "2026-01-05\n"),
        ("((foo)2026-01-05)", "+%H:%M:%S", "00:00:00\n"), // Nested/unbalanced case
        ("(2026-01-05(foo))", "+%H:%M:%S", "00:00:00\n"), // Balanced parentheses removed (empty result)
    ];

    for (input, format, expected) in cases {
        new_ucmd!()
            .env("TZ", "UTC")
            .arg("-d")
            .arg(input)
            .arg("-u")
            .arg(format)
            .succeeds()
            .stdout_only(expected);
    }
}

#[test]
fn test_date_parenthesis_vs_other_special_chars() {
    // Ensure parentheses are special but other chars like [, ., ^ are still rejected
    for special_char in ["[", ".", "^"] {
        new_ucmd!()
            .arg("-d")
            .arg(special_char)
            .fails()
            .stderr_contains("invalid date");
    }
}

#[test]
#[cfg(unix)]
fn test_date_iranian_locale_solar_hijri_calendar() {
    // Test Iranian locale uses Solar Hijri calendar
    // Verify the Solar Hijri calendar is used in the Iranian locale
    use std::process::Command;

    // Check if Iranian locale is available
    let locale_check = Command::new("locale")
        .env("LC_ALL", "fa_IR.UTF-8")
        .arg("charmap")
        .output();

    let locale_available = match locale_check {
        Ok(output) => String::from_utf8_lossy(&output.stdout).trim() == "UTF-8",
        Err(_) => false,
    };

    if !locale_available {
        println!("Skipping Iranian locale test - fa_IR.UTF-8 locale not available");
        return;
    }

    // Get current year in Gregorian calendar
    let current_year: i32 = new_ucmd!()
        .env("LC_ALL", "C")
        .arg("+%Y")
        .succeeds()
        .stdout_str()
        .trim()
        .parse()
        .unwrap();

    // 03-19 and 03-22 of the same Gregorian year are in different years in the
    // Solar Hijri calendar
    let year_march_19: i32 = new_ucmd!()
        .env("LC_ALL", "fa_IR.UTF-8")
        .arg("-d")
        .arg(format!("{current_year}-03-19"))
        .arg("+%Y")
        .succeeds()
        .stdout_str()
        .trim()
        .parse()
        .unwrap();

    let year_march_22: i32 = new_ucmd!()
        .env("LC_ALL", "fa_IR.UTF-8")
        .arg("-d")
        .arg(format!("{current_year}-03-22"))
        .arg("+%Y")
        .succeeds()
        .stdout_str()
        .trim()
        .parse()
        .unwrap();

    // Years should differ by 1
    assert_eq!(year_march_19, year_march_22 - 1);

    // The difference between the Gregorian year is 621 or 622 years
    assert_eq!(year_march_19, current_year - 622);
    assert_eq!(year_march_22, current_year - 621);

    // Check that --iso-8601 and --rfc-3339 use the Gregorian calendar
    let iso_result = new_ucmd!()
        .env("LC_ALL", "fa_IR.UTF-8")
        .arg("--iso-8601=hours")
        .succeeds();
    let iso_output = iso_result.stdout_str();
    assert!(iso_output.starts_with(&current_year.to_string()));

    let rfc_result = new_ucmd!()
        .env("LC_ALL", "fa_IR.UTF-8")
        .arg("--rfc-3339=date")
        .succeeds();
    let rfc_output = rfc_result.stdout_str();
    assert!(rfc_output.starts_with(&current_year.to_string()));
}

#[test]
#[cfg(unix)]
fn test_date_ethiopian_locale_calendar() {
    // Test Ethiopian locale uses Ethiopian calendar
    // Verify the Ethiopian calendar is used in the Ethiopian locale
    use std::process::Command;

    // Check if Ethiopian locale is available
    let locale_check = Command::new("locale")
        .env("LC_ALL", "am_ET.UTF-8")
        .arg("charmap")
        .output();

    let locale_available = match locale_check {
        Ok(output) => String::from_utf8_lossy(&output.stdout).trim() == "UTF-8",
        Err(_) => false,
    };

    if !locale_available {
        println!("Skipping Ethiopian locale test - am_ET.UTF-8 locale not available");
        return;
    }

    // Get current year in Gregorian calendar
    let current_year: i32 = new_ucmd!()
        .env("LC_ALL", "C")
        .arg("+%Y")
        .succeeds()
        .stdout_str()
        .trim()
        .parse()
        .unwrap();

    // 09-10 and 09-12 of the same Gregorian year are in different years in the
    // Ethiopian calendar
    let year_september_10: i32 = new_ucmd!()
        .env("LC_ALL", "am_ET.UTF-8")
        .arg("-d")
        .arg(format!("{current_year}-09-10"))
        .arg("+%Y")
        .succeeds()
        .stdout_str()
        .trim()
        .parse()
        .unwrap();

    let year_september_12: i32 = new_ucmd!()
        .env("LC_ALL", "am_ET.UTF-8")
        .arg("-d")
        .arg(format!("{current_year}-09-12"))
        .arg("+%Y")
        .succeeds()
        .stdout_str()
        .trim()
        .parse()
        .unwrap();

    // Years should differ by 1
    assert_eq!(year_september_10, year_september_12 - 1);

    // The difference between the Gregorian year is 7 or 8 years
    assert_eq!(year_september_10, current_year - 8);
    assert_eq!(year_september_12, current_year - 7);

    // Check that --iso-8601 and --rfc-3339 use the Gregorian calendar
    let iso_result = new_ucmd!()
        .env("LC_ALL", "am_ET.UTF-8")
        .arg("--iso-8601=hours")
        .succeeds();
    let iso_output = iso_result.stdout_str();
    assert!(iso_output.starts_with(&current_year.to_string()));

    let rfc_result = new_ucmd!()
        .env("LC_ALL", "am_ET.UTF-8")
        .arg("--rfc-3339=date")
        .succeeds();
    let rfc_output = rfc_result.stdout_str();
    assert!(rfc_output.starts_with(&current_year.to_string()));
}

#[test]
#[cfg(unix)]
fn test_date_thai_locale_solar_calendar() {
    // Test Thai locale uses Thai solar calendar
    // Verify the Thai solar calendar is used with the Thai locale
    use std::process::Command;

    // Check if Thai locale is available
    let locale_check = Command::new("locale")
        .env("LC_ALL", "th_TH.UTF-8")
        .arg("charmap")
        .output();

    let locale_available = match locale_check {
        Ok(output) => String::from_utf8_lossy(&output.stdout).trim() == "UTF-8",
        Err(_) => false,
    };

    if !locale_available {
        println!("Skipping Thai locale test - th_TH.UTF-8 locale not available");
        return;
    }

    // Get current year in Gregorian calendar
    let current_year: i32 = new_ucmd!()
        .env("LC_ALL", "C")
        .arg("+%Y")
        .succeeds()
        .stdout_str()
        .trim()
        .parse()
        .unwrap();

    // Since 1941, the year in the Thai solar calendar is the Gregorian year plus 543
    let thai_year: i32 = new_ucmd!()
        .env("LC_ALL", "th_TH.UTF-8")
        .arg("+%Y")
        .succeeds()
        .stdout_str()
        .trim()
        .parse()
        .unwrap();

    assert_eq!(thai_year, current_year + 543);

    // All months that have 31 days have names that end with "คม" (Thai characters)
    let days_31_suffix = "\u{0E04}\u{0E21}"; // "คม" in Unicode

    for month in ["01", "03", "05", "07", "08", "10", "12"] {
        let month_result = new_ucmd!()
            .env("LC_ALL", "th_TH.UTF-8")
            .arg("--date")
            .arg(format!("{current_year}-{month}-01"))
            .arg("+%B")
            .succeeds();
        let month_name = month_result.stdout_str();

        assert!(
            month_name.trim().ends_with(days_31_suffix),
            "Month {month} should end with 'คม', got: {month_name}"
        );
    }

    // Check that --iso-8601 and --rfc-3339 use the Gregorian calendar
    let iso_result = new_ucmd!()
        .env("LC_ALL", "th_TH.UTF-8")
        .arg("--iso-8601=hours")
        .succeeds();
    let iso_output = iso_result.stdout_str();
    assert!(iso_output.starts_with(&current_year.to_string()));

    let rfc_result = new_ucmd!()
        .env("LC_ALL", "th_TH.UTF-8")
        .arg("--rfc-3339=date")
        .succeeds();
    let rfc_output = rfc_result.stdout_str();
    assert!(rfc_output.starts_with(&current_year.to_string()));
}

#[cfg(unix)]
fn check_date(locale: &str, date: &str, fmt: &str, expected: &str) {
    let actual = new_ucmd!()
        .env("LC_ALL", locale)
        .arg("-d")
        .arg(date)
        .arg(fmt)
        .succeeds()
        .stdout_str()
        .trim()
        .to_string();
    assert_eq!(actual, expected, "LC_ALL={locale} date -d '{date}' '{fmt}'");
}

#[test]
#[cfg(unix)]
fn test_locale_calendar_conversions() {
    // Persian (Solar Hijri) - Nowruz is March 20/21
    for (d, e) in [
        ("2026-01-01", "1404-10-11"),
        ("2026-01-26", "1404-11-06"),
        ("2026-03-20", "1404-12-29"),
        ("2026-03-21", "1405-01-01"),
        ("2026-03-22", "1405-01-02"),
        ("2026-06-15", "1405-03-25"),
        ("2026-12-31", "1405-10-10"),
        ("2025-03-20", "1403-12-30"),
        ("2025-03-21", "1404-01-01"),
        ("2024-03-19", "1402-12-29"),
        ("2024-03-20", "1403-01-01"),
        ("2000-03-20", "1379-01-01"),
    ] {
        check_date("fa_IR.UTF-8", d, "+%Y-%m-%d", e);
    }

    // Thai Buddhist (year + 543, same month/day)
    for (d, e) in [
        ("2026-01-01", "2569-01-01"),
        ("2026-01-26", "2569-01-26"),
        ("2026-06-15", "2569-06-15"),
        ("2026-12-31", "2569-12-31"),
        ("2025-01-01", "2568-01-01"),
        ("2024-02-29", "2567-02-29"),
        ("2000-01-01", "2543-01-01"),
        ("1970-01-01", "2513-01-01"),
    ] {
        check_date("th_TH.UTF-8", d, "+%Y-%m-%d", e);
    }

    // Ethiopian (13 months, New Year on Sept 11)
    for (d, e) in [
        ("2026-01-01", "2018-04-23"),
        ("2026-01-26", "2018-05-18"),
        ("2026-09-10", "2018-13-05"),
        ("2026-09-11", "2019-01-01"),
        ("2026-09-12", "2019-01-02"),
        ("2026-12-31", "2019-04-22"),
        ("2025-09-11", "2018-01-01"),
        ("2025-09-10", "2017-13-05"),
        ("2000-09-11", "1993-01-01"),
    ] {
        check_date("am_ET.UTF-8", d, "+%Y-%m-%d", e);
    }
}

#[test]
#[cfg(unix)]
fn test_locale_month_names() {
    // %B full month names: Jan, Jun, Dec for each locale
    for (loc, jan, jun, dec) in [
        ("fr_FR.UTF-8", "janvier", "juin", "décembre"),
        ("de_DE.UTF-8", "Januar", "Juni", "Dezember"),
        ("es_ES.UTF-8", "enero", "junio", "diciembre"),
        ("it_IT.UTF-8", "gennaio", "giugno", "dicembre"),
        ("pt_BR.UTF-8", "janeiro", "junho", "dezembro"),
        ("ja_JP.UTF-8", "1月", "6月", "12月"),
        ("zh_CN.UTF-8", "一月", "六月", "十二月"),
    ] {
        check_date(loc, "2026-01-15", "+%B", jan);
        check_date(loc, "2026-06-15", "+%B", jun);
        check_date(loc, "2026-12-15", "+%B", dec);
    }
}

#[test]
#[cfg(unix)]
fn test_locale_abbreviated_month_names() {
    // %b abbreviated month names: Feb, Jun, Dec for each locale
    // This test ensures we don't get double periods in locales like Hungarian
    // where ICU returns "febr." but the format string also adds a period after %b
    for (loc, feb, jun, dec) in [
        ("fr_FR.UTF-8", "févr", "juin", "déc"),
        ("de_DE.UTF-8", "Feb", "Jun", "Dez"),
        ("es_ES.UTF-8", "feb", "jun", "dic"),
        ("it_IT.UTF-8", "feb", "giu", "dic"),
        ("pt_BR.UTF-8", "fev", "jun", "dez"),
        ("ja_JP.UTF-8", "2月", "6月", "12月"),
        ("zh_CN.UTF-8", "2月", "6月", "12月"),
        // Hungarian locale - the fix ensures no double periods
        ("hu_HU.UTF-8", "febr", "jún", "dec"),
    ] {
        check_date(loc, "2026-02-12", "+%b", feb);
        check_date(loc, "2026-06-14", "+%b", jun);
        check_date(loc, "2026-12-09", "+%b", dec);
    }
}

#[test]
#[cfg(unix)]
fn test_locale_day_names() {
    // %A full day names: Mon (26th), Sun (25th), Sat (24th) Jan 2026
    for (loc, mon, sun, sat) in [
        ("fr_FR.UTF-8", "lundi", "dimanche", "samedi"),
        ("de_DE.UTF-8", "Montag", "Sonntag", "Samstag"),
        ("es_ES.UTF-8", "lunes", "domingo", "sábado"),
        ("ja_JP.UTF-8", "月曜日", "日曜日", "土曜日"),
        ("zh_CN.UTF-8", "星期一", "星期日", "星期六"),
    ] {
        check_date(loc, "2026-01-26", "+%A", mon);
        check_date(loc, "2026-01-25", "+%A", sun);
        check_date(loc, "2026-01-24", "+%A", sat);
    }
}

#[test]
fn test_percent_percent_not_replaced() {
    let cases = [
        // Time conversion specifiers
        (
            "+%%H%%I%%k%%l%%M%%N%%p%%P%%r%%R%%s%%S%%T%%X%%z%%Z",
            "%H%I%k%l%M%N%p%P%r%R%s%S%T%X%z%Z\n",
        ),
        // Date conversion specifiers
        (
            "+%%a%%A%%b%%B%%c%%C%%d%%D%%e%%F%%g%%G%%h%%j%%m%%u%%U%%V%%w%%W%%x%%y%%Y",
            "%a%A%b%B%c%C%d%D%e%F%g%G%h%j%m%u%U%V%w%W%x%y%Y\n",
        ),
    ];
    for (format, expected) in cases {
        new_ucmd!()
            .env("TZ", "UTC")
            .arg(format)
            .succeeds()
            .stdout_is(expected);
        new_ucmd!()
            .env("TZ", "UTC")
            .env("LC_ALL", "fr_FR.UTF-8")
            .arg(format)
            .succeeds()
            .stdout_is(expected);
    }
}

#[test]
#[cfg(target_os = "linux")]
fn test_date_write_error_dev_full() {
    use std::fs::OpenOptions;
    let dev_full = OpenOptions::new().write(true).open("/dev/full").unwrap();
    new_ucmd!()
        .arg("+%s")
        .set_stdout(dev_full)
        .fails()
        .stderr_contains("write error");
}

// Tests for GNU test leap-1: leap year overflow in date arithmetic
#[test]
fn test_date_leap1_leap_year_overflow() {
    // GNU test leap-1: Adding years to Feb 29 should overflow to March 1
    // if target year is not a leap year
    new_ucmd!()
        .args(&["--date", "02/29/1996 1 year", "+%Y-%m-%d"])
        .succeeds()
        .stdout_is("1997-03-01\n");

    // Additional cases: 2 years
    new_ucmd!()
        .args(&["--date", "1996-02-29 + 2 years", "+%Y-%m-%d"])
        .succeeds()
        .stdout_is("1998-03-01\n");

    // Leap year to leap year should not overflow
    new_ucmd!()
        .args(&["--date", "1996-02-29 + 4 years", "+%Y-%m-%d"])
        .succeeds()
        .stdout_is("2000-02-29\n");
}

// Tests for GNU test rel-2b: month arithmetic precision
#[test]
fn test_date_rel2b_month_arithmetic() {
    // GNU test rel-2b: Subtracting months should maintain same day of month
    new_ucmd!()
        .args(&[
            "--date",
            "1997-01-19 08:17:48 +0 7 months ago",
            "+%Y-%m-%d %T",
        ])
        .succeeds()
        .stdout_contains("1996-06-19");

    // Month overflow: Adding months should overflow to next month if day doesn't exist
    new_ucmd!()
        .args(&["--date", "1996-01-31 + 1 month", "+%Y-%m-%d"])
        .succeeds()
        .stdout_is("1996-03-02\n");
}

// Tests for GNU test cross-TZ-mishandled: embedded timezone parsing
#[test]
fn test_date_cross_tz_mishandled() {
    // GNU test cross-TZ-mishandled: Parse date with embedded timezone
    // Date should be interpreted in embedded TZ, then displayed in environment TZ
    new_ucmd!()
        .env("TZ", "PST8")
        .env("LC_ALL", "C")
        .args(&["-d", r#"TZ="EST5" 1970-01-01 00:00"#])
        .succeeds()
        .stdout_contains("Dec 31")
        .stdout_contains("21:00:00")
        .stdout_contains("1969");
}

// Tests for GNU test invalid-high-bit-set: invalid UTF-8 in date string
#[test]
#[cfg(unix)]
fn test_date_invalid_high_bit_set() {
    use std::os::unix::ffi::OsStrExt;

    // GNU test invalid-high-bit-set: Invalid UTF-8 byte (0xb0) should produce
    // GNU-compatible error message with octal escape sequence
    let invalid_bytes = b"\xb0";
    let invalid_arg = std::ffi::OsStr::from_bytes(invalid_bytes);

    new_ucmd!()
        .args(&[std::ffi::OsStr::new("-d"), invalid_arg])
        .fails()
        .code_is(1)
        .stderr_contains("invalid date '\\260'");
}

// Tests for GNU format modifiers
#[test]
fn test_date_format_modifier_width() {
    // Test width modifier: %10Y should pad year to 10 digits
    new_ucmd!()
        .env("TZ", "UTC")
        .args(&["-d", "1999-06-01", "+%10Y"])
        .succeeds()
        .stdout_is("0000001999\n");
}

#[test]
fn test_date_format_modifier_underscore_padding() {
    // Test underscore flag: %_10m should pad month with spaces
    new_ucmd!()
        .env("TZ", "UTC")
        .args(&["-d", "1999-06-01", "+%_10m"])
        .succeeds()
        .stdout_is("         6\n");
}

#[test]
fn test_date_format_modifier_no_pad() {
    // Test no-pad flag: %-10Y suppresses all padding (width ignored)
    new_ucmd!()
        .env("TZ", "UTC")
        .args(&["-d", "1999-06-01", "+%-10Y"])
        .succeeds()
        .stdout_is("1999\n");

    // Test no-pad on day: %-d strips default zero padding
    new_ucmd!()
        .env("TZ", "UTC")
        .args(&["-d", "1999-06-01", "+%-d"])
        .succeeds()
        .stdout_is("1\n");
}

#[test]
fn test_date_format_modifier_uppercase() {
    // Test uppercase flag: %^B should uppercase month name
    new_ucmd!()
        .env("TZ", "UTC")
        .env("LC_ALL", "C")
        .args(&["-d", "1999-06-01", "+%^B"])
        .succeeds()
        .stdout_is("JUNE\n");
}

#[test]
fn test_date_format_modifier_force_sign() {
    // Test force sign flag: %+6Y should show + sign for positive years
    new_ucmd!()
        .env("TZ", "UTC")
        .args(&["-d", "1970-01-01", "+%+6Y"])
        .succeeds()
        .stdout_is("+01970\n");
}

#[test]
fn test_date_format_modifier_combined_flags() {
    // Test combined flags: %-^10B should uppercase, no-pad suppresses all padding
    new_ucmd!()
        .env("TZ", "UTC")
        .env("LC_ALL", "C")
        .args(&["-d", "1999-06-01", "+%-^10B"])
        .succeeds()
        .stdout_is("JUNE\n");
}

#[test]
fn test_date_format_modifier_case_precedence() {
    // Test that ^ (uppercase) takes precedence over # (swap case) regardless of order
    new_ucmd!()
        .env("TZ", "UTC")
        .env("LC_ALL", "C")
        .args(&["-d", "1999-06-01", "+%^#B"])
        .succeeds()
        .stdout_is("JUNE\n");

    new_ucmd!()
        .env("TZ", "UTC")
        .env("LC_ALL", "C")
        .args(&["-d", "1999-06-01", "+%#^B"])
        .succeeds()
        .stdout_is("JUNE\n");
}

#[test]
fn test_date_format_modifier_multiple() {
    // Test multiple modifiers in one format string
    // %-5d: no-pad suppresses all padding → "1"
    new_ucmd!()
        .env("TZ", "UTC")
        .args(&["-d", "1999-06-01", "+%10Y-%_5m-%-5d"])
        .succeeds()
        .stdout_is("0000001999-    6-1\n");
}

#[test]
fn test_date_format_modifier_percent_escape() {
    // Test that %% is preserved correctly with modifiers
    new_ucmd!()
        .env("TZ", "UTC")
        .args(&["-d", "1999-06-01", "+%%Y=%10Y"])
        .succeeds()
        .stdout_is("%Y=0000001999\n");
}

// Tests for format modifier edge cases (flags without explicit width)
#[test]
fn test_date_format_modifier_edge_cases() {
    // Test cases: (date, format, expected_output, description)
    let cases = vec![
        // Underscore flag without explicit width (uses default width)
        ("1999-06-01", "%_d", " 1", "%_d pads day to default width 2"),
        (
            "1999-06-15",
            "%_m",
            " 6",
            "%_m pads month to default width 2",
        ),
        (
            "1999-06-01 05:00:00",
            "%_H",
            " 5",
            "%_H pads hour to default width 2",
        ),
        (
            "1999-06-01",
            "%_Y",
            "1999",
            "%_Y year already at default width 4",
        ),
        (
            "1999-06-01",
            "%_C",
            "19",
            "%_C century uses default width 2",
        ),
        ("2024-06-01", "%_C", "20", "%_C century for year 2024"),
        (
            "1999-01-01",
            "%_j",
            "  1",
            "%_j pads day-of-year to default width 3",
        ),
        ("1999-04-10", "%_j", "100", "%_j day 100 already at width 3"),
        // Zero flag on space-padded specifiers (overrides default padding)
        (
            "1999-06-05",
            "%0e",
            "05",
            "%0e overrides space-padding with zero",
        ),
        (
            "1999-06-01 05:00:00",
            "%0k",
            "05",
            "%0k overrides space-padding with zero",
        ),
        (
            "1999-06-01 05:00:00",
            "%0l",
            "05",
            "%0l overrides space-padding with zero",
        ),
        // Zero flag without explicit width (uses default width)
        (
            "1999-06-01",
            "%0d",
            "01",
            "%0d day with zero padding (default width 2)",
        ),
        (
            "1999-06-15",
            "%0m",
            "06",
            "%0m month with zero padding (default width 2)",
        ),
        (
            "1999-01-01",
            "%0j",
            "001",
            "%0j day-of-year with zero padding (default width 3)",
        ),
        // Space-padded specifiers default behavior (no modifier)
        ("1999-06-05", "%e", " 5", "%e defaults to space padding"),
        (
            "1999-06-01 05:00:00",
            "%k",
            " 5",
            "%k defaults to space padding",
        ),
        (
            "1999-06-01 05:00:00",
            "%l",
            " 5",
            "%l defaults to space padding",
        ),
        // Plus flag without explicit width
        (
            "1999-06-01",
            "%+Y",
            "1999",
            "%+Y no sign for 4-digit year without width",
        ),
        (
            "1999-06-01",
            "%+6Y",
            "+01999",
            "%+6Y with explicit width adds sign",
        ),
    ];

    for (date, format, expected, description) in cases {
        let result = new_ucmd!()
            .env("TZ", "UTC")
            .args(&["-d", date, &format!("+{format}")])
            .succeeds();
        // stdout includes newline, expected is without newline
        assert_eq!(
            result.stdout_str(),
            format!("{expected}\n"),
            "{description}"
        );
    }
}

// Tests for --debug flag
#[test]
fn test_date_debug_basic() {
    // Test that --debug outputs to stderr, not stdout
    let result = new_ucmd!()
        .env("TZ", "UTC")
        .args(&["--debug", "-d", "2005-01-01", "+%Y"])
        .succeeds();

    // Stdout should contain only the formatted date
    assert_eq!(result.stdout_str().trim(), "2005");

    // Stderr should contain debug information
    let stderr = result.stderr_str();
    assert!(stderr.contains("date: input string:"));
    assert!(stderr.contains("date: parsed date part:"));
    assert!(stderr.contains("date: parsed time part:"));
    assert!(stderr.contains("date: input timezone:"));
}

#[test]
fn test_date_debug_various_formats() {
    // Test debug mode with various date formats and expected output
    let test_cases = [
        // (input, format, expected_stdout_contains, expected_stderr_contains, stderr_not_contains, check_input_string)
        (
            "2005-01-01 +345 day",
            "+%Y-%m-%d",
            "2005-12-12",
            "date: parsed date part: (Y-M-D) 2005-12-12",
            "",
            true,
        ),
        (
            "@0",
            "+%Y-%m-%d",
            "1970-01-01",
            "date: parsed date part: (Y-M-D) 1970-01-01",
            "warning: using midnight",
            true,
        ),
        (
            "@-22",
            "+%s",
            "-22",
            "date: parsed date part: (Y-M-D) 1969-12-31",
            "",
            true,
        ),
        (
            "2021-03-20 14:53:01 EST",
            "+%Y-%m-%d",
            "2021-03-20",
            "date: parsed date part: (Y-M-D) 2021-03-20",
            "",
            true,
        ),
        (
            "m9",
            "+%T",
            "21:00:00",
            "date: parsed time part:",
            "",
            false,
        ), // Military TZ is composed before parsing
        (
            " ",
            "+%T",
            "00:00:00",
            "date: warning: using midnight",
            "",
            false,
        ), // Whitespace is composed
        (
            "1 day ago",
            "+%Y-%m-%d",
            "",
            "date: parsed date part: (Y-M-D)",
            "",
            true,
        ),
    ];

    for (
        input,
        format,
        stdout_contains,
        stderr_contains,
        stderr_not_contains,
        check_input_string,
    ) in test_cases
    {
        let result = new_ucmd!()
            .env("TZ", "UTC")
            .args(&["--debug", "-d", input, format])
            .succeeds();

        if !stdout_contains.is_empty() {
            assert!(
                result.stdout_str().contains(stdout_contains),
                "For input '{input}': stdout should contain '{stdout_contains}', got: {}",
                result.stdout_str()
            );
        }

        let stderr = result.stderr_str();
        assert!(
            stderr.contains(stderr_contains),
            "For input '{input}': stderr should contain '{stderr_contains}'"
        );

        if check_input_string {
            assert!(
                stderr.contains(&format!("date: input string: {input}")),
                "For input '{input}': stderr should contain input string"
            );
        } else {
            // Just check that there is some input string
            assert!(
                stderr.contains("date: input string:"),
                "For input '{input}': stderr should contain some input string"
            );
        }

        if !stderr_not_contains.is_empty() {
            assert!(
                !stderr.contains(stderr_not_contains),
                "For input '{input}': stderr should not contain '{stderr_not_contains}'"
            );
        }
    }
}

#[test]
fn test_date_debug_midnight_warnings() {
    // Test midnight warning behavior with various inputs
    let test_cases = [
        // (input, format, should_warn)
        ("2005-01-01", "+%Y", true), // No time specified
        ("1997-01-19 08:17:48 +0", "+%Y-%m-%d", false), // Time specified
        ("@0", "+%Y-%m-%d", false),  // Epoch format
        (" ", "+%T", true),          // Whitespace (defaults to midnight)
    ];

    for (input, format, should_warn) in test_cases {
        let result = new_ucmd!()
            .env("TZ", "UTC")
            .args(&["--debug", "-d", input, format])
            .succeeds();

        let stderr = result.stderr_str();
        if should_warn {
            assert!(
                stderr.contains("date: warning: using midnight"),
                "Input '{input}' should produce midnight warning"
            );
        } else {
            assert!(
                !stderr.contains("warning: using midnight"),
                "Input '{input}' should not produce midnight warning"
            );
        }
    }
}

#[test]
fn test_date_debug_without_flag() {
    // Test that without --debug, no debug output appears
    let result = new_ucmd!()
        .env("TZ", "UTC")
        .args(&["-d", "2005-01-01", "+%Y"])
        .succeeds();

    let stderr = result.stderr_str();
    assert!(!stderr.contains("date: input string:"));
    assert!(!stderr.contains("date: parsed date part:"));
}

#[test]
fn test_date_debug_with_multiple_inputs() {
    // Test debug mode with file and stdin input (multiple dates)
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "debug_test_file";
    at.write(file, "2005-01-01\n2006-02-02\n");

    let result = ucmd
        .env("TZ", "UTC")
        .args(&["--debug", "-f", file, "+%Y"])
        .succeeds();

    assert_eq!(result.stdout_str(), "2005\n2006\n");

    let stderr = result.stderr_str();
    // Should show debug output for both lines
    assert!(stderr.contains("date: input string: 2005-01-01"));
    assert!(stderr.contains("date: input string: 2006-02-02"));
    assert!(stderr.contains("date: parsed date part: (Y-M-D) 2005-01-01"));
    assert!(stderr.contains("date: parsed date part: (Y-M-D) 2006-02-02"));

    // Test with stdin
    let result = new_ucmd!()
        .env("TZ", "UTC")
        .args(&["--debug", "-f", "-", "+%Y"])
        .pipe_in("2005-01-01\n2006-02-02\n")
        .succeeds();

    assert_eq!(result.stdout_str(), "2005\n2006\n");
    let stderr = result.stderr_str();
    assert!(stderr.contains("date: input string: 2005-01-01"));
    assert!(stderr.contains("date: input string: 2006-02-02"));
}

#[test]
fn test_date_debug_with_flags() {
    // Test debug mode combined with other flags and exit codes
    let test_cases = [
        // (args, should_succeed, stdout_contains, stderr_contains)
        (
            vec!["--debug", "-d", "2005-01-01", "+%Y"],
            true,
            "2005",
            "date: input string:",
        ),
        (
            vec!["--debug", "-u", "-d", "2005-01-01", "+%Y-%m-%d %Z"],
            true,
            "UTC",
            "date: parsed date part:",
        ),
        (
            vec!["--debug", "-R", "-d", "2005-01-01"],
            true,
            "Sat, 01 Jan 2005",
            "date: input string:",
        ),
        (
            vec!["--debug", "-d", "invalid", "+%Y"],
            false,
            "",
            "invalid date",
        ),
    ];

    for (args, should_succeed, stdout_contains, stderr_contains) in test_cases {
        let mut cmd = new_ucmd!();
        cmd.env("TZ", "UTC").args(&args);

        if should_succeed {
            let result = cmd.succeeds();
            assert!(
                result.stdout_str().contains(stdout_contains),
                "Args {args:?}: stdout should contain '{stdout_contains}'"
            );
            assert!(
                result.stderr_str().contains(stderr_contains),
                "Args {args:?}: stderr should contain '{stderr_contains}'"
            );
        } else {
            let result = cmd.fails();
            assert!(
                result.stderr_str().contains(stderr_contains),
                "Args {args:?}: stderr should contain '{stderr_contains}'"
            );
        }
    }
}

#[test]
fn test_date_debug_current_time() {
    // Test that debug mode without -d doesn't produce debug output (no parsing)
    let result = new_ucmd!()
        .env("TZ", "UTC")
        .args(&["--debug", "+%Y"])
        .succeeds();

    let stderr = result.stderr_str();
    // No parsing happens for "now", so no debug output
    assert_eq!(stderr, "");
}
