// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore fullwidth refgh tefgh nefgh unflushed

use bytecount::count;
use rstest::rstest;
use unicode_width::UnicodeWidthChar;
#[cfg(target_os = "linux")]
use uutests::at_and_ucmd;
use uutests::util::TestScenario;
use uutests::{new_ucmd, util_name};

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(wasi_runner, ignore = "WASI: argv/filenames must be valid UTF-8")]
fn test_non_utf8_filename() {
    use std::os::unix::ffi::OsStringExt;

    let (at, mut ucmd) = at_and_ucmd!();

    let filename = std::ffi::OsString::from_vec(vec![0xFF, 0xFE]);
    std::fs::write(at.plus(&filename), b"foo").unwrap();

    ucmd.arg(&filename).succeeds().stdout_only("foo");
}

#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(wasi_runner, ignore = "WASI: argv/filenames must be valid UTF-8")]
fn test_non_utf8_filename_of_nonexisting_file() {
    use std::os::unix::ffi::OsStringExt;

    let filename = std::ffi::OsString::from_vec(vec![0xFF, 0xFE]);

    new_ucmd!()
        .arg(&filename)
        .fails()
        .stderr_only("fold: ''$'\\377\\376': No such file or directory\n");
}

#[test]
fn test_default_80_column_wrap() {
    new_ucmd!()
        .arg("lorem_ipsum.txt")
        .succeeds()
        .stdout_is_fixture("lorem_ipsum_80_column.expected");
}

#[test]
fn test_40_column_hard_cutoff() {
    new_ucmd!()
        .args(&["-w", "40", "lorem_ipsum.txt"])
        .succeeds()
        .stdout_is_fixture("lorem_ipsum_40_column_hard.expected");
}

#[test]
fn test_40_column_word_boundary() {
    new_ucmd!()
        .args(&["-s", "-w", "40", "lorem_ipsum.txt"])
        .succeeds()
        .stdout_is_fixture("lorem_ipsum_40_column_word.expected");
}

#[test]
fn test_default_wrap_with_newlines() {
    new_ucmd!()
        .arg("lorem_ipsum_new_line.txt")
        .succeeds()
        .stdout_is_fixture("lorem_ipsum_new_line_80_column.expected");
}

#[rstest]
#[case::wide_characters_in_column_mode(&["-w", "5"], "\u{B250}\u{B250}\u{B250}\n", "\u{B250}\u{B250}\n\u{B250}\n")]
#[case::wide_characters_with_characters_option(&["--characters", "-w", "5"], "\u{B250}\u{B250}\u{B250}\n", "\u{B250}\u{B250}\u{B250}\n")]
#[case::wide_characters_with_characters_short_option(&["-c", "-w", "5"], "\u{B250}\u{B250}\u{B250}\n", "\u{B250}\u{B250}\u{B250}\n")]
#[case::should_preserve_empty_line_without_final_newline(&["-w2"], "12\n\n34", "12\n\n34")]
#[case::should_preserve_empty_line_and_final_newline(&["-w2"], "12\n\n34\n", "12\n\n34\n")]
#[case::should_not_add_newline_when_line_longer_than_fold(&["-w2"], "1234", "12\n34")]
#[case::should_not_add_newline_when_line_equal_to_fold(&["-w1"], " ", " ")]
#[case::should_preserve_final_newline_when_line_longer_than_fold(&["-w2"], "1234\n", "12\n34\n")]
#[case::should_preserve_final_newline_when_line_equal_to_fold(&["-w2"], "1\n", "1\n")]
#[case::single_tab_should_not_add_extra_newline(&["-w1"], "\t", "\t")]
#[case::initial_tab_counts_as_8_columns(&["-w8"], "\t1", "\t\n1")]
#[case::fold_before_tab_with_narrow_width(&["-w7"], "a\t1", "a\n\t\n1")]
#[case::fold_at_word_boundary(&["-w4", "-s"], "one two", "one \ntwo")]
#[case::fold_at_leading_word_boundary(&["-w3", "-s"], " aaa", " \naaa")]
#[case::fold_at_word_boundary_preserve_final_newline(&["-w4", "-s"], "one two\n", "one \ntwo\n")]
#[case::fold_at_tab(&["-w8"], "a\tbbb\n", "a\t\nbbb\n")]
#[case::fold_after_tab(&["-w10"], "a\tbbb\n", "a\tbb\nb\n")]
#[case::fold_characters_tab_advances_to_next_tab_stop(&["-c", "-w", "4"], "ab\tcd\n", "ab\n\t\ncd\n")]
#[case::fold_characters_tab_with_non_ascii(&["-c", "-w", "2"], "\u{00E9}\tb\n", "\u{00E9}\n\t\nb\n")]
#[case::fold_at_tab_as_word_boundary(&["-w8", "-s"], "a\tbbb\n", "a\t\nbbb\n")]
#[case::fold_after_tab_as_word_boundary(&["-w10", "-s"], "a\tbbb\n", "a\t\nbbb\n")]
#[case::fold_at_word_boundary_only_whitespace(&["-w2", "-s"], "    ", "  \n  ")]
#[case::fold_at_word_boundary_only_whitespace_preserve_final_newline(&["-w2", "-s"], "    \n", "  \n  \n")]
#[case::backspace_should_decrease_column_count(&["-w2"], "1\x08345", "1\x0834\n5")]
#[case::backspace_should_not_decrease_column_count_past_zero(&["-w2"], "1\x08\x083456", "1\x08\x0834\n56")]
#[case::carriage_return_should_reset_column_count(&["-w6"], "12345\r123456789abcdef", "12345\r123456\n789abc\ndef")]
#[case::bytewise_should_preserve_empty_line_without_final_newline(&["-w2", "-b"], "123\n\n45", "12\n3\n\n45")]
#[case::bytewise_should_preserve_empty_line_and_final_newline(&["-w2", "-b"], "12\n\n34\n", "12\n\n34\n")]
#[case::bytewise_should_not_add_newline_when_line_less_than_fold(&["-b"], "1234", "1234")]
#[case::bytewise_should_not_add_newline_when_line_longer_than_fold(&["-w2", "-b"], "1234", "12\n34")]
#[case::bytewise_should_not_add_newline_when_line_equal_to_fold(&["-w1", "-b"], " ", " ")]
#[case::bytewise_should_preserve_final_newline_when_line_less_than_fold(&["-b"], "1234\n", "1234\n")]
#[case::bytewise_should_preserve_final_newline_when_line_longer_than_fold(&["-w2", "-b"], "1234\n", "12\n34\n")]
#[case::bytewise_should_preserve_final_newline_when_line_equal_to_fold(&["-w2", "-b"], "1\n", "1\n")]
#[case::bytewise_single_tab_should_not_add_extra_newline(&["-w1", "-b"], "\t", "\t")]
#[case::tab_counts_as_one_byte(&["-w2", "-b"], "1\t2\n", "1\t\n2\n")]
#[case::bytewise_fold_before_tab_with_narrow_width(&["-w7", "-b"], "a\t1", "a\t1")]
#[case::bytewise_fold_at_word_boundary_only_whitespace(&["-w2", "-s", "-b"], "    ", "  \n  ")]
#[case::bytewise_fold_at_word_boundary_only_whitespace_preserve_final_newline(&["-w2", "-s", "-b"], "    \n", "  \n  \n")]
#[case::bytewise_fold_line_longer_than_width_still_folds(&["-w7", "-s", "-b"], "aaa bbbb\n", "aaa \nbbbb\n")]
#[case::bytewise_backspace_should_be_preserved(&["-b"], "\x08", "\x08")]
#[case::bytewise_backspaced_char_should_be_preserved(&["-b"], "x\x08", "x\x08")]
#[case::bytewise_backspace_should_not_decrease_column_count(&["-w2", "-b"], "1\x08345", "1\x08\n34\n5")]
#[case::bytewise_carriage_return_should_be_preserved(&["-b"], "\r", "\r")]
#[case::bytewise_carriage_return_overwritten_char_should_be_preserved(&["-b"], "x\ry", "x\ry")]
#[case::bytewise_carriage_return_should_not_reset_column_count(&["-w6", "-b"], "12345\r123456789abcdef", "12345\r\n123456\n789abc\ndef")]
// A bare `-` has no trailing `w`, so it must not be mistaken for one of the
// short spellings that consumes the next argument as its width -- that once
// swallowed whatever followed the `-` operand instead of scanning it.
#[case::dash_operand_does_not_swallow_the_next_argument(&["-", "-5"], "hello\n", "hello\n")]
fn test_fold_stdin(#[case] args: &[&str], #[case] input: &str, #[case] expected: &str) {
    new_ucmd!()
        .args(args)
        .pipe_in(input)
        .succeeds()
        .stdout_only(expected);
}

#[test]
fn test_multiple_wide_characters_in_column_mode() {
    let wide = '\u{FF1A}';
    let mut input = wide.to_string().repeat(50);
    input.push('\n');

    let mut expected = String::new();
    for i in 1..=50 {
        expected.push(wide);
        if i % 5 == 0 {
            expected.push('\n');
        }
    }

    new_ucmd!()
        .args(&["-w", "10"])
        .pipe_in(input)
        .succeeds()
        .stdout_is(expected);
}

#[test]
fn test_multiple_wide_characters_in_character_mode() {
    let wide = '\u{FF1A}';
    let mut input = wide.to_string().repeat(50);
    input.push('\n');

    let mut expected = String::new();
    for i in 1..=50 {
        expected.push(wide);
        if i % 10 == 0 {
            expected.push('\n');
        }
    }

    new_ucmd!()
        .args(&["--characters", "-w", "10"])
        .pipe_in(input)
        .succeeds()
        .stdout_is(expected);
}

#[test]
fn test_unicode_on_reader_buffer_boundary_in_character_mode() {
    let boundary = buf_reader_capacity().saturating_sub(1);
    assert!(boundary > 0, "BufReader capacity must be greater than 1");

    let mut input = "a".repeat(boundary);
    input.push('\u{B250}');
    input.push_str(&"a".repeat(100));
    input.push('\n');

    let expected_tail = tail_inclusive(&fold_characters_reference(&input, 80), 4);

    let result = new_ucmd!().arg("--characters").pipe_in(input).succeeds();

    let actual_tail = tail_inclusive(result.stdout_str(), 4);

    assert_eq!(actual_tail, expected_tail);
}

#[test]
fn test_fold_preserves_invalid_utf8_sequences() {
    let bad_input: &[u8] = b"\xC3|\xED\xBA\xAD|\x00|\x89|\xED\xA6\xBF\xED\xBF\xBF\n";

    new_ucmd!()
        .pipe_in(bad_input.to_vec())
        .succeeds()
        .stdout_is_bytes(bad_input);
}

#[test]
fn test_fold_preserves_incomplete_utf8_at_eof() {
    let trailing_byte: &[u8] = b"\xC3";

    new_ucmd!()
        .pipe_in(trailing_byte.to_vec())
        .succeeds()
        .stdout_is_bytes(trailing_byte);
}

#[test]
fn test_zero_width_bytes_in_column_mode() {
    let len = io_buf_size_times_two();
    let input = vec![0u8; len];

    new_ucmd!()
        .pipe_in(input.clone())
        .succeeds()
        .stdout_is_bytes(input);
}

#[test]
fn test_zero_width_bytes_in_character_mode() {
    let len = io_buf_size_times_two();
    let input = vec![0u8; len];
    let expected = fold_characters_reference_bytes(&input, 80);

    new_ucmd!()
        .args(&["--characters"])
        .pipe_in(input)
        .succeeds()
        .stdout_is_bytes(expected);
}

#[test]
fn test_zero_width_spaces_in_column_mode() {
    let len = io_buf_size_times_two();
    let input = "\u{200B}".repeat(len);

    new_ucmd!()
        .pipe_in(input.clone())
        .succeeds()
        .stdout_is(&input);
}

#[test]
fn test_zero_width_spaces_in_character_mode() {
    let len = io_buf_size_times_two();
    let input = "\u{200B}".repeat(len);
    let expected = fold_characters_reference(&input, 80);

    new_ucmd!()
        .args(&["--characters"])
        .pipe_in(input)
        .succeeds()
        .stdout_is(&expected);
}

#[test]
fn test_zero_width_bytes_from_file() {
    let len = io_buf_size_times_two();
    let input = vec![0u8; len];
    let expected = fold_characters_reference_bytes(&input, 80);

    let ts = TestScenario::new(util_name!());
    let path = "zeros.bin";
    ts.fixtures.write_bytes(path, &input);

    ts.ucmd().arg(path).succeeds().stdout_is_bytes(&input);

    ts.ucmd()
        .args(&["--characters", path])
        .succeeds()
        .stdout_is_bytes(expected);
}

#[test]
fn test_zero_width_spaces_from_file() {
    let len = io_buf_size_times_two();
    let input = "\u{200B}".repeat(len);
    let expected = fold_characters_reference(&input, 80);

    let ts = TestScenario::new(util_name!());
    let path = "zero-width.txt";
    ts.fixtures.write(path, &input);

    ts.ucmd().arg(path).succeeds().stdout_is(&input);

    ts.ucmd()
        .args(&["--characters", path])
        .succeeds()
        .stdout_is(&expected);
}

#[test]
fn test_zero_width_data_line_counts() {
    let len = io_buf_size_times_two();

    let zero_bytes = vec![0u8; len];
    let column_bytes = new_ucmd!().pipe_in(zero_bytes.clone()).succeeds();
    assert_eq!(
        newline_count(column_bytes.stdout()),
        0,
        "fold should not wrap zero-width bytes in column mode",
    );

    let characters_bytes = new_ucmd!()
        .args(&["--characters"])
        .pipe_in(zero_bytes)
        .succeeds();
    assert_eq!(
        newline_count(characters_bytes.stdout()),
        len / 80,
        "fold --characters should wrap zero-width bytes every 80 bytes",
    );

    if UnicodeWidthChar::width('\u{200B}') != Some(0) {
        eprintln!("skip zero width space checks because width != 0");
        return;
    }

    let zero_width_spaces = "\u{200B}".repeat(len);
    let column_spaces = new_ucmd!().pipe_in(zero_width_spaces.clone()).succeeds();
    assert_eq!(
        newline_count(column_spaces.stdout()),
        0,
        "fold should keep zero-width spaces on a single line in column mode",
    );

    let characters_spaces = new_ucmd!()
        .args(&["--characters"])
        .pipe_in(zero_width_spaces)
        .succeeds();
    assert_eq!(
        newline_count(characters_spaces.stdout()),
        len / 80,
        "fold --characters should wrap zero-width spaces every 80 characters",
    );
}

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "netbsd"))]
#[test]
fn test_fold_reports_no_space_left_on_dev_full() {
    use std::fs::OpenOptions;
    use std::process::Stdio;

    for &byte in &[b'\n', b'\0', 0xC3u8] {
        let dev_full = OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .expect("/dev/full must exist on supported targets");

        new_ucmd!()
            .pipe_in(vec![byte; 1024])
            .set_stdout(Stdio::from(dev_full))
            .fails()
            .stderr_contains("No space left");
    }
}

fn buf_reader_capacity() -> usize {
    std::io::BufReader::new(&b""[..]).capacity()
}

fn io_buf_size_times_two() -> usize {
    buf_reader_capacity()
        .checked_mul(2)
        .expect("BufReader capacity overflow")
}

fn fold_characters_reference(input: &str, width: usize) -> String {
    let mut output = String::with_capacity(input.len());
    let mut col_count = 0usize;

    for ch in input.chars() {
        if ch == '\n' {
            output.push('\n');
            col_count = 0;
            continue;
        }

        if col_count >= width {
            output.push('\n');
            col_count = 0;
        }

        output.push(ch);
        col_count += 1;
    }

    output
}

fn fold_characters_reference_bytes(input: &[u8], width: usize) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len() + input.len() / width + 1);

    for chunk in input.chunks(width) {
        output.extend_from_slice(chunk);
        if chunk.len() == width {
            output.push(b'\n');
        }
    }

    output
}

fn newline_count(bytes: &[u8]) -> usize {
    count(bytes, b'\n')
}

fn tail_inclusive(text: &str, lines: usize) -> String {
    if lines == 0 {
        return String::new();
    }

    let segments: Vec<&str> = text.split_inclusive('\n').collect();
    if segments.is_empty() {
        return text.to_owned();
    }

    let start = segments.len().saturating_sub(lines);
    segments[start..].concat()
}

#[test]
fn test_should_preserve_empty_lines() {
    new_ucmd!().pipe_in("\n").succeeds().stdout_is("\n");

    new_ucmd!()
        .arg("-w1")
        .pipe_in("0\n1\n\n2\n\n\n")
        .succeeds()
        .stdout_is("0\n1\n\n2\n\n\n");
}

#[test]
fn test_word_boundary_split_should_preserve_empty_lines() {
    new_ucmd!()
        .arg("-s")
        .pipe_in("\n")
        .succeeds()
        .stdout_is("\n");

    new_ucmd!()
        .args(&["-w1", "-s"])
        .pipe_in("0\n1\n\n2\n\n\n")
        .succeeds()
        .stdout_is("0\n1\n\n2\n\n\n");
}

#[test]
fn test_should_not_add_newline_when_line_less_than_fold() {
    new_ucmd!().pipe_in("1234").succeeds().stdout_is("1234");
}

#[test]
fn test_should_preserve_final_newline_when_line_less_than_fold() {
    new_ucmd!().pipe_in("1234\n").succeeds().stdout_is("1234\n");
}

#[test]
fn test_tab_should_advance_to_next_tab_stop() {
    // tab advances the column count to the next tab stop, i.e. the width
    // of the tab varies based on the leading text
    new_ucmd!()
        .args(&["-w8", "tab_stops.input"])
        .succeeds()
        .stdout_is_fixture("tab_stops_w8.expected");
}

#[test]
fn test_all_tabs_should_advance_to_next_tab_stops() {
    new_ucmd!()
        .args(&["-w16", "tab_stops.input"])
        .succeeds()
        .stdout_is_fixture("tab_stops_w16.expected");
}

#[test]
fn test_backspace_should_be_preserved() {
    new_ucmd!().pipe_in("\x08").succeeds().stdout_is("\x08");
}

#[test]
fn test_backspaced_char_should_be_preserved() {
    new_ucmd!().pipe_in("x\x08").succeeds().stdout_is("x\x08");
}

#[test]
fn test_backspace_is_not_word_boundary() {
    new_ucmd!()
        .args(&["-w10", "-s"])
        .pipe_in("foobar\x086789abcdef")
        .succeeds()
        .stdout_is("foobar\x086789a\nbcdef"); // spell-checker:disable-line
}

#[test]
fn test_carriage_return_should_be_preserved() {
    new_ucmd!().pipe_in("\r").succeeds().stdout_is("\r");
}

#[test]
fn test_carriage_return_overwritten_char_should_be_preserved() {
    new_ucmd!().pipe_in("x\ry").succeeds().stdout_is("x\ry");
}

#[test]
fn test_carriage_return_is_not_word_boundary() {
    new_ucmd!()
        .args(&["-w6", "-s"])
        .pipe_in("fizz\rbuzz\rfizzbuzz") // spell-checker:disable-line
        .succeeds()
        .stdout_is("fizz\rbuzz\rfizzbu\nzz"); // spell-checker:disable-line
}

//
// bytewise tests

#[test]
fn test_bytewise_should_preserve_empty_lines() {
    new_ucmd!()
        .arg("-b")
        .pipe_in("\n")
        .succeeds()
        .stdout_is("\n");

    new_ucmd!()
        .args(&["-w1", "-b"])
        .pipe_in("0\n1\n\n2\n\n\n")
        .succeeds()
        .stdout_is("0\n1\n\n2\n\n\n");
}

#[test]
fn test_bytewise_word_boundary_split_should_preserve_empty_lines() {
    new_ucmd!()
        .args(&["-s", "-b"])
        .pipe_in("\n")
        .succeeds()
        .stdout_is("\n");

    new_ucmd!()
        .args(&["-w1", "-s", "-b"])
        .pipe_in("0\n1\n\n2\n\n\n")
        .succeeds()
        .stdout_is("0\n1\n\n2\n\n\n");
}

#[test]
fn test_bytewise_fold_line_of_exactly_width_is_not_folded() {
    // A line that is exactly `width` bytes long already fits, so -s must not
    // break it at its last blank.
    new_ucmd!()
        .args(&["-w7", "-s", "-b"])
        .pipe_in("aaa bbb\nccc ddd\n")
        .succeeds()
        .stdout_is("aaa bbb\nccc ddd\n");
}

#[test]
fn test_bytewise_fold_remainder_of_exactly_width_is_not_folded() {
    // Same, for what is left of a line after a width-driven fold.
    new_ucmd!()
        .args(&["-w7", "-s", "-b"])
        .pipe_in("aaa bbb ccc\n")
        .succeeds()
        .stdout_is("aaa \nbbb ccc\n");
}

#[test]
fn test_bytewise_backspace_is_not_word_boundary() {
    new_ucmd!()
        .args(&["-w10", "-s", "-b"])
        .pipe_in("foobar\x0889abcdef")
        .succeeds()
        .stdout_is("foobar\x0889a\nbcdef"); // spell-checker:disable-line
}

#[test]
fn test_bytewise_carriage_return_is_not_word_boundary() {
    new_ucmd!()
        .args(&["-w6", "-s", "-b"])
        .pipe_in("fizz\rbuzz\rfizzbuzz") // spell-checker:disable-line
        .succeeds()
        .stdout_is("fizz\rb\nuzz\rfi\nzzbuzz"); // spell-checker:disable-line
}

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "netbsd"))]
#[cfg_attr(
    wasi_runner,
    ignore = "WASI: killing the wasmtime process discards the unflushed output buffer, so the streamed bytes never reach stdout"
)]
#[test]
fn test_bytewise_read_from_pseudo_device() {
    let mut child = new_ucmd!().arg("-b").arg("/dev/zero").run_no_wait();

    child.make_assertion_with_delay(100).is_alive();

    child
        .kill()
        .make_assertion()
        .with_all_output()
        .stdout_contains_bytes(b"\x00\x0a")
        .no_stderr();
}

/// A fold boundary that lands exactly on the read-buffer boundary must still
/// insert the fold newline. The streaming reader fills at most `BufReader`
/// capacity per call, so folding at `width == capacity` is the case where the
/// byte following the fold point is not yet buffered.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_bytewise_fold_at_read_buffer_boundary() {
    let width = buf_reader_capacity();
    let input = vec![b'a'; width * 2];

    let mut expected = vec![b'a'; width];
    expected.push(b'\n');
    expected.extend(std::iter::repeat_n(b'a', width));

    new_ucmd!()
        .args(&["-b", &format!("-w{width}")])
        .pipe_in(input)
        .succeeds()
        .stdout_is_bytes(expected);
}

#[test]
fn test_continue_after_missing_file() {
    // A nonexistent operand must not abort the run: the surrounding files are
    // still folded, the error is reported on stderr, and the exit status is 1.
    let ts = TestScenario::new(util_name!());
    ts.fixtures.write("first.txt", "hello\n");
    ts.fixtures.write("third.txt", "world\n");

    ts.ucmd()
        .args(&["first.txt", "absent.txt", "third.txt"])
        .fails_with_code(1)
        .stdout_is("hello\nworld\n")
        .stderr_is("fold: absent.txt: No such file or directory\n");
}

#[test]
fn test_obsolete_syntax() {
    new_ucmd!()
        .arg("-5")
        .arg("-s")
        .arg("space_separated_words.txt")
        .succeeds()
        .stdout_is("test1\n \ntest2\n \ntest3\n \ntest4\n \ntest5\n \ntest6\n ");
}
#[test]
fn test_byte_break_at_non_utf8_character() {
    new_ucmd!()
        .arg("-b")
        .arg("-s")
        .arg("-w")
        .arg("40")
        .arg("non_utf8.input")
        .succeeds()
        .stdout_is_fixture_bytes("non_utf8.expected");
}
#[test]
fn test_tab_advances_at_non_utf8_character() {
    new_ucmd!()
        .arg("-w8")
        .arg("non_utf8_tab_stops.input")
        .succeeds()
        .stdout_is_fixture_bytes("non_utf8_tab_stops_w8.expected");
}
#[test]
fn test_all_tab_advances_at_non_utf8_character() {
    new_ucmd!()
        .arg("-w16")
        .arg("non_utf8_tab_stops.input")
        .succeeds()
        .stdout_is_fixture_bytes("non_utf8_tab_stops_w16.expected");
}

#[test]
fn test_combining_characters_nfc() {
    // e acute NFC form (single character)
    let e_acute_nfc = "\u{00E9}"; // é as single character
    new_ucmd!()
        .arg("-w2")
        .pipe_in(format!("{e_acute_nfc}{e_acute_nfc}{e_acute_nfc}"))
        .succeeds()
        .stdout_is(format!("{e_acute_nfc}{e_acute_nfc}\n{e_acute_nfc}"));
}

#[test]
fn test_combining_characters_nfd() {
    // e acute NFD form (base + combining acute)
    let e_acute_nfd = "e\u{0301}"; // e + combining acute accent
    new_ucmd!()
        .arg("-w2")
        .pipe_in(format!("{e_acute_nfd}{e_acute_nfd}{e_acute_nfd}"))
        .succeeds()
        .stdout_is(format!("{e_acute_nfd}{e_acute_nfd}\n{e_acute_nfd}"));
}

#[test]
fn test_fullwidth_characters() {
    // e fullwidth (takes 2 columns)
    let e_fullwidth = "\u{FF45}"; // ｅ
    new_ucmd!()
        .arg("-w2")
        .pipe_in(format!("{e_fullwidth}{e_fullwidth}"))
        .succeeds()
        .stdout_is(format!("{e_fullwidth}\n{e_fullwidth}"));
}

#[test]
fn test_character_mode_special_chars() {
    for (args, input, expected) in [
        // backspace decreases column
        (&["-c", "-w", "5"][..], "abcde\x08fg\n", "abcde\x08f\ng\n"),
        // carriage return resets column
        (&["-c", "-w", "5"], "abcd\refgh\n", "abcd\refgh\n"),
        // tab at start exceeds width
        (&["-c", "-w", "4"], "\tabc\n", "\t\nabc\n"),
        // multiple tabs
        (&["-c", "-w", "10"], "a\tb\tc\n", "a\tb\n\tc\n"),
        // basic folding
        (&["-c", "-w", "3"], "abcdef\n", "abc\ndef\n"),
        // preserves empty lines
        (&["-c", "-w", "5"], "abc\n\ndef\n", "abc\n\ndef\n"),
        // word boundary with -s
        (&["-c", "-s", "-w", "5"], "ab cd ef\n", "ab \ncd ef\n"),
        // tab as word boundary
        (&["-c", "-s", "-w", "10"], "abcd\tefgh\n", "abcd\t\nefgh\n"),
        // wide chars count as 1 in -c mode
        (
            &["-c", "-w", "3"],
            "\u{FF1A}\u{FF1A}\u{FF1A}\u{FF1A}\n",
            "\u{FF1A}\u{FF1A}\u{FF1A}\n\u{FF1A}\n",
        ),
    ] {
        new_ucmd!()
            .args(args)
            .pipe_in(input)
            .succeeds()
            .stdout_is(expected);
    }
}

#[test]
fn test_width_zero() {
    new_ucmd!()
        .arg("-w")
        .arg("0")
        .fails_with_code(1)
        .stderr_is("fold: invalid number of columns: '0': Numerical result out of range\n");
}

#[test]
fn test_width_invalid() {
    for width in ["xyz", "12x", "12.5", "1 2"] {
        new_ucmd!()
            .arg("-w")
            .arg(width)
            .fails_with_code(1)
            .stderr_is(format!("fold: invalid number of columns: '{width}'\n"));
    }
}

#[test]
fn test_width_overflow() {
    new_ucmd!()
        .arg("-w")
        .arg("999999999999999999999")
        .fails_with_code(1)
        .stderr_is(
            "fold: invalid number of columns: '999999999999999999999': Numerical result out of range\n",
        );
}

#[test]
fn test_negative_width_as_separate_value() {
    // GNU consumes the argument after `-w` as its value even when it looks
    // like an option, and reports that value as the invalid one. Passing a
    // file too, because the width used to be mistaken for the obsolete
    // `-WIDTH` form, leaving the file name to be read as the width.
    for args in [
        vec!["-w", "-1"],
        vec!["--width", "-1"],
        vec!["-bw", "-1"],
        vec!["-sw", "-1"],
    ] {
        new_ucmd!()
            .args(&args)
            .arg("lorem_ipsum.txt")
            .fails_with_code(1)
            .stderr_is("fold: invalid number of columns: '-1'\n");
    }
}

#[test]
fn test_obsolete_width_after_double_dash_is_a_file() {
    // After `--` there are no more options, so `-3` names a file.
    new_ucmd!()
        .arg("--")
        .arg("-3")
        .fails_with_code(1)
        .stderr_is("fold: -3: No such file or directory\n");
}

#[test]
fn test_last_width_wins() {
    // GNU processes options left to right, so the last width given wins,
    // regardless of which spelling each one used.
    for args in [
        vec!["-w3", "-5"],
        vec!["-3", "-5"],
        vec!["-w", "3", "-w", "5"],
        vec!["-w3", "-w5"],
        vec!["-5", "-w", "5"],
    ] {
        new_ucmd!()
            .args(&args)
            .pipe_in("aaaaaaaaaa\n")
            .succeeds()
            .stdout_is("aaaaa\naaaaa\n");
    }

    // ...and an obsolete width is overridden by a later `-w` just the same.
    new_ucmd!()
        .args(&["-5", "-w", "3"])
        .pipe_in("aaaaaa\n")
        .succeeds()
        .stdout_is("aaa\naaa\n");
}

#[test]
fn test_dash_operand_reads_stdin() {
    // A bare `-` names stdin and leaves nothing after the dash for the
    // width-value scan to index, which used to abort before reading input.
    new_ucmd!()
        .arg("-")
        .pipe_in("hello\n")
        .succeeds()
        .stdout_is("hello\n");

    // ...in every position, including as the value of -w, where GNU rejects
    // it as a width rather than treating it as the operand.
    new_ucmd!()
        .args(&["-w", "3", "-"])
        .pipe_in("hello\n")
        .succeeds()
        .stdout_is("hel\nlo\n");
    new_ucmd!()
        .args(&["-w", "-"])
        .pipe_in("hello\n")
        // fold rejects the width before it reads stdin, so the write can
        // race with its exit and hit a broken pipe.
        .ignore_stdin_write_error()
        .fails()
        .stderr_contains("invalid number of columns: '-'");
}
