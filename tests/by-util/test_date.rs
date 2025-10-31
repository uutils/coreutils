// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// spell-checker: ignore: AEDT AEST EEST NZDT NZST Kolkata Iseconds

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
    for param in ["--rfc-email", "--rfc-e", "-R", "--rfc-2822", "--rfc-822"] {
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
                .arg(format!("{direction} {weekday}"))
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
    let today = Utc::now().date_naive();
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
