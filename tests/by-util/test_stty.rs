// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore parenb parmrk ixany iuclc onlcr ofdel icanon noflsh econl igpar ispeed ospeed notachar cbreak evenp oddp CSIZE

use uutests::new_ucmd;

#[test]
fn test_invalid_arg() {
    new_ucmd!()
        .arg("--definitely-invalid")
        .fails_with_code(1)
        .stderr_contains("invalid argument")
        .stderr_contains("--definitely-invalid");
}

#[test]
#[ignore = "Fails because cargo test does not run in a tty"]
fn runs() {
    new_ucmd!().succeeds();
}

#[test]
#[ignore = "Fails because cargo test does not run in a tty"]
fn print_all() {
    let res = new_ucmd!().args(&["--all"]).succeeds();

    // Random selection of flags to check for
    for flag in [
        "parenb", "parmrk", "ixany", "onlcr", "ofdel", "icanon", "noflsh",
    ] {
        res.stdout_contains(flag);
    }
}

#[test]
#[ignore = "Fails because cargo test does not run in a tty"]
fn sane_settings() {
    new_ucmd!().args(&["intr", "^A"]).succeeds();
    new_ucmd!().succeeds().stdout_contains("intr = ^A");
    new_ucmd!()
        .args(&["sane"])
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
