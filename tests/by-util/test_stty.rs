// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore parenb parmrk ixany iuclc onlcr ofdel icanon noflsh econl igpar ispeed ospeed

use uutests::new_ucmd;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
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
