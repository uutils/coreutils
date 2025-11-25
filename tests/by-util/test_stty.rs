// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore parenb parmrk ixany iuclc onlcr icanon noflsh econl igpar ispeed ospeed

use uutests::util::{expected_result, pty_path};
use uutests::{at_and_ts, new_ucmd, unwrap_or_return};

/// Normalize stderr by replacing the full binary path with just the utility name
/// This allows comparison between GNU (which shows "stty") and ours (which shows full path)
fn normalize_stderr(stderr: &str, util_name: &str) -> String {
    // Replace patterns like "Try '/path/to/binary util_name --help'" with "Try 'util_name --help'"
    let re = regex::Regex::new(&format!(r"Try '[^']*{} --help'", util_name)).unwrap();
    re.replace_all(stderr, &format!("Try '{} --help'", util_name))
        .to_string()
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
#[cfg(unix)]
fn test_basic() {
    let (path, _controller, _replica) = pty_path();
    new_ucmd!()
        .args(&["--file", &path])
        .succeeds()
        .stdout_contains("speed");
}

#[test]
#[cfg(unix)]
fn test_all_flag() {
    let (path, _controller, _replica) = pty_path();
    let result = new_ucmd!().args(&["--all", "--file", &path]).succeeds();

    for flag in ["parenb", "parmrk", "ixany", "onlcr", "icanon", "noflsh"] {
        result.stdout_contains(flag);
    }
}

#[test]
#[cfg(unix)]
fn test_sane() {
    let (path, _controller, _replica) = pty_path();

    new_ucmd!()
        .args(&["--file", &path, "intr", "^A"])
        .succeeds();
    new_ucmd!()
        .args(&["--file", &path])
        .succeeds()
        .stdout_contains("intr = ^A");
    new_ucmd!().args(&["--file", &path, "sane"]).succeeds();
    new_ucmd!()
        .args(&["--file", &path])
        .succeeds()
        .stdout_str_check(|s| !s.contains("intr = ^A"));
}

#[test]
fn save_and_setting() {
    new_ucmd!()
        .args(&["--save", "nl0"])
        .fails()
        .stderr_contains("when specifying an output style, modes may not be set");
}

#[test]
fn all_and_setting() {
    new_ucmd!()
        .args(&["--all", "nl0"])
        .fails()
        .stderr_contains("when specifying an output style, modes may not be set");
}

#[test]
fn all_and_print_setting() {
    new_ucmd!()
        .args(&["--all", "size"])
        .fails()
        .stderr_contains("when specifying an output style, modes may not be set");
}

#[test]
fn save_and_all() {
    new_ucmd!()
        .args(&["--save", "--all"])
        .fails()
        .stderr_contains(
            "the options for verbose and stty-readable output styles are mutually exclusive",
        );

    new_ucmd!()
        .args(&["--all", "--save"])
        .fails()
        .stderr_contains(
            "the options for verbose and stty-readable output styles are mutually exclusive",
        );
}

#[test]
fn no_mapping() {
    new_ucmd!()
        .args(&["intr"])
        .fails()
        .stderr_contains("missing argument to 'intr'");
}

#[test]
fn invalid_mapping() {
    new_ucmd!()
        .args(&["intr", "cc"])
        .fails()
        .stderr_contains("invalid integer argument: 'cc'");

    new_ucmd!()
        .args(&["intr", "256"])
        .fails()
        .stderr_contains("invalid integer argument: '256': Value too large for defined data type");

    new_ucmd!()
        .args(&["intr", "0x100"])
        .fails()
        .stderr_contains(
            "invalid integer argument: '0x100': Value too large for defined data type",
        );

    new_ucmd!()
        .args(&["intr", "0400"])
        .fails()
        .stderr_contains("invalid integer argument: '0400': Value too large for defined data type");
}

#[test]
fn invalid_setting() {
    new_ucmd!()
        .args(&["-econl"])
        .fails()
        .stderr_contains("invalid argument '-econl'");

    new_ucmd!()
        .args(&["igpar"])
        .fails()
        .stderr_contains("invalid argument 'igpar'");
}

#[test]
fn invalid_baud_setting() {
    #[cfg(not(any(
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "ios",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd"
    )))]
    new_ucmd!()
        .args(&["100"])
        .fails()
        .stderr_contains("invalid argument '100'");

    new_ucmd!()
        .args(&["-1"])
        .fails()
        .stderr_contains("invalid argument '-1'");

    new_ucmd!()
        .args(&["ispeed"])
        .fails()
        .stderr_contains("missing argument to 'ispeed'");

    new_ucmd!()
        .args(&["ospeed"])
        .fails()
        .stderr_contains("missing argument to 'ospeed'");

    #[cfg(not(any(
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "ios",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd"
    )))]
    new_ucmd!()
        .args(&["ispeed", "995"])
        .fails()
        .stderr_contains("invalid ispeed '995'");

    #[cfg(not(any(
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "ios",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd"
    )))]
    new_ucmd!()
        .args(&["ospeed", "995"])
        .fails()
        .stderr_contains("invalid ospeed '995'");

    for speed in &[
        "9599..", "9600..", "9600.5.", "9600.50.", "9600.0.", "++9600", "0x2580", "96E2", "9600,0",
        "9600.0 ",
    ] {
        new_ucmd!().args(&["ispeed", speed]).fails();
    }
}

#[test]
#[cfg(unix)]
fn valid_baud_formats() {
    let (path, _controller, _replica) = pty_path();
    for speed in &["  +9600", "9600.49", "9600.50", "9599.51", "  9600."] {
        new_ucmd!()
            .args(&["--file", &path, "ispeed", speed])
            .succeeds();
    }
}

#[test]
#[ignore = "Fails because cargo test does not run in a tty"]
fn set_mapping() {
    new_ucmd!().args(&["intr", "'"]).succeeds();
    new_ucmd!()
        .args(&["--all"])
        .succeeds()
        .stdout_contains("intr = '");

    new_ucmd!().args(&["intr", "undef"]).succeeds();
    new_ucmd!()
        .args(&["--all"])
        .succeeds()
        .stdout_contains("intr = <undef>");

    new_ucmd!().args(&["intr", "^-"]).succeeds();
    new_ucmd!()
        .args(&["--all"])
        .succeeds()
        .stdout_contains("intr = <undef>");

    new_ucmd!().args(&["intr", ""]).succeeds();
    new_ucmd!()
        .args(&["--all"])
        .succeeds()
        .stdout_contains("intr = <undef>");

    new_ucmd!().args(&["intr", "^C"]).succeeds();
    new_ucmd!()
        .args(&["--all"])
        .succeeds()
        .stdout_contains("intr = ^C");
}

#[test]
fn row_column_sizes() {
    new_ucmd!()
        .args(&["rows", "-1"])
        .fails()
        .stderr_contains("invalid integer argument: '-1'");

    new_ucmd!()
        .args(&["columns", "-1"])
        .fails()
        .stderr_contains("invalid integer argument: '-1'");

    // overflow the u32 used for row/col counts
    new_ucmd!()
        .args(&["cols", "4294967296"])
        .fails()
        .stderr_contains("invalid integer argument: '4294967296'");

    new_ucmd!()
        .args(&["rows", ""])
        .fails()
        .stderr_contains("invalid integer argument: ''");

    new_ucmd!()
        .args(&["columns"])
        .fails()
        .stderr_contains("missing argument to 'columns'");

    new_ucmd!()
        .args(&["rows"])
        .fails()
        .stderr_contains("missing argument to 'rows'");
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn line() {
    new_ucmd!()
        .args(&["line"])
        .fails()
        .stderr_contains("missing argument to 'line'");

    new_ucmd!()
        .args(&["line", "-1"])
        .fails()
        .stderr_contains("invalid integer argument: '-1'");

    new_ucmd!()
        .args(&["line", "256"])
        .fails()
        .stderr_contains("invalid integer argument: '256'");
}

#[test]
fn min_and_time() {
    new_ucmd!()
        .args(&["min"])
        .fails()
        .stderr_contains("missing argument to 'min'");

    new_ucmd!()
        .args(&["time"])
        .fails()
        .stderr_contains("missing argument to 'time'");

    new_ucmd!()
        .args(&["min", "-1"])
        .fails()
        .stderr_contains("invalid integer argument: '-1'");

    new_ucmd!()
        .args(&["time", "-1"])
        .fails()
        .stderr_contains("invalid integer argument: '-1'");

    new_ucmd!()
        .args(&["min", "256"])
        .fails()
        .stderr_contains("invalid integer argument: '256': Value too large for defined data type");

    new_ucmd!()
        .args(&["time", "256"])
        .fails()
        .stderr_contains("invalid integer argument: '256': Value too large for defined data type");
}

#[test]
fn non_negatable_combo() {
    new_ucmd!()
        .args(&["-dec"])
        .fails()
        .stderr_contains("invalid argument '-dec'");
    new_ucmd!()
        .args(&["-crt"])
        .fails()
        .stderr_contains("invalid argument '-crt'");
    new_ucmd!()
        .args(&["-ek"])
        .fails()
        .stderr_contains("invalid argument '-ek'");
}

// Tests for saved state parsing and restoration
#[test]
#[cfg(unix)]
fn test_save_and_restore() {
    let (path, _controller, _replica) = pty_path();
    let saved = new_ucmd!()
        .args(&["--save", "--file", &path])
        .succeeds()
        .stdout_move_str();

    let saved = saved.trim();
    assert!(saved.contains(':'));

    new_ucmd!().args(&["--file", &path, saved]).succeeds();
}

#[test]
#[cfg(unix)]
fn test_save_with_g_flag() {
    let (path, _controller, _replica) = pty_path();
    let saved = new_ucmd!()
        .args(&["-g", "--file", &path])
        .succeeds()
        .stdout_move_str();

    let saved = saved.trim();
    assert!(saved.contains(':'));

    new_ucmd!().args(&["--file", &path, saved]).succeeds();
}

#[test]
#[cfg(unix)]
fn test_save_restore_after_change() {
    let (path, _controller, _replica) = pty_path();
    let saved = new_ucmd!()
        .args(&["--save", "--file", &path])
        .succeeds()
        .stdout_move_str();

    let saved = saved.trim();

    new_ucmd!()
        .args(&["--file", &path, "intr", "^A"])
        .succeeds();

    new_ucmd!().args(&["--file", &path, saved]).succeeds();

    new_ucmd!()
        .args(&["--file", &path])
        .succeeds()
        .stdout_str_check(|s| !s.contains("intr = ^A"));
}

// These tests both validate what we expect each input to return and their error codes
// and also use the GNU coreutils results to validate our results match expectations
#[test]
#[cfg(unix)]
fn test_saved_state_valid_formats() {
    let (path, _controller, _replica) = pty_path();
    let (_at, ts) = at_and_ts!();

    let valid_states = [
        "500:5:4bf:8a3b:3:1c:7f:15:4:0:1:0:11:13:1a:0:12:f:17:16:0:0:0:0:0:0:0:0:0:0:0:0:0:0:0:0", // 36 parts (4 flags + 32 control chars)
        "500:5:4BF:8A3B:3:1c:7f:15:4:0:1:0:11:13:1a:0:12:f:17:16:0:0:0:0:0:0:0:0:0:0:0:0:0:0:0:0", // uppercase hex
        "500:5:4bF:8a3B:3:1C:7F:15:4:0:1:0:11:13:1A:0:12:F:17:16:0:0:0:0:0:0:0:0:0:0:0:0:0:0:0:0", // mixed case
        "0500:05:04bf:8a3b:03:1c:7f:15:4:0:1:0:11:13:1a:0:12:f:17:16:0:0:0:0:0:0:0:0:0:0:0:0:0:0:0:0", // leading zeros
    ];

    for state in &valid_states {
        let result = ts.ucmd().args(&["--file", &path, state]).run();

        result.success().no_stderr();

        let exp_result = unwrap_or_return!(expected_result(&ts, &["--file", &path, state]));
        let normalized_stderr = normalize_stderr(result.stderr_str(), "stty");
        result
            .stdout_is(exp_result.stdout_str())
            .code_is(exp_result.code());
        assert_eq!(normalized_stderr, exp_result.stderr_str());
    }
}

#[test]
#[cfg(unix)]
fn test_saved_state_invalid_formats() {
    let (path, _controller, _replica) = pty_path();
    let (_at, ts) = at_and_ts!();

    let invalid_states = [
        "500:5:4bf",                                         // fewer than 36 parts (3 parts)
        "500",                                               // only 1 part
        "500:5:4bf:8a3b",                                    // only 4 parts (not 36)
        "500:5:4bf:8a3b:3:1c:7f:15:4:11:0:13:1a:12:17:16:f", // only 17 parts
        "500::4bf:8a3b:3:1c:7f:15:4:0:1:0:11:13:1a:0:12:f:17:16:0:0:0:0:0:0:0:0:0:0:0:0:0:0:0:0", // empty hex value
        "500:5:xyz:8a3b:3:1c:7f:15:4:0:1:0:11:13:1a:0:12:f:17:16:0:0:0:0:0:0:0:0:0:0:0:0:0:0:0:0", // non-hex characters
        "500:5:4bf :8a3b:3:1c:7f:15:4:0:1:0:11:13:1a:0:12:f:17:16:0:0:0:0:0:0:0:0:0:0:0:0:0:0:0:0", // space in hex value
        "500:5:4bf:8a3b:100:1c:7f:15:4:0:1:0:11:13:1a:0:12:f:17:16:0:0:0:0:0:0:0:0:0:0:0:0:0:0:0:0", // control char > 255
        "500:5:4bf:8a3b:3:1c:7f:15:4:0:1:0:11:13:1a:0:12:f:17:16:0:0:0:0:0:0:0:0:0:0:0:0:0:0:0:0:extra", // 37 parts
    ];

    for state in &invalid_states {
        let result = ts.ucmd().args(&["--file", &path, state]).run();

        result.failure().stderr_contains("invalid argument");

        let exp_result = unwrap_or_return!(expected_result(&ts, &["--file", &path, state]));
        let normalized_stderr = normalize_stderr(result.stderr_str(), "stty");
        result
            .stdout_is(exp_result.stdout_str())
            .code_is(exp_result.code());
        assert_eq!(normalized_stderr, exp_result.stderr_str());
    }
}

#[test]
#[cfg(unix)]
fn test_saved_state_with_control_chars() {
    let (path, _controller, _replica) = pty_path();
    let (_at, ts) = at_and_ts!();

    ts.ucmd()
        .args(&[
            "--file",
            &path,
            "500:5:4bf:8a3b:1:2:3:4:5:6:7:8:9:a:b:c:d:e:f:10:11:12:13:14:15:16:17:18:19:1a:1b:1c:1d:1e:1f:20",
        ])
        .succeeds();

    let result = ts.ucmd().args(&["-g", "--file", &path]).run();

    result.success().stdout_contains(":");

    let exp_result = unwrap_or_return!(expected_result(&ts, &["-g", "--file", &path]));
    result
        .stdout_is(exp_result.stdout_str())
        .stderr_is(exp_result.stderr_str())
        .code_is(exp_result.code());
}
