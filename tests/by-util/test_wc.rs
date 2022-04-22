#[cfg(all(unix, not(target_os = "macos")))]
use pretty_assertions::assert_ne;

use crate::common::util::*;

// spell-checker:ignore (flags) lwmcL clmwL ; (path) bogusfile emptyfile manyemptylines moby notrailingnewline onelongemptyline onelongword weirdchars

#[test]
fn test_count_bytes_large_stdin() {
    for n in [
        0,
        1,
        42,
        16 * 1024 - 7,
        16 * 1024 - 1,
        16 * 1024,
        16 * 1024 + 1,
        16 * 1024 + 3,
        32 * 1024,
        64 * 1024,
        80 * 1024,
        96 * 1024,
        112 * 1024,
        128 * 1024,
    ] {
        let data = vec_of_size(n);
        let expected = format!("{}\n", n);
        new_ucmd!()
            .args(&["-c"])
            .pipe_in(data)
            .succeeds()
            .stdout_is_bytes(&expected.as_bytes());
    }
}

#[test]
fn test_stdin_default() {
    new_ucmd!()
        .pipe_in_fixture("lorem_ipsum.txt")
        .run()
        .stdout_is("     13     109     772\n");
}

#[test]
fn test_stdin_explicit() {
    new_ucmd!()
        .pipe_in_fixture("lorem_ipsum.txt")
        .arg("-")
        .run()
        .stdout_is("     13     109     772 -\n");
}

#[test]
fn test_utf8() {
    new_ucmd!()
        .args(&["-lwmcL"])
        .pipe_in_fixture("UTF_8_test.txt")
        .run()
        .stdout_is("    303    2119   22457   23025      79\n");
}

#[test]
fn test_utf8_extra() {
    new_ucmd!()
        .arg("-lwmcL")
        .pipe_in_fixture("UTF_8_weirdchars.txt")
        .run()
        .stdout_is("     25      87     442     513      48\n");
}

#[test]
fn test_stdin_line_len_regression() {
    new_ucmd!()
        .args(&["-L"])
        .pipe_in("\n123456")
        .run()
        .stdout_is("6\n");
}

#[test]
fn test_stdin_only_bytes() {
    new_ucmd!()
        .args(&["-c"])
        .pipe_in_fixture("lorem_ipsum.txt")
        .run()
        .stdout_is("772\n");
}

#[test]
fn test_stdin_all_counts() {
    new_ucmd!()
        .args(&["-c", "-m", "-l", "-L", "-w"])
        .pipe_in_fixture("alice_in_wonderland.txt")
        .run()
        .stdout_is("      5      57     302     302      66\n");
}

#[test]
fn test_single_default() {
    new_ucmd!()
        .arg("moby_dick.txt")
        .run()
        .stdout_is("  18  204 1115 moby_dick.txt\n");
}

#[test]
fn test_single_only_lines() {
    new_ucmd!()
        .args(&["-l", "moby_dick.txt"])
        .run()
        .stdout_is("18 moby_dick.txt\n");
}

#[test]
fn test_single_all_counts() {
    new_ucmd!()
        .args(&["-c", "-l", "-L", "-m", "-w", "alice_in_wonderland.txt"])
        .run()
        .stdout_is("  5  57 302 302  66 alice_in_wonderland.txt\n");
}

#[test]
fn test_multiple_default() {
    new_ucmd!()
        .args(&[
            "lorem_ipsum.txt",
            "moby_dick.txt",
            "alice_in_wonderland.txt",
        ])
        .run()
        .stdout_is(
            "  13  109  772 lorem_ipsum.txt\n  18  204 1115 moby_dick.txt\n   5   57  302 \
             alice_in_wonderland.txt\n  36  370 2189 total\n",
        );
}

/// Test for an empty file.
#[test]
fn test_file_empty() {
    new_ucmd!()
        .args(&["-clmwL", "emptyfile.txt"])
        .run()
        .stdout_is("0 0 0 0 0 emptyfile.txt\n");
}

/// Test for an file containing a single non-whitespace character
/// *without* a trailing newline.
#[test]
fn test_file_single_line_no_trailing_newline() {
    new_ucmd!()
        .args(&["-clmwL", "notrailingnewline.txt"])
        .run()
        .stdout_is("1 1 2 2 1 notrailingnewline.txt\n");
}

/// Test for a file that has 100 empty lines (that is, the contents of
/// the file are the newline character repeated one hundred times).
#[test]
fn test_file_many_empty_lines() {
    new_ucmd!()
        .args(&["-clmwL", "manyemptylines.txt"])
        .run()
        .stdout_is("100   0 100 100   0 manyemptylines.txt\n");
}

/// Test for a file that has one long line comprising only spaces.
#[test]
fn test_file_one_long_line_only_spaces() {
    new_ucmd!()
        .args(&["-clmwL", "onelongemptyline.txt"])
        .run()
        .stdout_is("    1     0 10001 10001 10000 onelongemptyline.txt\n");
}

/// Test for a file that has one long line comprising a single "word".
#[test]
fn test_file_one_long_word() {
    new_ucmd!()
        .args(&["-clmwL", "onelongword.txt"])
        .run()
        .stdout_is("    1     1 10001 10001 10000 onelongword.txt\n");
}

/// Test that the total size of all the files in the input dictates
/// the display width.
///
/// The width in digits of any count is the width in digits of the
/// number of bytes in the file, regardless of whether the number of
/// bytes are displayed.
#[test]
fn test_file_bytes_dictate_width() {
    // This file has 10,001 bytes. Five digits are required to
    // represent that. Even though the number of lines is 1 and the
    // number of words is 0, each of those counts is formatted with
    // five characters, filled with whitespace.
    new_ucmd!()
        .args(&["-lw", "onelongemptyline.txt"])
        .run()
        .stdout_is("    1     0 onelongemptyline.txt\n");

    // This file has zero bytes. Only one digit is required to
    // represent that.
    new_ucmd!()
        .args(&["-lw", "emptyfile.txt"])
        .run()
        .stdout_is("0 0 emptyfile.txt\n");

    // lorem_ipsum.txt contains 772 bytes, and alice_in_wonderland.txt contains
    // 302 bytes. The total is 1074 bytes, which has a width of 4
    new_ucmd!()
        .args(&["-lwc", "alice_in_wonderland.txt", "lorem_ipsum.txt"])
        .run()
        .stdout_is(
            "   5   57  302 alice_in_wonderland.txt\n  13  109  772 \
                    lorem_ipsum.txt\n  18  166 1074 total\n",
        );

    // . is a directory, so minimum_width should get set to 7
    #[cfg(not(windows))]
    const STDOUT: &str = "      0       0       0 emptyfile.txt\n      0       0       0 \
                          .\n      0       0       0 total\n";
    #[cfg(windows)]
    const STDOUT: &str = "      0       0       0 emptyfile.txt\n      0       0       0 total\n";
    new_ucmd!()
        .args(&["-lwc", "emptyfile.txt", "."])
        .run()
        .stdout_is(STDOUT);
}

/// Test that getting counts from a directory is an error.
#[test]
fn test_read_from_directory_error() {
    #[cfg(not(windows))]
    const STDERR: &str = ".: Is a directory";
    #[cfg(windows)]
    const STDERR: &str = ".: Access is denied";

    #[cfg(not(windows))]
    const STDOUT: &str = "      0       0       0 .\n";
    #[cfg(windows)]
    const STDOUT: &str = "";

    new_ucmd!()
        .args(&["."])
        .fails()
        .stderr_contains(STDERR)
        .stdout_is(STDOUT);
}

/// Test that getting counts from nonexistent file is an error.
#[test]
fn test_read_from_nonexistent_file() {
    #[cfg(not(windows))]
    const MSG: &str = "bogusfile: No such file or directory";
    #[cfg(windows)]
    const MSG: &str = "bogusfile: The system cannot find the file specified";
    new_ucmd!()
        .args(&["bogusfile"])
        .fails()
        .stderr_contains(MSG)
        .stdout_is("");
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_files_from_pseudo_filesystem() {
    let result = new_ucmd!().arg("-c").arg("/proc/cpuinfo").succeeds();
    assert_ne!(result.stdout_str(), "0 /proc/cpuinfo\n");
}

#[test]
fn test_files0_disabled_files_argument() {
    const MSG: &str = "file operands cannot be combined with --files0-from";
    new_ucmd!()
        .args(&["--files0-from=files0_list.txt"])
        .arg("lorem_ipsum.txt")
        .fails()
        .stderr_contains(MSG)
        .stdout_is("");
}

#[test]
fn test_files0_from() {
    new_ucmd!()
        .args(&["--files0-from=files0_list.txt"])
        .run()
        .stdout_is(
            "  13  109  772 lorem_ipsum.txt\n  18  204 1115 moby_dick.txt\n   5   57  302 \
             alice_in_wonderland.txt\n  36  370 2189 total\n",
        );
}

#[test]
fn test_files0_from_with_stdin() {
    new_ucmd!()
        .args(&["--files0-from=-"])
        .pipe_in("lorem_ipsum.txt")
        .run()
        .stdout_is(" 13 109 772 lorem_ipsum.txt\n");
}

#[test]
fn test_files0_from_with_stdin_in_file() {
    new_ucmd!()
        .args(&["--files0-from=files0_list_with_stdin.txt"])
        .pipe_in_fixture("alice_in_wonderland.txt")
        .run()
        .stdout_is(
            "     13     109     772 lorem_ipsum.txt\n     18     204    1115 moby_dick.txt\n      5      57     302 \
             -\n     36     370    2189 total\n",
        );
}

#[test]
fn test_files0_from_with_stdin_try_read_from_stdin() {
    const MSG: &str = "when reading file names from stdin, no file name of '-' allowed";
    new_ucmd!()
        .args(&["--files0-from=-"])
        .pipe_in("-")
        .fails()
        .stderr_contains(MSG)
        .stdout_is("");
}
