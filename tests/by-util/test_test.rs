// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (words) egid euid icacls pseudofloat

use uutests::util::TestScenario;
use uutests::{at_and_ucmd, new_ucmd, util_name};

#[test]
fn test_empty_test_equivalent_to_false() {
    new_ucmd!().fails_with_code(1);
}

#[test]
fn test_empty_string_is_false() {
    new_ucmd!().arg("").fails_with_code(1);
}

#[test]
fn test_solo_not() {
    new_ucmd!().arg("!").succeeds();
}

#[test]
fn test_solo_and_or_or_is_a_literal() {
    // /bin/test '' -a '' => 1; so test(1) must interpret `-a` by itself as
    // a literal string
    new_ucmd!().arg("-a").succeeds();
    new_ucmd!().arg("-o").succeeds();
}

#[test]
fn test_some_literals() {
    let scenario = TestScenario::new(util_name!());
    let tests = [
        "a string",
        "(",
        ")",
        "-",
        "--",
        "-0",
        "-f",
        "--help",
        "--version",
        "-eq",
        "-lt",
        "-ef",
        "[",
    ];

    for test in &tests {
        scenario.ucmd().arg(test).succeeds();
    }

    // run the inverse of all these tests
    for test in &tests {
        scenario.ucmd().arg("!").arg(test).fails_with_code(1);
    }
}

#[test]
fn test_double_not_is_false() {
    new_ucmd!().args(&["!", "!"]).fails_with_code(1);
}

#[test]
fn test_and_not_is_false() {
    new_ucmd!().args(&["-a", "!"]).fails_with_code(2);
}

#[test]
fn test_not_and_is_false() {
    // `-a` is a literal here & has nonzero length
    new_ucmd!().args(&["!", "-a"]).fails_with_code(1);
}

#[test]
fn test_not_and_not_succeeds() {
    new_ucmd!().args(&["!", "-a", "!"]).succeeds();
}

#[test]
fn test_simple_or() {
    new_ucmd!().args(&["foo", "-o", ""]).succeeds();
}

#[test]
fn test_errors_miss_and_or() {
    new_ucmd!()
        .args(&["-o", "arg"])
        .fails_with_code(2)
        .stderr_contains("'-o': unary operator expected");
    new_ucmd!()
        .args(&["-a", "arg"])
        .fails()
        .stderr_contains("'-a': unary operator expected");
}

#[test]
fn test_negated_or() {
    new_ucmd!()
        .args(&["!", "foo", "-o", "bar"])
        .fails_with_code(1);
    new_ucmd!().args(&["foo", "-o", "!", "bar"]).succeeds();
    new_ucmd!()
        .args(&["!", "foo", "-o", "!", "bar"])
        .fails_with_code(1);
}

#[test]
fn test_string_length_of_nothing() {
    // odd but matches GNU, which must interpret -n as a literal here
    new_ucmd!().arg("-n").succeeds();
}

#[test]
fn test_string_length_of_empty() {
    new_ucmd!().args(&["-n", ""]).fails_with_code(1);

    // STRING equivalent to -n STRING
    new_ucmd!().arg("").fails_with_code(1);
}

#[test]
fn test_nothing_is_empty() {
    // -z is a literal here and has nonzero length
    new_ucmd!().arg("-z").succeeds();
}

#[test]
fn test_zero_len_of_empty() {
    new_ucmd!().args(&["-z", ""]).succeeds();
}

#[test]
fn test_zero_len_equals_zero_len() {
    new_ucmd!().args(&["", "=", ""]).succeeds();
}

#[test]
fn test_zero_len_not_equals_zero_len_is_false() {
    new_ucmd!().args(&["", "!=", ""]).fails_with_code(1);
}

#[test]
fn test_double_equal_is_string_comparison_op() {
    // undocumented but part of the GNU test suite
    new_ucmd!().args(&["t", "==", "t"]).succeeds();
    new_ucmd!().args(&["t", "==", "f"]).fails_with_code(1);
}

#[test]
fn test_string_comparison() {
    let scenario = TestScenario::new(util_name!());
    let tests = [
        ["foo", "!=", "bar"],
        ["contained\nnewline", "=", "contained\nnewline"],
        ["(", "=", "("],
        ["(", "!=", ")"],
        ["(", "!=", "="],
        ["!", "=", "!"],
        ["=", "=", "="],
    ];

    for test in &tests {
        scenario.ucmd().args(&test[..]).succeeds();
    }

    // run the inverse of all these tests
    for test in &tests {
        scenario.ucmd().arg("!").args(&test[..]).fails_with_code(1);
    }
}

#[test]
#[ignore = "fixme: error reporting"]
fn test_dangling_string_comparison_is_error() {
    new_ucmd!()
        .args(&["missing_something", "="])
        .fails_with_code(2)
        .stderr_is("test: missing argument after '='");
}

#[test]
fn test_string_operator_is_literal_after_bang() {
    let scenario = TestScenario::new(util_name!());
    let tests = [
        ["!", "="],
        ["!", "!="],
        ["!", "-eq"],
        ["!", "-ne"],
        ["!", "-lt"],
        ["!", "-le"],
        ["!", "-gt"],
        ["!", "-ge"],
        ["!", "-ef"],
        ["!", "-nt"],
        ["!", "-ot"],
    ];

    for test in &tests {
        scenario.ucmd().args(&test[..]).fails_with_code(1);
    }
}

#[test]
fn test_a_bunch_of_not() {
    new_ucmd!()
        .args(&["!", "", "!=", "", "-a", "!", "", "!=", ""])
        .succeeds();
}

#[test]
fn test_pseudofloat_equal() {
    // string comparison; test(1) doesn't support comparison of actual floats
    new_ucmd!().args(&["123.45", "=", "123.45"]).succeeds();
}

#[test]
fn test_pseudofloat_not_equal() {
    // string comparison; test(1) doesn't support comparison of actual floats
    new_ucmd!().args(&["123.45", "!=", "123.450"]).succeeds();
}

#[test]
fn test_negative_arg_is_a_string() {
    new_ucmd!().arg("-12345").succeeds();
    new_ucmd!().arg("--qwert").succeeds(); // spell-checker:disable-line
}

#[test]
fn test_some_int_compares() {
    let scenario = TestScenario::new(util_name!());

    let tests = [
        ["0", "-eq", "0"],
        ["0", "-ne", "1"],
        ["421", "-lt", "3720"],
        ["0", "-le", "0"],
        ["11", "-gt", "10"],
        ["1024", "-ge", "512"],
        ["9223372036854775806", "-le", "9223372036854775807"],
    ];

    for test in &tests {
        scenario.ucmd().args(&test[..]).succeeds();
    }

    // run the inverse of all these tests
    for test in &tests {
        scenario.ucmd().arg("!").args(&test[..]).fails_with_code(1);
    }
}

#[test]
fn test_values_greater_than_i64_allowed() {
    new_ucmd!()
        .args(&["9223372036854775808", "-gt", "0"])
        .succeeds();
}

/// The 71-digit operand reported in GNU compatibility issue #12874.
const BIG: &str = "16267277278126277227728782172782882627278282882172762677623672762783782";
/// `BIG` with its final digit incremented, so the two operands have the same
/// width and differ only in the least significant digit.
const BIG_PLUS_ONE: &str =
    "16267277278126277227728782172782882627278282882172762677623672762783783";
/// One digit shorter than `BIG`, so the two operands differ in width.
const SMALLER: &str = "1626727727812627722772878217278288262727828288217276267762367276278378";

#[test]
fn test_values_greater_than_i128_allowed() {
    // i128::MAX + 1
    new_ucmd!()
        .args(&["170141183460469231731687303715884105728", "-gt", "0"])
        .succeeds();
    // i128::MIN - 1
    new_ucmd!()
        .args(&["-170141183460469231731687303715884105729", "-lt", "0"])
        .succeeds();
}

#[test]
fn test_large_int_compares() {
    let scenario = TestScenario::new(util_name!());

    let tests = [
        [BIG, "-eq", BIG],
        [BIG, "-ge", BIG],
        [BIG, "-le", BIG],
        [BIG, "-ne", "1"],
        ["1", "-lt", BIG],
        [BIG, "-gt", "1"],
        // Same width, differing only in the least significant digit.
        [BIG_PLUS_ONE, "-gt", BIG],
        [BIG, "-lt", BIG_PLUS_ONE],
        // Differing widths.
        [BIG, "-gt", SMALLER],
        [SMALLER, "-lt", BIG],
    ];

    for test in &tests {
        scenario.ucmd().args(&test[..]).succeeds();
    }

    // run the inverse of all these tests
    for test in &tests {
        scenario.ucmd().arg("!").args(&test[..]).fails_with_code(1);
    }
}

#[test]
fn test_large_negative_int_compares() {
    let scenario = TestScenario::new(util_name!());
    let neg_big = format!("-{BIG}");
    let neg_smaller = format!("-{SMALLER}");

    let tests = [
        [neg_big.as_str(), "-eq", neg_big.as_str()],
        [neg_big.as_str(), "-lt", "0"],
        [neg_big.as_str(), "-lt", BIG],
        [BIG, "-gt", neg_big.as_str()],
        // A wider negative number is the smaller of the two.
        [neg_big.as_str(), "-lt", neg_smaller.as_str()],
        [neg_smaller.as_str(), "-gt", neg_big.as_str()],
    ];

    for test in &tests {
        scenario.ucmd().args(&test[..]).succeeds();
    }

    // run the inverse of all these tests
    for test in &tests {
        scenario.ucmd().arg("!").args(&test[..]).fails_with_code(1);
    }
}

#[test]
fn test_int_compares_ignore_redundant_sign_and_leading_zeros() {
    let scenario = TestScenario::new(util_name!());
    let padded_big = format!("+00{BIG}");

    let tests = [
        // Zero carries no sign.
        ["-0", "-eq", "0"],
        ["+0", "-eq", "-0"],
        ["0", "-eq", "0000000000"],
        ["007", "-eq", "7"],
        ["-007", "-eq", "-7"],
        [padded_big.as_str(), "-eq", BIG],
    ];

    for test in &tests {
        scenario.ucmd().args(&test[..]).succeeds();
    }
}

#[test]
fn test_malformed_integers_are_still_errors() {
    // Accepting wider operands must not make any of these parse.
    for operand in ["1_0", "0x10", "1e3", "++5", "5-", "-", "+", "", "4 2"] {
        new_ucmd!()
            .args(&[operand, "-eq", "0"])
            .fails_with_code(2)
            .stderr_is(format!("test: invalid integer '{operand}'\n"));
    }
}

#[test]
fn test_negative_int_compare() {
    let scenario = TestScenario::new(util_name!());

    let tests = [
        ["-1", "-eq", "-1"],
        ["-1", "-ne", "-2"],
        ["-3720", "-lt", "-421"],
        ["-10", "-le", "-10"],
        ["-21", "-gt", "-22"],
        ["-128", "-ge", "-256"],
        ["-9223372036854775808", "-le", "-9223372036854775807"],
    ];

    for test in &tests {
        scenario.ucmd().args(&test[..]).succeeds();
    }

    // run the inverse of all these tests
    for test in &tests {
        scenario.ucmd().arg("!").args(&test[..]).fails_with_code(1);
    }
}

#[test]
fn test_float_inequality_is_error() {
    new_ucmd!()
        .args(&["123.45", "-ge", "6"])
        .fails_with_code(2)
        .stderr_is("test: invalid integer '123.45'\n");
}

#[test]
#[cfg(not(windows))]
#[cfg_attr(wasi_runner, ignore = "WASI: argv/filenames must be valid UTF-8")]
fn test_invalid_utf8_integer_compare() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let source = [0x66, 0x6f, 0x80, 0x6f];
    let arg = OsStr::from_bytes(&source[..]);

    new_ucmd!()
        .args(&[OsStr::new("123"), OsStr::new("-ne"), arg])
        .fails_with_code(2)
        .stderr_is("test: invalid integer $'fo\\x80o'\n");

    new_ucmd!()
        .args(&[arg, OsStr::new("-eq"), OsStr::new("456")])
        .fails_with_code(2)
        .stderr_is("test: invalid integer $'fo\\x80o'\n");
}

#[test]
fn test_integer_whitespace_stripping() {
    new_ucmd!().args(&["42", "-eq", " 42 "]).succeeds();
    new_ucmd!().args(&["42", "-eq", " 42"]).succeeds();
    new_ucmd!().args(&["42", "-eq", "42 "]).succeeds();
    new_ucmd!().args(&[" 42 ", "-eq", "42"]).succeeds();

    new_ucmd!().args(&["42", "-eq", "\t42"]).succeeds();
    new_ucmd!().args(&["42", "-eq", "\n42"]).succeeds();
    new_ucmd!().args(&["42", "-eq", "\x0b42"]).succeeds(); // Vertical tab
    new_ucmd!().args(&["42", "-eq", "\x0c42"]).succeeds(); // Form feed
    new_ucmd!().args(&["42", "-eq", "\r42"]).succeeds();
}

#[test]
fn test_isatty_whitespace_stripping() {
    new_ucmd!().args(&["-t", " 0 "]).fails_with_code(1);
    new_ucmd!().args(&["-t", "\n0\t"]).fails_with_code(1);
}

#[test]
fn test_isatty_invalid_fd_is_false() {
    // Asking the CRT about an unopened descriptor aborted the process.
    new_ucmd!().args(&["-t", "99"]).fails_with_code(1);
    new_ucmd!().args(&["-t", "-1"]).fails_with_code(1);
}

#[test]
#[cfg(windows)]
fn test_isatty_unknown_fd_windows() {
    // Only the standard streams are known on Windows.
    new_ucmd!().args(&["-t", "3"]).fails_with_code(1);
}

#[test]
fn test_file_is_itself() {
    new_ucmd!()
        .args(&["regular_file", "-ef", "regular_file"])
        .succeeds();
}

#[test]
#[cfg_attr(wasi_runner, ignore = "WASI sandbox: host paths not visible")]
// Disabled for android, since the temp dir doesn't allow creating hard links
#[cfg(not(target_os = "android"))]
fn test_hard_link_is_same_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.hard_link("regular_file", "hard_link");
    ucmd.args(&["regular_file", "-ef", "hard_link"]).succeeds();
}

#[test]
#[cfg(not(target_os = "android"))]
fn test_file_is_newer_than_and_older_than_itself() {
    // odd but matches GNU
    new_ucmd!()
        .args(&["regular_file", "-nt", "regular_file"])
        .fails_with_code(1);
    new_ucmd!()
        .args(&["regular_file", "-ot", "regular_file"])
        .fails_with_code(1);
}

#[test]
fn test_file_is_newer_than_non_existing_file() {
    new_ucmd!()
        .args(&["non_existing_file", "-nt", "regular_file"])
        .fails_with_code(1)
        .no_output();

    new_ucmd!()
        .args(&["regular_file", "-nt", "non_existing_file"])
        .succeeds()
        .no_output();

    new_ucmd!()
        .args(&["non_existing_file", "-ot", "regular_file"])
        .succeeds()
        .no_output();

    new_ucmd!()
        .args(&["regular_file", "-ot", "non_existing_file"])
        .fails_with_code(1)
        .no_output();
}

#[test]
#[cfg_attr(wasi_runner, ignore = "WASI sandbox: host paths not visible")]
fn test_same_device_inode() {
    let scenario = TestScenario::new(util_name!());
    let at = &scenario.fixtures;

    at.touch("regular_file_second");
    at.symlink_file("regular_file", "symlink");

    scenario
        .ucmd()
        .args(&["regular_file", "-ef", "regular_file_second"])
        .fails();

    scenario
        .ucmd()
        .args(&["regular_file", "-ef", "symlink"])
        .succeeds();
}

#[test]
#[cfg(not(target_os = "android"))]
fn test_newer_file() {
    let scenario = TestScenario::new(util_name!());

    let older_file = scenario.fixtures.make_file("older_file");
    older_file.set_modified(std::time::UNIX_EPOCH).unwrap();
    scenario.fixtures.touch("newer_file");

    scenario
        .ucmd()
        .args(&["newer_file", "-nt", "older_file"])
        .succeeds();

    scenario
        .ucmd()
        .args(&["older_file", "-nt", "newer_file"])
        .fails();

    scenario
        .ucmd()
        .args(&["older_file", "-ot", "newer_file"])
        .succeeds();

    scenario
        .ucmd()
        .args(&["newer_file", "-ot", "older_file"])
        .fails();
}

#[test]
fn test_file_exists() {
    new_ucmd!().args(&["-e", "regular_file"]).succeeds();
}

#[test]
fn test_nonexistent_file_does_not_exist() {
    new_ucmd!()
        .args(&["-e", "nonexistent_file"])
        .fails_with_code(1);
}

#[test]
fn test_nonexistent_file_is_not_regular() {
    new_ucmd!()
        .args(&["-f", "nonexistent_file"])
        .fails_with_code(1);
}

#[test]
fn test_file_exists_and_is_regular() {
    new_ucmd!().args(&["-f", "regular_file"]).succeeds();
}

#[test]
fn test_file_is_readable() {
    new_ucmd!().args(&["-r", "regular_file"]).succeeds();
}

#[test]
#[cfg(not(windows))]
#[cfg_attr(wasi_runner, ignore = "WASI: no permission bits")]
fn test_file_is_not_readable() {
    let scenario = TestScenario::new(util_name!());
    let mut ucmd = scenario.ucmd();
    let mut chmod = scenario.cmd("chmod");

    scenario.fixtures.touch("crypto_file");
    chmod.args(&["u-r", "crypto_file"]).succeeds();

    ucmd.args(&["!", "-r", "crypto_file"]).succeeds();
}

#[test]
#[cfg_attr(wasi_runner, ignore = "WASI: no permission bits")]
fn test_file_is_writable() {
    new_ucmd!().args(&["-w", "regular_file"]).succeeds();
}

#[test]
#[cfg(not(windows))]
fn test_file_is_not_writable() {
    let scenario = TestScenario::new(util_name!());
    let mut ucmd = scenario.ucmd();
    let mut chmod = scenario.cmd("chmod");

    scenario.fixtures.touch("immutable_file");
    chmod.args(&["u-w", "immutable_file"]).succeeds();

    ucmd.args(&["!", "-w", "immutable_file"]).succeeds();
}

#[test]
fn test_file_is_not_executable() {
    #[cfg(unix)]
    let (at, mut ucmd) = at_and_ucmd!();
    #[cfg(not(unix))]
    let (_, mut ucmd) = at_and_ucmd!();

    // WSL creates executable files by default, so if we are on unix, make sure
    // to set make it non-executable.
    // Files on other targets are non-executable by default.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(at.plus("regular_file")).unwrap();
        let mut permissions = metadata.permissions();

        // The conversion is useless on some platforms and casts from u16 to
        // u32 on others
        #[allow(clippy::useless_conversion)]
        permissions.set_mode(permissions.mode() & !u32::from(libc::S_IXUSR));
        std::fs::set_permissions(at.plus("regular_file"), permissions).unwrap();
    }
    ucmd.args(&["!", "-x", "regular_file"]).succeeds();
}

#[test]
#[cfg(not(windows))]
#[cfg_attr(wasi_runner, ignore = "WASI: no permission bits")]
fn test_file_is_executable() {
    let scenario = TestScenario::new(util_name!());
    let mut chmod = scenario.cmd("chmod");

    chmod.args(&["u+x", "regular_file"]).succeeds();

    scenario.ucmd().args(&["-x", "regular_file"]).succeeds();
}

#[test]
#[cfg(windows)]
fn test_file_is_not_writable_windows() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("readonly_file");
    let mut perms = std::fs::metadata(at.plus("readonly_file"))
        .unwrap()
        .permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(at.plus("readonly_file"), perms).unwrap();
    ucmd.args(&["!", "-w", "readonly_file"]).succeeds();
}

#[test]
#[cfg(windows)]
fn test_readonly_directory_is_writable_windows() {
    // NTFS ignores the read-only attribute on directories.
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("readonly_dir");
    at.set_readonly("readonly_dir");
    ucmd.args(&["-w", "readonly_dir"]).succeeds();
}

#[test]
#[cfg(windows)]
fn test_file_is_executable_windows() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("PROGRAM.EXE");
    ucmd.args(&["-x", "PROGRAM.EXE"]).succeeds();
}

#[test]
#[cfg_attr(wasi_runner, ignore = "WASI: no permission bits")]
fn test_directory_is_executable() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("dir");
    ucmd.args(&["-x", "dir"]).succeeds();
}

// Denies a single right so cleanup still works: it needs DELETE and directory
// listing.
#[cfg(windows)]
fn deny_everyone(scenario: &TestScenario, name: &str, rights: &str) {
    let ace = format!("*S-1-1-0:({rights})");
    scenario
        .cmd("icacls")
        .args(&[name, "/deny", ace.as_str()])
        .succeeds();
}

#[test]
#[cfg(windows)]
fn test_file_is_not_readable_windows() {
    let scenario = TestScenario::new(util_name!());
    scenario.fixtures.touch("crypto_file");
    deny_everyone(&scenario, "crypto_file", "RD");

    scenario.ucmd().args(&["!", "-r", "crypto_file"]).succeeds();
}

#[test]
#[cfg(windows)]
fn test_file_is_not_writable_by_acl_windows() {
    let scenario = TestScenario::new(util_name!());
    let at = &scenario.fixtures;
    at.touch("immutable_file");
    at.mkdir("immutable_dir");
    deny_everyone(&scenario, "immutable_file", "WD");
    deny_everyone(&scenario, "immutable_dir", "WD");

    scenario
        .ucmd()
        .args(&["!", "-w", "immutable_file"])
        .succeeds();
    scenario
        .ucmd()
        .args(&["!", "-w", "immutable_dir"])
        .succeeds();
}

#[test]
#[cfg(windows)]
fn test_file_is_not_executable_by_acl_windows() {
    let scenario = TestScenario::new(util_name!());
    scenario.fixtures.touch("program.exe");
    deny_everyone(&scenario, "program.exe", "X");

    scenario.ucmd().args(&["!", "-x", "program.exe"]).succeeds();
}

#[test]
#[cfg(windows)]
fn test_directory_is_not_executable_by_acl_windows() {
    let scenario = TestScenario::new(util_name!());
    scenario.fixtures.mkdir("locked_dir");
    deny_everyone(&scenario, "locked_dir", "X");

    scenario.ucmd().args(&["!", "-x", "locked_dir"]).succeeds();
}

#[test]
fn test_is_not_empty() {
    new_ucmd!().args(&["-s", "non_empty_file"]).succeeds();
}

#[test]
fn test_nonexistent_file_size_test_is_false() {
    new_ucmd!()
        .args(&["-s", "nonexistent_file"])
        .fails_with_code(1);
}

#[test]
fn test_not_is_not_empty() {
    new_ucmd!().args(&["!", "-s", "regular_file"]).succeeds();
}

#[test]
fn test_symlink_is_symlink() {
    let scenario = TestScenario::new(util_name!());
    let at = &scenario.fixtures;

    at.symlink_file("regular_file", "symlink");

    scenario.ucmd().args(&["-h", "symlink"]).succeeds();
    scenario.ucmd().args(&["-L", "symlink"]).succeeds();
}

#[test]
fn test_file_is_not_symlink() {
    let scenario = TestScenario::new(util_name!());

    scenario
        .ucmd()
        .args(&["!", "-h", "regular_file"])
        .succeeds();
    scenario
        .ucmd()
        .args(&["!", "-L", "regular_file"])
        .succeeds();
}

#[test]
fn test_nonexistent_file_is_not_symlink() {
    let scenario = TestScenario::new(util_name!());

    scenario
        .ucmd()
        .args(&["!", "-h", "nonexistent_file"])
        .succeeds();
    scenario
        .ucmd()
        .args(&["!", "-L", "nonexistent_file"])
        .succeeds();
}

#[test]
// Only the superuser is allowed to set the sticky bit on files on FreeBSD/OpenBSD.
// Windows has no concept of sticky bit
#[cfg(not(any(windows, target_os = "freebsd", target_os = "openbsd")))]
#[cfg_attr(wasi_runner, ignore = "WASI: no permission bits")]
fn test_file_is_sticky() {
    let scenario = TestScenario::new(util_name!());
    let mut ucmd = scenario.ucmd();
    let mut chmod = scenario.cmd("chmod");

    scenario.fixtures.touch("sticky_file");
    chmod.args(&["+t", "sticky_file"]).succeeds();

    ucmd.args(&["-k", "sticky_file"]).succeeds();
}

#[test]
fn test_file_is_not_sticky() {
    new_ucmd!().args(&["-k", "regular_file"]).fails_with_code(1);
}

#[test]
fn test_solo_empty_parenthetical_is_error() {
    new_ucmd!().args(&["(", ")"]).fails_with_code(2);
}

#[test]
fn test_parenthesized_literal() {
    let scenario = TestScenario::new(util_name!());
    let tests = [
        "a string",
        "(",
        ")",
        "-",
        "--",
        "-0",
        "-f",
        "--help",
        "--version",
        "-e",
        "-t",
        "!",
        "-n",
        "-z",
        "[",
        "-a",
        "-o",
    ];

    for test in &tests {
        scenario.ucmd().arg("(").arg(test).arg(")").succeeds();
    }

    // run the inverse of all these tests
    for test in &tests {
        scenario
            .ucmd()
            .arg("!")
            .arg("(")
            .arg(test)
            .arg(")")
            .fails_with_code(1);
    }
}

#[test]
fn test_parenthesized_op_compares_literal_parenthesis() {
    // ensure we aren’t treating this case as “string length of literal equal
    // sign”
    new_ucmd!().args(&["(", "=", ")"]).fails_with_code(1);
}

#[test]
fn test_parenthesized_string_comparison() {
    let scenario = TestScenario::new(util_name!());
    let tests = [
        ["(", "foo", "!=", "bar", ")"],
        ["(", "contained\nnewline", "=", "contained\nnewline", ")"],
        ["(", "(", "=", "(", ")"],
        ["(", "(", "!=", ")", ")"],
        ["(", "!", "=", "!", ")"],
        ["(", "=", "=", "=", ")"],
    ];

    for test in &tests {
        scenario.ucmd().args(&test[..]).succeeds();
    }

    // run the inverse of all these tests
    for test in &tests {
        scenario.ucmd().arg("!").args(&test[..]).fails_with_code(1);
    }
}

#[test]
fn test_parenthesized_right_parenthesis_as_literal() {
    new_ucmd!().args(&["(", "-f", ")", ")"]).fails_with_code(1);
}

#[test]
#[cfg_attr(wasi_runner, ignore = "WASI: no uid/gid")]
fn test_file_owned_by_euid() {
    new_ucmd!().args(&["-O", "regular_file"]).succeeds();
}

#[test]
fn test_nonexistent_file_not_owned_by_euid() {
    new_ucmd!()
        .args(&["-O", "nonexistent_file"])
        .fails_with_code(1);
}

#[test]
#[cfg(not(windows))]
#[cfg_attr(wasi_runner, ignore = "WASI: no uid/gid")]
fn test_file_not_owned_by_euid() {
    new_ucmd!()
        .args(&["-f", "/bin/sh", "-a", "!", "-O", "/bin/sh"])
        .succeeds();
}

#[test]
#[cfg(not(windows))]
#[cfg_attr(wasi_runner, ignore = "WASI: no uid/gid")]
fn test_file_owned_by_egid() {
    // On some platforms (mostly the BSDs) the test fixture files copied to the
    // /tmp directory will have a different gid than the current egid (due to
    // the sticky bit set on the /tmp directory). Fix this before running the
    // test command.
    use std::os::unix::fs::MetadataExt;
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let metadata = at.metadata("regular_file");
    let file_gid = rustix::fs::Gid::from_raw(metadata.gid());
    let user_gid = rustix::process::getegid();

    if user_gid != file_gid {
        let file_uid = rustix::fs::Uid::from_raw(metadata.uid());
        let path = at.plus("regular_file");
        rustix::fs::chown(&path, Some(file_uid), Some(user_gid)).expect("chown failed");
    }

    scene.ucmd().args(&["-G", "regular_file"]).succeeds();
}

#[test]
fn test_nonexistent_file_not_owned_by_egid() {
    new_ucmd!()
        .args(&["-G", "nonexistent_file"])
        .fails_with_code(1);
}

#[test]
#[cfg(not(windows))]
#[cfg_attr(wasi_runner, ignore = "WASI: no uid/gid")]
fn test_file_not_owned_by_egid() {
    let target_file = if cfg!(target_os = "freebsd") {
        // The coreutils test runner user has a primary group id of "wheel",
        // which matches the gid of /bin/sh, so use /sbin/shutdown which has gid
        // of "operator".
        "/sbin/shutdown"
    } else {
        "/bin/sh"
    };

    new_ucmd!()
        .args(&["-f", target_file, "-a", "!", "-G", target_file])
        .succeeds();
}

#[test]
#[cfg(windows)]
fn test_file_owned_by_current_group_windows() {
    new_ucmd!().args(&["-G", "regular_file"]).succeeds();
}

#[test]
#[cfg(windows)]
fn test_file_not_owned_by_current_token_windows() {
    // The system directory belongs to TrustedInstaller, so it is owned neither
    // by the user nor by the Administrators group an elevated shell runs as.
    let system_root = std::env::var("SystemRoot").expect("SystemRoot is not set");

    new_ucmd!()
        .args(&["-d", &system_root, "-a", "!", "-O", &system_root])
        .succeeds();
}

#[test]
fn test_op_precedence_and_or_1() {
    new_ucmd!().args(&[" ", "-o", "", "-a", ""]).succeeds();
}

#[test]
fn test_op_precedence_and_or_1_overridden_by_parentheses() {
    new_ucmd!()
        .args(&["(", " ", "-o", "", ")", "-a", ""])
        .fails_with_code(1);
}

#[test]
fn test_op_precedence_and_or_2() {
    new_ucmd!()
        .args(&["", "-a", "", "-o", " ", "-a", " "])
        .succeeds();
}

#[test]
fn test_op_precedence_and_or_2_overridden_by_parentheses() {
    new_ucmd!()
        .args(&["", "-a", "(", "", "-o", " ", ")", "-a", " "])
        .fails_with_code(1);
}

#[test]
fn test_negated_boolean_precedence() {
    let scenario = TestScenario::new(util_name!());

    let tests = [
        vec!["!", "(", "foo", ")", "-o", "bar"],
        vec!["!", "", "-o", "", "-a", ""],
        vec!["!", "(", "", "-a", "", ")", "-o", ""],
    ];

    for test in &tests {
        scenario.ucmd().args(&test[..]).succeeds();
    }

    let negative_tests = [
        vec!["!", "-n", "", "-a", ""],
        vec!["", "-a", "", "-o", ""],
        vec!["!", "", "-a", "", "-o", ""],
        vec!["!", "(", "", "-a", "", ")", "-a", ""],
    ];

    for test in &negative_tests {
        scenario.ucmd().args(&test[..]).fails_with_code(1);
    }
}

#[test]
fn test_bang_bool_op_precedence() {
    // For a Boolean combination of two literals, bang inverts the entire expression
    new_ucmd!().args(&["!", "", "-a", ""]).succeeds();
    new_ucmd!().args(&["!", "", "-o", ""]).succeeds();

    new_ucmd!()
        .args(&["!", "a value", "-o", "another value"])
        .fails_with_code(1);

    // Introducing a UOP — even one that is equivalent to a bare string — causes
    // bang to invert only the first term
    new_ucmd!()
        .args(&["!", "-n", "", "-a", ""])
        .fails_with_code(1);
    new_ucmd!()
        .args(&["!", "", "-a", "-n", ""])
        .fails_with_code(1);

    // for compound Boolean expressions, bang inverts the _next_ expression
    // only, not the entire compound expression
    new_ucmd!()
        .args(&["!", "", "-a", "", "-a", ""])
        .fails_with_code(1);

    // parentheses can override this
    new_ucmd!()
        .args(&["!", "(", "", "-a", "", "-a", "", ")"])
        .succeeds();
}

#[test]
fn test_inverted_parenthetical_bool_op_precedence() {
    // For a Boolean combination of two literals, bang inverts the entire expression
    new_ucmd!()
        .args(&["!", "a value", "-o", "another value"])
        .fails_with_code(1);

    // only the parenthetical is inverted, not the entire expression
    new_ucmd!()
        .args(&["!", "(", "a value", ")", "-o", "another value"])
        .succeeds();
}

#[test]
#[ignore = "fixme: error reporting"]
fn test_dangling_parenthesis() {
    new_ucmd!()
        .args(&["(", "(", "a", "!=", "b", ")", "-o", "-n", "c"])
        .fails_with_code(2);
    new_ucmd!()
        .args(&["(", "(", "a", "!=", "b", ")", "-o", "-n", "c", ")"])
        .succeeds();
}

#[test]
fn test_complicated_parenthesized_expression() {
    new_ucmd!()
        .args(&[
            "(", "(", "!", "(", "a", "=", "b", ")", "-o", "c", "=", "d", ")", "-a", "(", "q", "!=",
            "r", ")", ")",
        ])
        .succeeds();
}

#[test]
fn test_erroneous_parenthesized_expression() {
    new_ucmd!()
        .args(&["a", "!=", "(", "b", "-a", "b", ")", "!=", "c"])
        .fails_with_code(2)
        .stderr_is("test: extra argument 'b'\n");
}

#[test]
fn test_or_as_filename() {
    new_ucmd!()
        .args(&["x", "-a", "-z", "-o"])
        .fails_with_code(1);
}

#[test]
#[ignore = "TODO: Busybox has this working"]
fn test_filename_or_with_equal() {
    new_ucmd!().args(&["-f", "=", "a", "-o", "b"]).succeeds();
}

#[test]
#[ignore = "GNU considers this an error"]
fn test_string_length_and_nothing() {
    new_ucmd!().args(&["-n", "a", "-a"]).fails_with_code(2);
}

#[test]
fn test_bracket_syntax_success() {
    let scenario = TestScenario::new("[");
    let mut ucmd = scenario.ucmd();

    ucmd.args(&["1", "-eq", "1", "]"]).succeeds();
}

#[test]
fn test_bracket_syntax_failure() {
    let scenario = TestScenario::new("[");
    let mut ucmd = scenario.ucmd();

    ucmd.args(&["1", "-eq", "2", "]"]).fails_with_code(1);
}

#[test]
fn test_bracket_syntax_missing_right_bracket() {
    let scenario = TestScenario::new("[");
    let mut ucmd = scenario.ucmd();

    // Missing closing bracket takes precedence over other possible errors.
    ucmd.args(&["1", "-eq"])
        .fails_with_code(2)
        .stderr_is("[: missing ']'\n");
}

#[test]
fn test_bracket_syntax_help() {
    let scenario = TestScenario::new("[");
    let mut ucmd = scenario.ucmd();

    ucmd.arg("--help").succeeds().stdout_contains("Usage:");
}

#[test]
fn test_bracket_syntax_version() {
    let scenario = TestScenario::new("[");
    let mut ucmd = scenario.ucmd();

    ucmd.arg("--version")
        .succeeds()
        .stdout_matches(&r"\[ \(uutils coreutils\) \d+\.\d+\.\d+".parse().unwrap());
}

#[test]
#[allow(non_snake_case)]
fn test_file_N() {
    use std::{fs::FileTimes, time::Duration};

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let f = at.make_file("file");

    // Set the times so that the file is accessed _after_ being modified
    // => test -N return false.
    let times = FileTimes::new()
        .set_accessed(std::time::UNIX_EPOCH + Duration::from_secs(123))
        .set_modified(std::time::UNIX_EPOCH);
    f.set_times(times).unwrap();
    // TODO: stat call for debugging #7570, remove?
    #[cfg(unix)]
    println!("{}", scene.cmd_shell("stat file").succeeds().stdout_str());
    scene.ucmd().args(&["-N", "file"]).fails();

    // Set the times so that the file is modified _after_ being accessed
    // => test -N return true.
    let times = FileTimes::new()
        .set_accessed(std::time::UNIX_EPOCH)
        .set_modified(std::time::UNIX_EPOCH + Duration::from_secs(123));
    f.set_times(times).unwrap();
    // TODO: stat call for debugging #7570, remove?
    #[cfg(unix)]
    println!("{}", scene.cmd_shell("stat file").succeeds().stdout_str());
    scene.ucmd().args(&["-N", "file"]).succeeds();
}

#[test]
fn test_long_integer() {
    let scene = TestScenario::new(util_name!());
    scene
        .ucmd()
        .args(&["18446744073709551616", "-eq", "0"])
        .fails();
    scene
        .ucmd()
        .args(&["-9223372036854775809", "-ge", "18446744073709551616"])
        .fails();
    scene
        .ucmd()
        .args(&[
            "'('",
            "-9223372036854775809",
            "-ge",
            "18446744073709551616",
            "')'",
        ])
        .fails();
}

#[test]
fn test_missing_argument_after() {
    let mut ucmd = new_ucmd!();

    let result = ucmd.args(&["(", "foo"]).fails();
    result.no_stdout();
    assert_eq!(result.exit_status().code().unwrap(), 2);
    assert_eq!(
        result.stderr_str().trim(),
        "test: missing argument after 'foo'"
    );
}

#[test]
fn test_string_lt_gt_operator() {
    let items = [
        ("a", "b"),
        ("a", "aa"),
        ("a", "a "),
        ("a", "a b"),
        ("", "b"),
        ("a", "ä"),
    ];
    for (left, right) in items {
        new_ucmd!().args(&[left, "<", right]).succeeds().no_output();
        new_ucmd!()
            .args(&[right, "<", left])
            .fails_with_code(1)
            .no_output();

        new_ucmd!().args(&[right, ">", left]).succeeds().no_output();
        new_ucmd!()
            .args(&[left, ">", right])
            .fails_with_code(1)
            .no_output();
    }
    new_ucmd!()
        .args(&["", "<", ""])
        .fails_with_code(1)
        .no_output();
    new_ucmd!()
        .args(&["", ">", ""])
        .fails_with_code(1)
        .no_output();
}

#[test]
fn test_unary_op_as_literal_in_three_arg_form() {
    // `-f = a` is string comparison "-f" = "a", not file test
    new_ucmd!().args(&["-f", "=", "a"]).fails_with_code(1);
    new_ucmd!().args(&["-f", "=", "a", "-o", "b"]).succeeds();
}

#[cfg(all(feature = "feat_diagnostics", not(wasi_runner)))]
mod diagnostics {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn test_snippet_points_at_the_offending_argument() {
        let result = new_ucmd!()
            .terminal_sim_stderr()
            .args(&["7", "-eq", "zap"])
            .fails_with_code(2);

        // The whole report: the `test: ` prefix of the plain form, the
        // expression echoed back, a caret on `zap`, and the operator advice.
        assert_eq!(
            result.stderr_as_displayed(),
            "\
test: invalid integer 'zap'
   ╭─[ test:1:7 ]
   │
 1 │ 7 -eq zap
   │       ───
   │
   │ Help: -eq, -ne, -lt, -le, -gt and -ge compare integers; use =, !=, < or > to compare strings
   │       -eq equal, -ne not equal, -lt less than, -le less than or equal, -gt greater than, -ge greater than or equal
───╯"
        );
    }

    #[test]
    fn test_plain_message_is_the_default() {
        // The test harness pipes stderr, so the report must not appear.
        new_ucmd!()
            .args(&["7", "-eq", "zap"])
            .fails_with_code(2)
            .stderr_is("test: invalid integer 'zap'\n");
    }

    #[cfg(unix)]
    #[test]
    fn test_extra_argument_points_past_the_expression() {
        let result = new_ucmd!()
            .terminal_sim_stderr()
            .args(&["k", "!=", "m", "spare"])
            .fails_with_code(2);

        assert_eq!(
            result.stderr_as_displayed(),
            "\
test: extra argument 'spare'
   ╭─[ test:1:8 ]
   │
 1 │ k != m spare
   │        ──┬──
   │          ╰──── the expression was already complete here
   │
   │ Help: an unquoted variable expanding to several words is the usual cause; quote it as \"$var\" to keep it a single operand
───╯"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_missing_operand_suggests_quoting_the_variable() {
        // `test "$empty" -gt 1` with an unset variable ends up here: the
        // operator is left without a right-hand operand.
        let result = new_ucmd!()
            .terminal_sim_stderr()
            .args(&["31", "-gt"])
            .fails_with_code(2);

        // The caret sits on the operator left dangling, and the advice names
        // the cause rather than restating the error.
        assert_eq!(
            result.stderr_as_displayed(),
            "\
test: missing argument after '-gt'
   ╭─[ test:1:4 ]
   │
 1 │ 31 -gt
   │    ───
   │
   │ Help: an unset or empty variable expands to nothing, leaving the operator without an operand; quote it as \"$var\"
───╯"
        );
    }

    #[test]
    fn test_missing_operand_plain_message_is_unchanged() {
        // Piped stderr keeps the one-line form scripts match on.
        new_ucmd!()
            .args(&["31", "-gt"])
            .fails_with_code(2)
            .stderr_is("test: missing argument after '-gt'\n");
    }

    #[cfg(unix)]
    #[test]
    fn test_extra_operand_suggests_quoting_the_variable() {
        // What `test x = $fruit` looks like when fruit="ripe pear".
        let result = new_ucmd!()
            .terminal_sim_stderr()
            .args(&["x", "=", "ripe", "pear"])
            .fails_with_code(2);

        // The caret lands on the word past the end of the comparison, and the
        // integer advice — which belongs to another error — stays out of it.
        assert_eq!(
            result.stderr_as_displayed(),
            "\
test: extra argument 'pear'
   ╭─[ test:1:10 ]
   │
 1 │ x = ripe pear
   │          ──┬─
   │            ╰─── the expression was already complete here
   │
   │ Help: an unquoted variable expanding to several words is the usual cause; quote it as \"$var\" to keep it a single operand
───╯"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_dash_t_advises_about_descriptors_not_comparisons() {
        let result = new_ucmd!()
            .terminal_sim_stderr()
            .args(&["-t", "stdout"])
            .fails_with_code(2);

        // The message stays the one GNU prints; only the label and the advice
        // are specific to `-t`, which has nothing to do with `-eq` and friends.
        assert_eq!(
            result.stderr_as_displayed(),
            "\
test: invalid integer 'stdout'
   ╭─[ test:1:4 ]
   │
 1 │ -t stdout
   │    ──────
   │
   │ Help: -t takes a file descriptor number: 0 is standard input, 1 standard output, 2 standard error
───╯"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_integer_comparison_keeps_its_own_advice() {
        // The counterpart of the test above: the same operand under a real
        // comparison still gets the operator advice, not the descriptor one.
        let result = new_ucmd!()
            .terminal_sim_stderr()
            .args(&["1", "-eq", "stdout"])
            .fails_with_code(2);

        assert_eq!(
            result.stderr_as_displayed(),
            "\
test: invalid integer 'stdout'
   ╭─[ test:1:7 ]
   │
 1 │ 1 -eq stdout
   │       ──────
   │
   │ Help: -eq, -ne, -lt, -le, -gt and -ge compare integers; use =, !=, < or > to compare strings
   │       -eq equal, -ne not equal, -lt less than, -le less than or equal, -gt greater than, -ge greater than or equal
───╯"
        );
    }

    #[test]
    fn test_dash_t_plain_message_is_unchanged() {
        new_ucmd!()
            .args(&["-t", "stdout"])
            .fails_with_code(2)
            .stderr_is("test: invalid integer 'stdout'\n");
    }

    #[cfg(unix)]
    #[test]
    fn test_bracket_form_reports_under_its_own_name() {
        // Both the header and the snippet name `[`, and the trailing `]` is
        // dropped before the expression is echoed back.
        let result = TestScenario::new("[")
            .ucmd()
            .terminal_sim_stderr()
            .args(&["7", "-eq", "zap", "]"])
            .fails_with_code(2);

        assert_eq!(
            result.stderr_as_displayed(),
            "\
[: invalid integer 'zap'
   ╭─[ [:1:7 ]
   │
 1 │ 7 -eq zap
   │       ───
   │
   │ Help: -eq, -ne, -lt, -le, -gt and -ge compare integers; use =, !=, < or > to compare strings
   │       -eq equal, -ne not equal, -lt less than, -le less than or equal, -gt greater than, -ge greater than or equal
───╯"
        );
    }
}
