// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore parenb parmrk ixany iuclc onlcr ofdel icanon noflsh econl igpar ispeed ospeed NCCS nonhex gstty notachar cbreak evenp oddp CSIZE

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
    new_ucmd!()
        .arg("--definitely-invalid")
        .fails_with_code(1)
        .stderr_contains("invalid argument")
        .stderr_contains("--definitely-invalid");
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
        ("rows", "0x1e"),  // lowercase hexadecimal = 30
        ("rows", "0X1e"),  // upper and lowercase hexadecimal = 30
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

#[test]
fn help_output() {
    new_ucmd!()
        .arg("--help")
        .succeeds()
        .stdout_contains("Usage:")
        .stdout_contains("stty");
}

#[test]
fn version_output() {
    new_ucmd!()
        .arg("--version")
        .succeeds()
        .stdout_contains("stty");
}

#[test]
fn invalid_control_char_names() {
    // Test invalid control character names
    new_ucmd!()
        .args(&["notachar", "^C"])
        .fails()
        .stderr_contains("invalid argument 'notachar'");
}

#[test]
fn control_char_overflow_hex() {
    // Test hex overflow for control characters
    new_ucmd!()
        .args(&["erase", "0xFFF"])
        .fails()
        .stderr_contains("Value too large for defined data type");
}

#[test]
fn control_char_overflow_octal() {
    // Test octal overflow for control characters
    new_ucmd!()
        .args(&["kill", "0777"])
        .fails()
        .stderr_contains("Value too large for defined data type");
}

#[test]
fn multiple_invalid_args() {
    // Test multiple invalid arguments
    new_ucmd!()
        .args(&["invalid1", "invalid2"])
        .fails()
        .stderr_contains("invalid argument");
}

#[test]
#[ignore = "Fails because cargo test does not run in a tty"]
fn negatable_combo_settings() {
    // These should fail without TTY but validate the argument parsing
    // Testing that negatable combos are recognized (even if they fail later)
    new_ucmd!().args(&["-cbreak"]).fails();

    new_ucmd!().args(&["-evenp"]).fails();

    new_ucmd!().args(&["-oddp"]).fails();
}

#[test]
fn grouped_flag_removal() {
    // Test that removing a grouped flag is invalid
    // cs7 is part of CSIZE group, removing it should fail
    new_ucmd!()
        .args(&["-cs7"])
        .fails()
        .stderr_contains("invalid argument '-cs7'");

    new_ucmd!()
        .args(&["-cs8"])
        .fails()
        .stderr_contains("invalid argument '-cs8'");
}

#[test]
#[ignore = "Fails because cargo test does not run in a tty"]
fn baud_rate_validation() {
    // Test various baud rate formats
    #[cfg(any(
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "ios",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        // BSD accepts numeric baud rates
        new_ucmd!().args(&["9600"]).fails(); // Fails due to no TTY, but validates parsing
    }

    // Test ispeed/ospeed with valid baud rates
    new_ucmd!().args(&["ispeed", "9600"]).fails(); // Fails due to no TTY
    new_ucmd!().args(&["ospeed", "115200"]).fails(); // Fails due to no TTY
}

#[test]
#[ignore = "Fails because cargo test does not run in a tty"]
fn combination_setting_validation() {
    // Test that combination settings are recognized
    new_ucmd!().args(&["sane"]).fails(); // Fails due to no TTY, but validates parsing
    new_ucmd!().args(&["raw"]).fails();
    new_ucmd!().args(&["cooked"]).fails();
    new_ucmd!().args(&["cbreak"]).fails();
}

#[test]
#[ignore = "Fails because cargo test does not run in a tty"]
fn control_char_hat_notation() {
    // Test various hat notation formats
    new_ucmd!().args(&["intr", "^?"]).fails(); // Fails due to no TTY
    new_ucmd!().args(&["quit", "^\\"]).fails();
    new_ucmd!().args(&["erase", "^H"]).fails();
}

#[test]
#[ignore = "Fails because cargo test does not run in a tty"]
fn special_settings() {
    // Test special settings that require arguments
    new_ucmd!().args(&["speed"]).fails(); // Fails due to no TTY but validates it's recognized

    new_ucmd!().args(&["size"]).fails(); // Fails due to no TTY but validates it's recognized
}

#[test]
fn file_argument() {
    // Test --file argument with non-existent file
    new_ucmd!()
        .args(&["--file", "/nonexistent/device"])
        .fails()
        .stderr_contains("No such file or directory");
}

#[test]
fn conflicting_print_modes() {
    // Test more conflicting option combinations
    new_ucmd!()
        .args(&["--save", "speed"])
        .fails()
        .stderr_contains("when specifying an output style, modes may not be set");

    new_ucmd!()
        .args(&["--all", "speed"])
        .fails()
        .stderr_contains("when specifying an output style, modes may not be set");
}

// Additional integration tests to increase coverage

#[test]
#[cfg(unix)]
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_save_format() {
    // Test --save flag outputs settings in save format
    let result = new_ucmd!()
        .terminal_simulation(true)
        .args(&["--save"])
        .succeeds();
    // Save format should contain colon-separated fields
    result.stdout_contains(":");
    // Should contain speed information
    let stdout = result.stdout_str();
    assert!(
        stdout.split(':').count() > 1,
        "Save format should have multiple colon-separated fields"
    );
}

#[test]
#[cfg(unix)]
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_set_control_flags() {
    // Test setting parenb flag and verify it's set
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["parenb"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds()
        .stdout_contains("parenb");

    // Test unsetting parenb flag and verify it's unset
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["-parenb"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds()
        .stdout_contains("-parenb");

    // Test setting parodd flag
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["parodd"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds()
        .stdout_contains("parodd");

    // Test setting cstopb flag
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["cstopb"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds()
        .stdout_contains("cstopb");
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
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_set_input_flags() {
    // Test setting ignbrk flag and verify
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["ignbrk"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds()
        .stdout_contains("ignbrk");

    // Test setting brkint flag and verify
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["brkint"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds()
        .stdout_contains("brkint");

    // Test setting ignpar flag and verify
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["ignpar"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds()
        .stdout_contains("ignpar");
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
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_set_output_flags() {
    // Test setting opost flag and verify
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["opost"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds()
        .stdout_contains("opost");

    // Test unsetting opost flag and verify
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["-opost"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds()
        .stdout_contains("-opost");

    // Test setting onlcr flag and verify
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["onlcr"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds()
        .stdout_contains("onlcr");
}

#[test]
#[cfg(unix)]
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_set_local_flags() {
    // Test setting isig flag and verify
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["isig"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds()
        .stdout_contains("isig");

    // Test setting icanon flag and verify
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["icanon"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds()
        .stdout_contains("icanon");

    // Test setting echo flag and verify
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["echo"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds()
        .stdout_contains("echo");
}

#[test]
#[cfg(unix)]
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_combo_cbreak() {
    // Test cbreak combination setting - should disable icanon
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["cbreak"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds()
        .stdout_contains("-icanon");

    // Test -cbreak should enable icanon
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["-cbreak"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds()
        .stdout_contains("icanon");
}

#[test]
#[cfg(unix)]
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_combo_nl() {
    // Test nl combination setting - should disable icrnl and onlcr
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["nl"])
        .succeeds();
    let result = new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds();
    result.stdout_contains("-icrnl");
    result.stdout_contains("-onlcr");
}

#[test]
#[cfg(unix)]
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_combo_ek() {
    // Test ek combination setting (erase and kill) - should set erase and kill to defaults
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["ek"])
        .succeeds();
    let result = new_ucmd!().terminal_simulation(true).succeeds();
    // Should show erase and kill characters
    result.stdout_contains("erase");
    result.stdout_contains("kill");
}

#[test]
#[cfg(unix)]
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_combo_litout() {
    // Test litout combination setting - should disable parenb, istrip, opost
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["litout"])
        .succeeds();
    let result = new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds();
    result.stdout_contains("-parenb");
    result.stdout_contains("-istrip");
    result.stdout_contains("-opost");
}

#[test]
#[cfg(unix)]
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_combo_pass8() {
    // Test pass8 combination setting - should disable parenb, istrip, set cs8
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["pass8"])
        .succeeds();
    let result = new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds();
    result.stdout_contains("-parenb");
    result.stdout_contains("-istrip");
    result.stdout_contains("cs8");
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
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_combo_decctlq() {
    // Test decctlq combination setting - should enable ixany
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["decctlq"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds()
        .stdout_contains("ixany");
}

#[test]
#[cfg(unix)]
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_combo_dec() {
    // Test dec combination setting - should set multiple flags
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["dec"])
        .succeeds();
    let result = new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds();
    // dec sets echoe, echoctl, echoke
    result.stdout_contains("echoe");
}

#[test]
#[cfg(unix)]
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_combo_crt() {
    // Test crt combination setting - should set echoe
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["crt"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds()
        .stdout_contains("echoe");
}

#[test]
#[cfg(unix)]
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_multiple_settings() {
    // Test setting multiple flags at once and verify all are set
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["parenb", "parodd", "cs7"])
        .succeeds();
    let result = new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds();
    result.stdout_contains("parenb");
    result.stdout_contains("parodd");
    result.stdout_contains("cs7");
}

#[test]
#[cfg(unix)]
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_set_all_control_chars() {
    // Test setting intr control character and verify
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["intr", "^C"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .succeeds()
        .stdout_contains("intr = ^C");

    // Test setting quit control character and verify
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["quit", "^\\"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .succeeds()
        .stdout_contains("quit = ^\\");

    // Test setting erase control character and verify
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["erase", "^?"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .succeeds()
        .stdout_contains("erase = ^?");

    // Test setting kill control character and verify
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["kill", "^U"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .succeeds()
        .stdout_contains("kill = ^U");
}

#[test]
#[cfg(unix)]
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_print_size() {
    // Test size print setting - should output "rows <num>; columns <num>;"
    let result = new_ucmd!()
        .terminal_simulation(true)
        .args(&["size"])
        .succeeds();
    result.stdout_contains("rows");
    result.stdout_contains("columns");
}

#[test]
#[cfg(unix)]
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_print_speed() {
    // Test speed print setting - should output a numeric speed
    let result = new_ucmd!()
        .terminal_simulation(true)
        .args(&["speed"])
        .succeeds();
    // Speed should be a number (common speeds: 9600, 38400, 115200, etc.)
    let stdout = result.stdout_str();
    assert!(
        stdout.trim().parse::<u32>().is_ok(),
        "Speed should be a numeric value"
    );
}

#[test]
#[cfg(unix)]
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_set_rows_cols() {
    // Test setting rows and verify
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["rows", "24"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["size"])
        .succeeds()
        .stdout_contains("rows 24");

    // Test setting cols and verify
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["cols", "80"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["size"])
        .succeeds()
        .stdout_contains("columns 80");

    // Test setting both rows and cols together
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["rows", "50", "cols", "100"])
        .succeeds();
    let result = new_ucmd!()
        .terminal_simulation(true)
        .args(&["size"])
        .succeeds();
    result.stdout_contains("rows 50");
    result.stdout_contains("columns 100");
}

#[test]
#[cfg(unix)]
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_character_size_settings() {
    // Test cs5 setting and verify
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["cs5"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds()
        .stdout_contains("cs5");

    // Test cs7 setting and verify
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["cs7"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds()
        .stdout_contains("cs7");

    // Test cs8 setting and verify
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["cs8"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds()
        .stdout_contains("cs8");
}

#[test]
#[cfg(unix)]
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_baud_rate_settings() {
    // Test setting ispeed and verify
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["ispeed", "9600"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["speed"])
        .succeeds()
        .stdout_contains("9600");

    // Test setting both ispeed and ospeed
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["ispeed", "38400", "ospeed", "38400"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["speed"])
        .succeeds()
        .stdout_contains("38400");
}

#[test]
#[cfg(unix)]
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_min_time_settings() {
    // Test min setting and verify
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["min", "1"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds()
        .stdout_contains("min = 1");

    // Test time setting and verify
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["time", "10"])
        .succeeds();
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds()
        .stdout_contains("time = 10");
}

#[test]
#[cfg(unix)]
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_complex_scenario() {
    // Test a complex scenario with multiple settings and verify all are applied
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["sane", "rows", "24", "cols", "80", "intr", "^C"])
        .succeeds();

    // Verify all settings were applied
    let size_result = new_ucmd!()
        .terminal_simulation(true)
        .args(&["size"])
        .succeeds();
    size_result.stdout_contains("rows 24");
    size_result.stdout_contains("columns 80");

    let result = new_ucmd!().terminal_simulation(true).succeeds();
    result.stdout_contains("intr = ^C");
}

#[test]
#[cfg(unix)]
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_raw_mode() {
    // Test raw mode setting
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["raw"])
        .succeeds();
    // Verify raw mode is set by checking output
    let result = new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds();
    result.stdout_contains("-icanon");
}

#[test]
#[cfg(unix)]
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_cooked_mode() {
    // Test cooked mode setting (opposite of raw)
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["cooked"])
        .succeeds();
    // Verify cooked mode is set
    let result = new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds();
    result.stdout_contains("icanon");
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
#[ignore = "Fails because cargo test does not run in a tty"]
fn test_parity_settings() {
    // Test evenp setting and verify (should set parenb and cs7)
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["evenp"])
        .succeeds();
    let result = new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds();
    result.stdout_contains("parenb");
    result.stdout_contains("cs7");

    // Test oddp setting and verify (should set parenb, parodd, and cs7)
    new_ucmd!()
        .terminal_simulation(true)
        .args(&["oddp"])
        .succeeds();
    let result = new_ucmd!()
        .terminal_simulation(true)
        .args(&["--all"])
        .succeeds();
    result.stdout_contains("parenb");
    result.stdout_contains("parodd");
    result.stdout_contains("cs7");
}

// Additional integration tests for missing coverage

#[test]
fn missing_arg_ispeed() {
    // Test missing argument for ispeed
    new_ucmd!()
        .args(&["ispeed"])
        .fails()
        .stderr_contains("missing argument")
        .stderr_contains("ispeed");
}

#[test]
fn missing_arg_ospeed() {
    // Test missing argument for ospeed
    new_ucmd!()
        .args(&["ospeed"])
        .fails()
        .stderr_contains("missing argument")
        .stderr_contains("ospeed");
}

#[test]
fn missing_arg_line() {
    // Test missing argument for line
    new_ucmd!()
        .args(&["line"])
        .fails()
        .stderr_contains("missing argument")
        .stderr_contains("line");
}

#[test]
fn missing_arg_min() {
    // Test missing argument for min
    new_ucmd!()
        .args(&["min"])
        .fails()
        .stderr_contains("missing argument")
        .stderr_contains("min");
}

#[test]
fn missing_arg_time() {
    // Test missing argument for time
    new_ucmd!()
        .args(&["time"])
        .fails()
        .stderr_contains("missing argument")
        .stderr_contains("time");
}

#[test]
fn missing_arg_rows() {
    // Test missing argument for rows
    new_ucmd!()
        .args(&["rows"])
        .fails()
        .stderr_contains("missing argument")
        .stderr_contains("rows");
}

#[test]
fn missing_arg_cols() {
    // Test missing argument for cols
    new_ucmd!()
        .args(&["cols"])
        .fails()
        .stderr_contains("missing argument")
        .stderr_contains("cols");
}

#[test]
fn missing_arg_columns() {
    // Test missing argument for columns
    new_ucmd!()
        .args(&["columns"])
        .fails()
        .stderr_contains("missing argument")
        .stderr_contains("columns");
}

#[test]
fn missing_arg_control_char() {
    // Test missing argument for control character
    new_ucmd!()
        .args(&["intr"])
        .fails()
        .stderr_contains("missing argument")
        .stderr_contains("intr");

    new_ucmd!()
        .args(&["erase"])
        .fails()
        .stderr_contains("missing argument")
        .stderr_contains("erase");
}

#[test]
fn invalid_integer_rows() {
    // Test invalid integer for rows
    new_ucmd!()
        .args(&["rows", "abc"])
        .fails()
        .stderr_contains("invalid integer argument");

    new_ucmd!()
        .args(&["rows", "-1"])
        .fails()
        .stderr_contains("invalid integer argument");
}

#[test]
fn invalid_integer_cols() {
    // Test invalid integer for cols
    new_ucmd!()
        .args(&["cols", "xyz"])
        .fails()
        .stderr_contains("invalid integer argument");

    new_ucmd!()
        .args(&["columns", "12.5"])
        .fails()
        .stderr_contains("invalid integer argument");
}

#[test]
fn invalid_min_value() {
    // Test invalid min value
    new_ucmd!()
        .args(&["min", "256"])
        .fails()
        .stderr_contains("Value too large");

    new_ucmd!()
        .args(&["min", "-1"])
        .fails()
        .stderr_contains("invalid integer argument");
}

#[test]
fn invalid_time_value() {
    // Test invalid time value
    new_ucmd!()
        .args(&["time", "1000"])
        .fails()
        .stderr_contains("Value too large");

    new_ucmd!()
        .args(&["time", "abc"])
        .fails()
        .stderr_contains("invalid integer argument");
}

#[test]
fn invalid_baud_rate() {
    // Test invalid baud rate for ispeed (non-numeric string)
    // spell-checker:ignore notabaud
    new_ucmd!()
        .args(&["ispeed", "notabaud"])
        .fails()
        .stderr_contains("invalid ispeed");

    // On non-BSD systems, test invalid numeric baud rate
    // On BSD systems, any u32 is accepted, so we skip this test
    #[cfg(not(any(
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "ios",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd"
    )))]
    {
        new_ucmd!()
            .args(&["ospeed", "999999999"])
            .fails()
            .stderr_contains("invalid ospeed");
    }
}

#[test]
fn control_char_multiple_chars_error() {
    // Test that control characters with multiple chars fail
    new_ucmd!()
        .args(&["intr", "ABC"])
        .fails()
        .stderr_contains("invalid integer argument");
}

#[test]
fn control_char_decimal_overflow() {
    // Test decimal overflow for control characters
    new_ucmd!()
        .args(&["quit", "256"])
        .fails()
        .stderr_contains("Value too large");

    // spell-checker:ignore susp
    new_ucmd!()
        .args(&["susp", "1000"])
        .fails()
        .stderr_contains("Value too large");
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

#[test]
#[cfg(unix)]
fn test_columns_env_wrapping() {
    use std::process::Stdio;
    let (path, _controller, _replica) = pty_path();

    // Must pipe output so stty uses COLUMNS env instead of actual terminal size
    for (columns, max_len) in [(20, 20), (40, 40), (50, 50)] {
        let result = new_ucmd!()
            .args(&["--all", "--file", &path])
            .env("COLUMNS", columns.to_string())
            .set_stdout(Stdio::piped())
            .succeeds();

        for line in result.stdout_str().lines() {
            assert!(
                line.len() <= max_len,
                "Line exceeds COLUMNS={columns}: '{line}'"
            );
        }
    }

    // Wide columns should allow longer lines
    let result = new_ucmd!()
        .args(&["--all", "--file", &path])
        .env("COLUMNS", "200")
        .set_stdout(Stdio::piped())
        .succeeds();
    let has_long_line = result.stdout_str().lines().any(|line| line.len() > 80);
    assert!(
        has_long_line,
        "Expected at least one line longer than 80 chars with COLUMNS=200"
    );

    // Invalid values should fall back to default
    for invalid in ["invalid", "0", "-10"] {
        new_ucmd!()
            .args(&["--all", "--file", &path])
            .env("COLUMNS", invalid)
            .set_stdout(Stdio::piped())
            .succeeds();
    }

    // Without --all flag
    let result = new_ucmd!()
        .args(&["--file", &path])
        .env("COLUMNS", "30")
        .set_stdout(Stdio::piped())
        .succeeds();
    for line in result.stdout_str().lines() {
        assert!(
            line.len() <= 30,
            "Line exceeds COLUMNS=30 without --all: '{line}'"
        );
    }
}
