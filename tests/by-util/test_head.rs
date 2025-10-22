// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (words) bogusfile emptyfile abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstu
// spell-checker:ignore (words) seekable

#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "macos"),
    not(target_os = "android"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))]
use std::io::Read;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;
static INPUT: &str = "lorem_ipsum.txt";

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_stdin_default() {
    new_ucmd!()
        .pipe_in_fixture(INPUT)
        .succeeds()
        .stdout_is_fixture("lorem_ipsum_default.expected");
}

#[test]
fn test_stdin_1_line_obsolete() {
    new_ucmd!()
        .args(&["-1"])
        .pipe_in_fixture(INPUT)
        .succeeds()
        .stdout_is_fixture("lorem_ipsum_1_line.expected");
}

#[test]
fn test_stdin_1_line() {
    new_ucmd!()
        .args(&["-n", "1"])
        .pipe_in_fixture(INPUT)
        .succeeds()
        .stdout_is_fixture("lorem_ipsum_1_line.expected");
}

#[test]
fn test_stdin_negative_23_line() {
    new_ucmd!()
        .args(&["-n", "-23"])
        .pipe_in_fixture(INPUT)
        .succeeds()
        .stdout_is_fixture("lorem_ipsum_1_line.expected");
}

#[test]
fn test_stdin_5_chars() {
    new_ucmd!()
        .args(&["-c", "5"])
        .pipe_in_fixture(INPUT)
        .succeeds()
        .stdout_is_fixture("lorem_ipsum_5_chars.expected");
}

#[test]
fn test_single_default() {
    new_ucmd!()
        .arg(INPUT)
        .succeeds()
        .stdout_is_fixture("lorem_ipsum_default.expected");
}

#[test]
fn test_single_1_line_obsolete() {
    new_ucmd!()
        .args(&["-1", INPUT])
        .succeeds()
        .stdout_is_fixture("lorem_ipsum_1_line.expected");
}

#[test]
fn test_single_1_line() {
    new_ucmd!()
        .args(&["-n", "1", INPUT])
        .succeeds()
        .stdout_is_fixture("lorem_ipsum_1_line.expected");
}

#[test]
fn test_single_1_line_presume_input_pipe() {
    new_ucmd!()
        .args(&["---presume-input-pipe", "-n", "1", INPUT])
        .succeeds()
        .stdout_is_fixture("lorem_ipsum_1_line.expected");
}

#[test]
fn test_single_5_chars() {
    new_ucmd!()
        .args(&["-c", "5", INPUT])
        .succeeds()
        .stdout_is_fixture("lorem_ipsum_5_chars.expected");
}

#[test]
fn test_verbose() {
    new_ucmd!()
        .args(&["-v", INPUT])
        .succeeds()
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
        .succeeds()
        .stdout_is("a");
}

#[test]
fn test_line_syntax() {
    new_ucmd!()
        .args(&["-n", "2048m"])
        .pipe_in("a\n")
        .succeeds()
        .stdout_is("a\n");
}

#[test]
fn test_zero_terminated_syntax() {
    new_ucmd!()
        .args(&["-z", "-n", "1"])
        .pipe_in("x\0y")
        .succeeds()
        .stdout_is("x\0");
}

#[test]
fn test_zero_terminated_syntax_2() {
    new_ucmd!()
        .args(&["-z", "-n", "2"])
        .pipe_in("x\0y")
        .succeeds()
        .stdout_is("x\0y");
}

#[test]
fn test_non_terminated_input() {
    new_ucmd!()
        .args(&["-n", "-1"])
        .pipe_in("x\ny")
        .succeeds()
        .stdout_is("x\n");
}

#[test]
fn test_zero_terminated_negative_lines() {
    new_ucmd!()
        .args(&["-z", "-n", "-1"])
        .pipe_in("x\0y\0z\0")
        .succeeds()
        .stdout_is("x\0y\0");
}

#[test]
fn test_negative_byte_syntax() {
    new_ucmd!()
        .args(&["--bytes=-2"])
        .pipe_in("a\n")
        .succeeds()
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
        .succeeds()
        .stdout_is_fixture("sequence.expected");
}
#[test]
fn test_file_backwards() {
    new_ucmd!()
        .args(&["-c", "-10", "lorem_ipsum.txt"])
        .succeeds()
        .stdout_is_fixture("lorem_ipsum_backwards_file.expected");
}

#[test]
fn test_zero_terminated() {
    new_ucmd!()
        .args(&["-z", "zero_terminated.txt"])
        .succeeds()
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
        .succeeds()
        .no_output();
    new_ucmd!()
        .args(&["-n", "1024R", "emptyfile.txt"])
        .succeeds()
        .no_output();
    new_ucmd!()
        .args(&["-c", "1Y", "emptyfile.txt"])
        .succeeds()
        .no_output();
    new_ucmd!()
        .args(&["-n", "1Y", "emptyfile.txt"])
        .succeeds()
        .no_output();
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
            new_ucmd!().args(&["-c", size]).succeeds().no_output();
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
        .succeeds()
        .stdout_is_fixture("lorem_ipsum_default.expected");
}

#[test]
fn test_presume_input_pipe_5_chars() {
    new_ucmd!()
        .args(&["-c", "5", "---presume-input-pipe"])
        .pipe_in_fixture(INPUT)
        .succeeds()
        .stdout_is_fixture("lorem_ipsum_5_chars.expected");
}

#[test]
fn test_all_but_last_bytes_large_file_piped() {
    // Validate print-all-but-last-n-bytes with a large piped-in (i.e. non-seekable) file.
    let scene = TestScenario::new(util_name!());
    let fixtures = &scene.fixtures;

    // First, create all our fixtures.
    let seq_20000_file_name = "seq_20000";
    let seq_19000_file_name = "seq_19000";
    let seq_19001_20000_file_name = "seq_19001_20000";
    scene
        .cmd("seq")
        .arg("20000")
        .set_stdout(fixtures.make_file(seq_20000_file_name))
        .succeeds();
    scene
        .cmd("seq")
        .arg("19000")
        .set_stdout(fixtures.make_file(seq_19000_file_name))
        .succeeds();
    scene
        .cmd("seq")
        .args(&["19001", "20000"])
        .set_stdout(fixtures.make_file(seq_19001_20000_file_name))
        .succeeds();

    let seq_19001_20000_file_length = fixtures
        .open(seq_19001_20000_file_name)
        .metadata()
        .unwrap()
        .len();
    scene
        .ucmd()
        .args(&["-c", &format!("-{seq_19001_20000_file_length}")])
        .pipe_in_fixture(seq_20000_file_name)
        .succeeds()
        .stdout_only_fixture(seq_19000_file_name);
}

#[test]
fn test_all_but_last_lines_large_file() {
    // Create our fixtures on the fly. We need the input file to be at least double
    // the size of BUF_SIZE as specified in head.rs. Go for something a bit bigger
    // than that.
    let scene = TestScenario::new(util_name!());
    let fixtures = &scene.fixtures;
    let seq_20000_file_name = "seq_20000";
    let seq_20000_truncated_file_name = "seq_20000_truncated";
    let seq_1000_file_name = "seq_1000";
    scene
        .cmd("seq")
        .arg("20000")
        .set_stdout(fixtures.make_file(seq_20000_file_name))
        .succeeds();
    // Create a file the same as seq_20000 except for the final terminating endline.
    scene
        .ucmd()
        .args(&["-c", "-1", seq_20000_file_name])
        .set_stdout(fixtures.make_file(seq_20000_truncated_file_name))
        .succeeds();
    scene
        .cmd("seq")
        .arg("1000")
        .set_stdout(fixtures.make_file(seq_1000_file_name))
        .succeeds();

    // Now run our tests.
    scene
        .ucmd()
        .args(&["-n", "-19000", seq_20000_file_name])
        .succeeds()
        .stdout_only_fixture(seq_1000_file_name);

    scene
        .ucmd()
        .args(&["-n", "-20000", seq_20000_file_name])
        .succeeds()
        .stdout_only_fixture("emptyfile.txt");

    scene
        .ucmd()
        .args(&["-n", "-20001", seq_20000_file_name])
        .succeeds()
        .stdout_only_fixture("emptyfile.txt");

    // Confirm correct behavior when the input file doesn't end with a newline.
    scene
        .ucmd()
        .args(&["-n", "-19000", seq_20000_truncated_file_name])
        .succeeds()
        .stdout_only_fixture(seq_1000_file_name);

    scene
        .ucmd()
        .args(&["-n", "-20000", seq_20000_truncated_file_name])
        .succeeds()
        .stdout_only_fixture("emptyfile.txt");

    scene
        .ucmd()
        .args(&["-n", "-20001", seq_20000_truncated_file_name])
        .succeeds()
        .stdout_only_fixture("emptyfile.txt");
}

#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "macos"),
    not(target_os = "android"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))]
#[test]
fn test_validate_stdin_offset_lines() {
    // A handful of unix-only tests to validate behavior when reading from stdin on a seekable
    // file. GNU-compatibility requires that the stdin file be left such that if another
    // process is invoked on the same stdin file after head has run, the subsequent file should
    // start reading from the byte after the last byte printed by head.
    // Since this is unix-only requirement, keep this as a separate test rather than adding a
    // conditionally-compiled segment to multiple tests.
    //
    // Test scenarios...
    // 1 - Print the first n lines
    // 2 - Print all-but the last n lines
    // 3 - Print all but the last n lines, large file.
    let scene = TestScenario::new(util_name!());
    let fixtures = &scene.fixtures;

    // Test 1 - Print the first n lines
    fixtures.write("f1", "a\nb\nc\n");
    let file = fixtures.open("f1");
    let mut file_shadow = file.try_clone().unwrap();
    scene
        .ucmd()
        .args(&["-n", "1"])
        .set_stdin(file)
        .succeeds()
        .stdout_only("a\n");
    let mut bytes_remaining_in_stdin = vec![];
    assert_eq!(
        file_shadow
            .read_to_end(&mut bytes_remaining_in_stdin)
            .unwrap(),
        4
    );
    assert_eq!(
        String::from_utf8(bytes_remaining_in_stdin).unwrap(),
        "b\nc\n"
    );

    // Test 2 - Print all-but the last n lines
    fixtures.write("f2", "a\nb\nc\n");
    let file = fixtures.open("f2");
    let mut file_shadow = file.try_clone().unwrap();
    scene
        .ucmd()
        .args(&["-n", "-1"])
        .set_stdin(file)
        .succeeds()
        .stdout_only("a\nb\n");
    let mut bytes_remaining_in_stdin = vec![];
    assert_eq!(
        file_shadow
            .read_to_end(&mut bytes_remaining_in_stdin)
            .unwrap(),
        2
    );
    assert_eq!(String::from_utf8(bytes_remaining_in_stdin).unwrap(), "c\n");

    // Test 3 - Print all but the last n lines, large input file.
    // First, create all our fixtures.
    let seq_20000_file_name = "seq_20000";
    let seq_1000_file_name = "seq_1000";
    let seq_1001_20000_file_name = "seq_1001_20000";
    scene
        .cmd("seq")
        .arg("20000")
        .set_stdout(fixtures.make_file(seq_20000_file_name))
        .succeeds();
    scene
        .cmd("seq")
        .arg("1000")
        .set_stdout(fixtures.make_file(seq_1000_file_name))
        .succeeds();
    scene
        .cmd("seq")
        .args(&["1001", "20000"])
        .set_stdout(fixtures.make_file(seq_1001_20000_file_name))
        .succeeds();

    let file = fixtures.open(seq_20000_file_name);
    let file_shadow = file.try_clone().unwrap();
    scene
        .ucmd()
        .args(&["-n", "-19000"])
        .set_stdin(file)
        .succeeds()
        .stdout_only_fixture(seq_1000_file_name);
    scene
        .cmd("cat")
        .set_stdin(file_shadow)
        .succeeds()
        .stdout_only_fixture(seq_1001_20000_file_name);
}

#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "macos"),
    not(target_os = "android"),
    not(target_os = "freebsd"),
    not(target_os = "openbsd")
))]
#[test]
fn test_validate_stdin_offset_bytes() {
    // A handful of unix-only tests to validate behavior when reading from stdin on a seekable
    // file. GNU-compatibility requires that the stdin file be left such that if another
    // process is invoked on the same stdin file after head has run, the subsequent file should
    // start reading from the byte after the last byte printed by head.
    // Since this is unix-only requirement, keep this as a separate test rather than adding a
    // conditionally-compiled segment to multiple tests.
    //
    // Test scenarios...
    // 1 - Print the first n bytes
    // 2 - Print all-but the last n bytes
    // 3 - Print all-but the last n bytes, with n=0 (i.e. print everything)
    // 4 - Print all but the last n bytes, large file.
    let scene = TestScenario::new(util_name!());
    let fixtures = &scene.fixtures;

    // Test 1 - Print the first n bytes
    fixtures.write("f1", "abc\ndef\n");
    let file = fixtures.open("f1");
    let mut file_shadow = file.try_clone().unwrap();
    scene
        .ucmd()
        .args(&["-c", "2"])
        .set_stdin(file)
        .succeeds()
        .stdout_only("ab");
    let mut bytes_remaining_in_stdin = vec![];
    assert_eq!(
        file_shadow
            .read_to_end(&mut bytes_remaining_in_stdin)
            .unwrap(),
        6
    );
    assert_eq!(
        String::from_utf8(bytes_remaining_in_stdin).unwrap(),
        "c\ndef\n"
    );

    // Test 2 - Print all-but the last n bytes
    fixtures.write("f2", "abc\ndef\n");
    let file = fixtures.open("f2");
    let mut file_shadow = file.try_clone().unwrap();
    scene
        .ucmd()
        .args(&["-c", "-3"])
        .set_stdin(file)
        .succeeds()
        .stdout_only("abc\nd");
    let mut bytes_remaining_in_stdin = vec![];
    assert_eq!(
        file_shadow
            .read_to_end(&mut bytes_remaining_in_stdin)
            .unwrap(),
        3
    );
    assert_eq!(String::from_utf8(bytes_remaining_in_stdin).unwrap(), "ef\n");

    // Test 3 - Print all-but the last n bytes, n=0 (i.e. print everything)
    fixtures.write("f3", "abc\ndef\n");
    let file = fixtures.open("f3");
    let mut file_shadow = file.try_clone().unwrap();
    scene
        .ucmd()
        .args(&["-c", "-0"])
        .set_stdin(file)
        .succeeds()
        .stdout_only("abc\ndef\n");
    let mut bytes_remaining_in_stdin = vec![];
    assert_eq!(
        file_shadow
            .read_to_end(&mut bytes_remaining_in_stdin)
            .unwrap(),
        0
    );
    assert_eq!(String::from_utf8(bytes_remaining_in_stdin).unwrap(), "");

    // Test 4 - Print all but the last n bytes, large input file.
    // First, create all our fixtures.
    let seq_20000_file_name = "seq_20000";
    let seq_19000_file_name = "seq_19000";
    let seq_19001_20000_file_name = "seq_19001_20000";
    scene
        .cmd("seq")
        .arg("20000")
        .set_stdout(fixtures.make_file(seq_20000_file_name))
        .succeeds();
    scene
        .cmd("seq")
        .arg("19000")
        .set_stdout(fixtures.make_file(seq_19000_file_name))
        .succeeds();
    scene
        .cmd("seq")
        .args(&["19001", "20000"])
        .set_stdout(fixtures.make_file(seq_19001_20000_file_name))
        .succeeds();

    let file = fixtures.open(seq_20000_file_name);
    let file_shadow = file.try_clone().unwrap();
    let seq_19001_20000_file_length = fixtures
        .open(seq_19001_20000_file_name)
        .metadata()
        .unwrap()
        .len();
    scene
        .ucmd()
        .args(&["-c", &format!("-{seq_19001_20000_file_length}")])
        .set_stdin(file)
        .succeeds()
        .stdout_only_fixture(seq_19000_file_name);
    scene
        .cmd("cat")
        .set_stdin(file_shadow)
        .succeeds()
        .stdout_only_fixture(seq_19001_20000_file_name);
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

    // Only expect output if the file is not empty, e.g. it is empty in default WSL2.
    if !ts.fixtures.read("/proc/modules").is_empty() {
        assert!(!result.stdout().is_empty());
    }
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

    // Only expect output if the file is not empty, e.g. it is empty in default WSL2.
    if !ts.fixtures.read("/proc/modules").is_empty() {
        assert!(!result.stdout().is_empty());
    }
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
    // in case the kernel was not built with profiling support, e.g. WSL
    if !ts.fixtures.file_exists("/sys/kernel/profiling") {
        println!("test skipped: /sys/kernel/profiling does not exist");
        return;
    }
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
        .succeeds();
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
                .fails()
                .stderr_contains("error writing 'standard output': No space left on device");
        }
    }
}

#[test]
#[cfg(target_os = "linux")]
fn test_head_non_utf8_paths() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    // Create a test file with non-UTF-8 bytes in the name
    let non_utf8_bytes = b"test_\xFF\xFE.txt";
    let non_utf8_name = OsStr::from_bytes(non_utf8_bytes);

    std::fs::write(at.plus(non_utf8_name), "line1\nline2\nline3\n").unwrap();

    let result = scene.ucmd().arg(non_utf8_name).succeeds();

    let output = result.stdout_str_lossy();
    assert!(output.contains("line1"));
    assert!(output.contains("line2"));
    assert!(output.contains("line3"));
}
// Test that head handles non-UTF-8 file names without crashing
