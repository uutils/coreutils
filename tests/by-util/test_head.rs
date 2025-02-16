// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (words) bogusfile emptyfile abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstu

use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

static INPUT: &str = "lorem_ipsum.txt";

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_stdin_default() {
    new_ucmd!()
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("lorem_ipsum_default.expected");
}

#[test]
fn test_stdin_1_line_obsolete() {
    new_ucmd!()
        .args(&["-1"])
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("lorem_ipsum_1_line.expected");
}

#[test]
fn test_stdin_1_line() {
    new_ucmd!()
        .args(&["-n", "1"])
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("lorem_ipsum_1_line.expected");
}

#[test]
fn test_stdin_negative_23_line() {
    new_ucmd!()
        .args(&["-n", "-23"])
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("lorem_ipsum_1_line.expected");
}

#[test]
fn test_stdin_5_chars() {
    new_ucmd!()
        .args(&["-c", "5"])
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("lorem_ipsum_5_chars.expected");
}

#[test]
fn test_single_default() {
    new_ucmd!()
        .arg(INPUT)
        .run()
        .stdout_is_fixture("lorem_ipsum_default.expected");
}

#[test]
fn test_single_1_line_obsolete() {
    new_ucmd!()
        .args(&["-1", INPUT])
        .run()
        .stdout_is_fixture("lorem_ipsum_1_line.expected");
}

#[test]
fn test_single_1_line() {
    new_ucmd!()
        .args(&["-n", "1", INPUT])
        .run()
        .stdout_is_fixture("lorem_ipsum_1_line.expected");
}

#[test]
fn test_single_5_chars() {
    new_ucmd!()
        .args(&["-c", "5", INPUT])
        .run()
        .stdout_is_fixture("lorem_ipsum_5_chars.expected");
}

#[test]
fn test_verbose() {
    new_ucmd!()
        .args(&["-v", INPUT])
        .run()
        .stdout_is_fixture("lorem_ipsum_verbose.expected");
}

#[test]
fn test_spams_newline() {
    new_ucmd!().pipe_in("a").succeeds().stdout_is("a");
}

#[test]
fn test_byte_syntax() {
    new_ucmd!()
        .args(&["-1c"])
        .pipe_in("abc")
        .run()
        .stdout_is("a");
}

#[test]
fn test_line_syntax() {
    new_ucmd!()
        .args(&["-n", "2048m"])
        .pipe_in("a\n")
        .run()
        .stdout_is("a\n");
}

#[test]
fn test_zero_terminated_syntax() {
    new_ucmd!()
        .args(&["-z", "-n", "1"])
        .pipe_in("x\0y")
        .run()
        .stdout_is("x\0");
}

#[test]
fn test_zero_terminated_syntax_2() {
    new_ucmd!()
        .args(&["-z", "-n", "2"])
        .pipe_in("x\0y")
        .run()
        .stdout_is("x\0y");
}

#[test]
fn test_zero_terminated_negative_lines() {
    new_ucmd!()
        .args(&["-z", "-n", "-1"])
        .pipe_in("x\0y\0z\0")
        .run()
        .stdout_is("x\0y\0");
}

#[test]
fn test_negative_byte_syntax() {
    new_ucmd!()
        .args(&["--bytes=-2"])
        .pipe_in("a\n")
        .run()
        .stdout_is("");
}

#[test]
fn test_negative_bytes_greater_than_input_size_stdin() {
    new_ucmd!()
        .args(&["-c", "-2"])
        .pipe_in("a")
        .succeeds()
        .no_output();
}

#[test]
fn test_negative_bytes_greater_than_input_size_file() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.write_bytes("f", b"a");
    ts.ucmd().args(&["-c", "-2", "f"]).succeeds().no_output();
}

#[test]
fn test_negative_zero_lines() {
    new_ucmd!()
        .arg("--lines=-0")
        .pipe_in("a\nb\n")
        .succeeds()
        .stdout_is("a\nb\n");
    new_ucmd!()
        .arg("--lines=-0")
        .pipe_in("a\nb")
        .succeeds()
        .stdout_is("a\nb");
}

#[test]
fn test_negative_zero_bytes() {
    new_ucmd!()
        .args(&["--bytes=-0"])
        .pipe_in("qwerty")
        .succeeds()
        .stdout_is("qwerty");
}
#[test]
fn test_no_such_file_or_directory() {
    new_ucmd!()
        .arg("no_such_file.toml")
        .fails()
        .stderr_contains("cannot open 'no_such_file.toml' for reading: No such file or directory");
}

#[test]
fn test_lines_leading_zeros() {
    new_ucmd!()
        .arg("--lines=010")
        .pipe_in("\n\n\n\n\n\n\n\n\n\n\n\n")
        .succeeds()
        .stdout_is("\n\n\n\n\n\n\n\n\n\n");
}

/// Test that each non-existent files gets its own error message printed.
#[test]
fn test_multiple_nonexistent_files() {
    new_ucmd!()
        .args(&["bogusfile1", "bogusfile2"])
        .fails()
        .stdout_does_not_contain("==> bogusfile1 <==")
        .stderr_contains("cannot open 'bogusfile1' for reading: No such file or directory")
        .stdout_does_not_contain("==> bogusfile2 <==")
        .stderr_contains("cannot open 'bogusfile2' for reading: No such file or directory");
}

// there was a bug not caught by previous tests
// where for negative n > 3, the total amount of lines
// was correct, but it would eat from the second line
#[test]
fn test_sequence_fixture() {
    new_ucmd!()
        .args(&["-n", "-10", "sequence"])
        .run()
        .stdout_is_fixture("sequence.expected");
}
#[test]
fn test_file_backwards() {
    new_ucmd!()
        .args(&["-c", "-10", "lorem_ipsum.txt"])
        .run()
        .stdout_is_fixture("lorem_ipsum_backwards_file.expected");
}

#[test]
fn test_zero_terminated() {
    new_ucmd!()
        .args(&["-z", "zero_terminated.txt"])
        .run()
        .stdout_is_fixture("zero_terminated.expected");
}

#[test]
fn test_obsolete_extras() {
    new_ucmd!()
        .args(&["-5zv"])
        .pipe_in("1\x002\x003\x004\x005\x006")
        .succeeds()
        .stdout_is("==> standard input <==\n1\x002\x003\x004\x005\0");
}

#[test]
fn test_multiple_files() {
    new_ucmd!()
        .args(&["emptyfile.txt", "emptyfile.txt"])
        .succeeds()
        .stdout_is("==> emptyfile.txt <==\n\n==> emptyfile.txt <==\n");
}

#[test]
fn test_multiple_files_with_stdin() {
    new_ucmd!()
        .args(&["emptyfile.txt", "-", "emptyfile.txt"])
        .pipe_in("hello\n")
        .succeeds()
        .stdout_is(
            "==> emptyfile.txt <==

==> standard input <==
hello

==> emptyfile.txt <==
",
        );
}

#[test]
fn test_bad_utf8() {
    let bytes: &[u8] = b"\xfc\x80\x80\x80\x80\xaf";
    new_ucmd!()
        .args(&["-c", "6"])
        .pipe_in(bytes)
        .succeeds()
        .stdout_is_bytes(bytes);
}

#[test]
fn test_bad_utf8_lines() {
    let input: &[u8] =
        b"\xfc\x80\x80\x80\x80\xaf\nb\xfc\x80\x80\x80\x80\xaf\nb\xfc\x80\x80\x80\x80\xaf";
    let output = b"\xfc\x80\x80\x80\x80\xaf\nb\xfc\x80\x80\x80\x80\xaf\n";
    new_ucmd!()
        .args(&["-n", "2"])
        .pipe_in(input)
        .succeeds()
        .stdout_is_bytes(output);
}

#[test]
fn test_head_invalid_num() {
    new_ucmd!()
        .args(&["-c", "1024R", "emptyfile.txt"])
        .fails()
        .stderr_is(
            "head: invalid number of bytes: '1024R': Value too large for defined data type\n",
        );
    new_ucmd!()
        .args(&["-n", "1024R", "emptyfile.txt"])
        .fails()
        .stderr_is(
            "head: invalid number of lines: '1024R': Value too large for defined data type\n",
        );
    new_ucmd!()
        .args(&["-c", "1Y", "emptyfile.txt"])
        .fails()
        .stderr_is("head: invalid number of bytes: '1Y': Value too large for defined data type\n");
    new_ucmd!()
        .args(&["-n", "1Y", "emptyfile.txt"])
        .fails()
        .stderr_is("head: invalid number of lines: '1Y': Value too large for defined data type\n");
    #[cfg(target_pointer_width = "32")]
    {
        let sizes = ["1000G", "10T"];
        for size in &sizes {
            new_ucmd!().args(&["-c", size]).succeeds();
        }
    }
    #[cfg(target_pointer_width = "32")]
    {
        let sizes = ["-1000G", "-10T"];
        for size in &sizes {
            new_ucmd!()
                .args(&["-c", size])
                .fails()
                .stderr_is("head: out of range integral type conversion attempted: number of -bytes or -lines is too large\n");
        }
    }
    new_ucmd!()
        .args(&["-c", "-³"])
        .fails()
        .stderr_is("head: invalid number of bytes: '³'\n");
}

#[test]
fn test_head_num_with_undocumented_sign_bytes() {
    // tail: '-' is not documented (8.32 man pages)
    // head: '+' is not documented (8.32 man pages)
    const ALPHABET: &str = "abcdefghijklmnopqrstuvwxyz";
    new_ucmd!()
        .args(&["-c", "5"])
        .pipe_in(ALPHABET)
        .succeeds()
        .stdout_is("abcde");
    new_ucmd!()
        .args(&["-c", "-5"])
        .pipe_in(ALPHABET)
        .succeeds()
        .stdout_is("abcdefghijklmnopqrstu");
    new_ucmd!()
        .args(&["-c", "+5"])
        .pipe_in(ALPHABET)
        .succeeds()
        .stdout_is("abcde");
}

#[test]
fn test_presume_input_pipe_default() {
    new_ucmd!()
        .args(&["---presume-input-pipe"])
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("lorem_ipsum_default.expected");
}

#[test]
fn test_presume_input_pipe_5_chars() {
    new_ucmd!()
        .args(&["-c", "5", "---presume-input-pipe"])
        .pipe_in_fixture(INPUT)
        .run()
        .stdout_is_fixture("lorem_ipsum_5_chars.expected");
}

#[test]
fn test_read_backwards_lines_large_file() {
    // Create our fixtures on the fly. We need the input file to be at least double
    // the size of BUF_SIZE as specified in head.rs. Go for something a bit bigger
    // than that.
    let scene = TestScenario::new(util_name!());
    let fixtures = &scene.fixtures;
    let seq_30000_file_name = "seq_30000";
    let seq_1000_file_name = "seq_1000";
    scene
        .cmd("seq")
        .arg("30000")
        .set_stdout(fixtures.make_file(seq_30000_file_name))
        .succeeds();
    scene
        .cmd("seq")
        .arg("1000")
        .set_stdout(fixtures.make_file(seq_1000_file_name))
        .succeeds();

    // Now run our tests.
    scene
        .ucmd()
        .args(&["-n", "-29000", "seq_30000"])
        .succeeds()
        .stdout_is_fixture("seq_1000");

    scene
        .ucmd()
        .args(&["-n", "-30000", "seq_30000"])
        .run()
        .stdout_is_fixture("emptyfile.txt");

    scene
        .ucmd()
        .args(&["-n", "-30001", "seq_30000"])
        .run()
        .stdout_is_fixture("emptyfile.txt");
}

#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "macos"),
    not(target_os = "android"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))]
#[test]
fn test_read_backwards_bytes_proc_fs_version() {
    let ts = TestScenario::new(util_name!());

    let args = ["-c", "-1", "/proc/version"];
    let result = ts.ucmd().args(&args).succeeds();
    assert!(!result.stdout().is_empty());
}

#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "macos"),
    not(target_os = "android"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))]
#[test]
fn test_read_backwards_bytes_proc_fs_modules() {
    let ts = TestScenario::new(util_name!());

    let args = ["-c", "-1", "/proc/modules"];
    let result = ts.ucmd().args(&args).succeeds();
    assert!(!result.stdout().is_empty());
}

#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "macos"),
    not(target_os = "android"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))]
#[test]
fn test_read_backwards_lines_proc_fs_modules() {
    let ts = TestScenario::new(util_name!());

    let args = ["--lines", "-1", "/proc/modules"];
    let result = ts.ucmd().args(&args).succeeds();
    assert!(!result.stdout().is_empty());
}

#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "macos"),
    not(target_os = "android"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))]
#[test]
fn test_read_backwards_bytes_sys_kernel_profiling() {
    let ts = TestScenario::new(util_name!());

    let args = ["-c", "-1", "/sys/kernel/profiling"];
    let result = ts.ucmd().args(&args).succeeds();
    let stdout_str = result.stdout_str();
    assert_eq!(stdout_str.len(), 1);
    assert!(stdout_str == "0" || stdout_str == "1");
}

#[test]
fn test_value_too_large() {
    const MAX: u64 = u64::MAX;

    new_ucmd!()
        .args(&["-n", format!("{MAX}0").as_str(), "lorem_ipsum.txt"])
        .fails()
        .stderr_contains("Value too large for defined data type");
}

#[test]
fn test_all_but_last_lines() {
    new_ucmd!()
        .args(&["-n", "-15", "lorem_ipsum.txt"])
        .succeeds()
        .stdout_is_fixture("lorem_ipsum_backwards_15_lines.expected");
}

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "netbsd"))]
#[test]
fn test_write_to_dev_full() {
    use std::fs::OpenOptions;

    for append in [true, false] {
        {
            let dev_full = OpenOptions::new()
                .write(true)
                .append(append)
                .open("/dev/full")
                .unwrap();

            new_ucmd!()
                .pipe_in_fixture(INPUT)
                .set_stdout(dev_full)
                .run()
                .stderr_contains("No space left on device");
        }
    }
}
