// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore parenb parmrk ixany iuclc onlcr icanon noflsh econl igpar ispeed ospeed NCCS nonhex gstty

use uutests::util::{expected_result, pty_path};
use uutests::{at_and_ts, new_ucmd, unwrap_or_return};

/// Normalize stderr by replacing the full binary path with just the utility name
/// This allows comparison between GNU (which shows "stty" or "gstty") and ours (which shows full path)
fn normalize_stderr(stderr: &str) -> String {
    // Replace patterns like "Try 'gstty --help'" or "Try '/path/to/stty --help'" with "Try 'stty --help'"
    let re = regex::Regex::new(r"Try '[^']*(?:g)?stty --help'").unwrap();
    re.replace_all(stderr, "Try 'stty --help'").to_string()
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
#[cfg(unix)]
fn test_row_column_hex_octal() {
    let (path, _controller, _replica) = pty_path();
    let (_at, ts) = at_and_ts!();

    // Test various numeric formats: hex (0x1E), octal (036), uppercase hex (0X1E), decimal (30), and zero
    let test_cases = [
        ("rows", "0x1E"),  // hexadecimal = 30
        ("rows", "036"),   // octal = 30
        ("cols", "0X1E"),  // uppercase hex = 30
        ("columns", "30"), // decimal = 30
        ("rows", "0"),     // zero (not octal prefix)
    ];

    for (setting, value) in test_cases {
        let result = ts.ucmd().args(&["--file", &path, setting, value]).run();
        let exp_result =
            unwrap_or_return!(expected_result(&ts, &["--file", &path, setting, value]));
        let normalized_stderr = normalize_stderr(result.stderr_str());

        result
            .stdout_is(exp_result.stdout_str())
            .code_is(exp_result.code());
        assert_eq!(normalized_stderr, exp_result.stderr_str());
    }
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

    // Generate valid saved state from the actual terminal
    let saved = unwrap_or_return!(expected_result(&ts, &["-g", "--file", &path])).stdout_move_str();
    let saved = saved.trim();

    let result = ts.ucmd().args(&["--file", &path, saved]).run();

    result.success().no_stderr();

    let exp_result = unwrap_or_return!(expected_result(&ts, &["--file", &path, saved]));
    let normalized_stderr = normalize_stderr(result.stderr_str());
    result
        .stdout_is(exp_result.stdout_str())
        .code_is(exp_result.code());
    assert_eq!(normalized_stderr, exp_result.stderr_str());
}

#[test]
#[cfg(unix)]
fn test_saved_state_invalid_formats() {
    let (path, _controller, _replica) = pty_path();
    let (_at, ts) = at_and_ts!();

    let num_cc = nix::libc::NCCS;

    // Build test strings with platform-specific counts
    let cc_zeros = vec!["0"; num_cc].join(":");
    let cc_with_invalid = if num_cc > 0 {
        let mut parts = vec!["1c"; num_cc];
        parts[0] = "100"; // First control char > 255
        parts.join(":")
    } else {
        String::new()
    };
    let cc_with_space = if num_cc > 0 {
        let mut parts = vec!["1c"; num_cc];
        parts[0] = "1c "; // Space in hex
        parts.join(":")
    } else {
        String::new()
    };
    let cc_with_nonhex = if num_cc > 0 {
        let mut parts = vec!["1c"; num_cc];
        parts[0] = "xyz"; // Non-hex
        parts.join(":")
    } else {
        String::new()
    };
    let cc_with_empty = if num_cc > 0 {
        let mut parts = vec!["1c"; num_cc];
        parts[0] = ""; // Empty
        parts.join(":")
    } else {
        String::new()
    };

    // Cannot test single value since it would be interpreted as baud rate
    let invalid_states = vec![
        "500:5:4bf".to_string(),                        // fewer than expected parts
        "500:5:4bf:8a3b".to_string(),                   // only 4 parts
        format!("500:5:{}:8a3b:{}", cc_zeros, "extra"), // too many parts
        format!("500::4bf:8a3b:{}", cc_zeros),          // empty hex value in flags
        format!("500:5:4bf:8a3b:{}", cc_with_empty),    // empty hex value in cc
        format!("500:5:4bf:8a3b:{}", cc_with_nonhex),   // non-hex characters
        format!("500:5:4bf:8a3b:{}", cc_with_space),    // space in hex value
        format!("500:5:4bf:8a3b:{}", cc_with_invalid),  // control char > 255
    ];

    for state in &invalid_states {
        let result = ts.ucmd().args(&["--file", &path, state]).run();

        result.failure().stderr_contains("invalid argument");

        let exp_result = unwrap_or_return!(expected_result(&ts, &["--file", &path, state]));
        let normalized_stderr = normalize_stderr(result.stderr_str());
        let exp_normalized_stderr = normalize_stderr(exp_result.stderr_str());
        result
            .stdout_is(exp_result.stdout_str())
            .code_is(exp_result.code());
        assert_eq!(normalized_stderr, exp_normalized_stderr);
    }
}

#[test]
#[cfg(unix)]
#[ignore = "Fails because the implementation of print state is not correctly printing flags on certain platforms"]
fn test_saved_state_with_control_chars() {
    let (path, _controller, _replica) = pty_path();
    let (_at, ts) = at_and_ts!();

    // Build a valid saved state with platform-specific number of control characters
    let num_cc = nix::libc::NCCS;
    let cc_values: Vec<String> = (1..=num_cc).map(|_| format!("{:x}", 0)).collect();
    let saved_state = format!("500:5:4bf:8a3b:{}", cc_values.join(":"));

    ts.ucmd().args(&["--file", &path, &saved_state]).succeeds();

    let result = ts.ucmd().args(&["-g", "--file", &path]).run();

    result.success().stdout_contains(":");

    let exp_result = unwrap_or_return!(expected_result(&ts, &["-g", "--file", &path]));
    result
        .stdout_is(exp_result.stdout_str())
        .stderr_is(exp_result.stderr_str())
        .code_is(exp_result.code());
}
