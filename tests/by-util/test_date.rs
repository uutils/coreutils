// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use chrono::{DateTime, Datelike, Duration, NaiveTime, Utc}; // spell-checker:disable-line
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
fn test_date_email() {
    for param in ["--rfc-email", "--rfc-e", "-R"] {
        new_ucmd!().arg(param).succeeds();
    }
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
    for param in ["--universal", "--utc", "--uni", "--u"] {
        new_ucmd!().arg(param).succeeds();
    }
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
/// TODO: expected to fail currently; change to `succeeds()` when required.
fn test_date_set_valid_2() {
    if geteuid() == 0 {
        let result = new_ucmd!()
            .arg("--set")
            .arg("Sat 20 Mar 2021 14:53:01 AWST") // spell-checker:disable-line
            .fails();
        result.no_stdout();
        assert!(result.stderr_str().starts_with("date: invalid date "));
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
#[cfg(all(unix, not(target_os = "macos")))]
/// TODO: expected to fail currently; change to `succeeds()` when required.
fn test_date_set_valid_3() {
    if geteuid() == 0 {
        let result = new_ucmd!()
            .arg("--set")
            .arg("Sat 20 Mar 2021 14:53:01") // Local timezone
            .fails();
        result.no_stdout();
        assert!(result.stderr_str().starts_with("date: invalid date "));
    }
}

#[test]
#[cfg(all(unix, not(target_os = "macos")))]
/// TODO: expected to fail currently; change to `succeeds()` when required.
fn test_date_set_valid_4() {
    if geteuid() == 0 {
        let result = new_ucmd!()
            .arg("--set")
            .arg("2020-03-11 21:45:00") // Local timezone
            .fails();
        result.no_stdout();
        assert!(result.stderr_str().starts_with("date: invalid date "));
    }
}

#[test]
fn test_invalid_format_string() {
    let result = new_ucmd!().arg("+%!").fails();
    result.no_stdout();
    assert!(result.stderr_str().starts_with("date: invalid format "));
}

#[test]
fn test_capitalized_numeric_time_zone() {
    // %z     +hhmm numeric time zone (e.g., -0400)
    // # is supposed to capitalize, which makes little sense here, but chrono crashes
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
        ("-1 hour", Duration::hours(1)),
        ("-1 hours", Duration::hours(1)),
        ("-1 day", Duration::days(1)),
        ("-2 weeks", Duration::weeks(2)),
    ];
    for (date_format, offset) in data_formats {
        new_ucmd!()
            .arg("-d")
            .arg(date_format)
            .arg("--rfc-3339=seconds")
            .succeeds()
            .stdout_str_check(|out| {
                let date = DateTime::parse_from_rfc3339(out.trim()).unwrap();

                // Is the resulting date roughly what is expected?
                let expected_date = Utc::now() - offset;
                (date.to_utc() - expected_date).abs() < Duration::minutes(10)
            });
    }
}

#[test]
fn test_relative_weekdays() {
    // Truncate time component to midnight
    let today = Utc::now().with_time(NaiveTime::MIN).unwrap();
    // Loop through each day of the week, starting with today
    for offset in 0..7 {
        for direction in ["last", "this", "next"] {
            let weekday = (today + Duration::days(offset))
                .weekday()
                .to_string()
                .to_lowercase();
            new_ucmd!()
                .arg("-d")
                .arg(format!("{} {}", direction, weekday))
                .arg("--rfc-3339=seconds")
                .arg("--utc")
                .succeeds()
                .stdout_str_check(|out| {
                    let result = DateTime::parse_from_rfc3339(out.trim()).unwrap().to_utc();
                    let expected = match (direction, offset) {
                        ("last", _) => today - Duration::days(7 - offset),
                        ("this", 0) => today,
                        ("next", 0) => today + Duration::days(7),
                        _ => today + Duration::days(offset),
                    };
                    result == expected
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

#[test]
fn test_date_empty_tz() {
    new_ucmd!()
        .env("TZ", "")
        .arg("+%Z")
        .succeeds()
        .stdout_only("UTC\n");
}
