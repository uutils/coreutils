// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore fullwidth

use bytecount::count;
use unicode_width::UnicodeWidthChar;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
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

#[test]
fn test_wide_characters_in_column_mode() {
    new_ucmd!()
        .args(&["-w", "5"])
        .pipe_in("\u{B250}\u{B250}\u{B250}\n")
        .succeeds()
        .stdout_is("\u{B250}\u{B250}\n\u{B250}\n");
}

#[test]
fn test_wide_characters_with_characters_option() {
    new_ucmd!()
        .args(&["--characters", "-w", "5"])
        .pipe_in("\u{B250}\u{B250}\u{B250}\n")
        .succeeds()
        .stdout_is("\u{B250}\u{B250}\u{B250}\n");
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
fn test_should_preserve_empty_line_without_final_newline() {
    new_ucmd!()
        .arg("-w2")
        .pipe_in("12\n\n34")
        .succeeds()
        .stdout_is("12\n\n34");
}

#[test]
fn test_should_preserve_empty_line_and_final_newline() {
    new_ucmd!()
        .arg("-w2")
        .pipe_in("12\n\n34\n")
        .succeeds()
        .stdout_is("12\n\n34\n");
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
fn test_should_not_add_newline_when_line_longer_than_fold() {
    new_ucmd!()
        .arg("-w2")
        .pipe_in("1234")
        .succeeds()
        .stdout_is("12\n34");
}

#[test]
fn test_should_not_add_newline_when_line_equal_to_fold() {
    new_ucmd!()
        .arg("-w1")
        .pipe_in(" ")
        .succeeds()
        .stdout_is(" ");
}

#[test]
fn test_should_preserve_final_newline_when_line_less_than_fold() {
    new_ucmd!().pipe_in("1234\n").succeeds().stdout_is("1234\n");
}

#[test]
fn test_should_preserve_final_newline_when_line_longer_than_fold() {
    new_ucmd!()
        .arg("-w2")
        .pipe_in("1234\n")
        .succeeds()
        .stdout_is("12\n34\n");
}

#[test]
fn test_should_preserve_final_newline_when_line_equal_to_fold() {
    new_ucmd!()
        .arg("-w2")
        .pipe_in("1\n")
        .succeeds()
        .stdout_is("1\n");
}

#[test]
fn test_single_tab_should_not_add_extra_newline() {
    new_ucmd!()
        .arg("-w1")
        .pipe_in("\t")
        .succeeds()
        .stdout_is("\t");
}

#[test]
fn test_initial_tab_counts_as_8_columns() {
    new_ucmd!()
        .arg("-w8")
        .pipe_in("\t1")
        .succeeds()
        .stdout_is("\t\n1");
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
fn test_fold_before_tab_with_narrow_width() {
    new_ucmd!()
        .arg("-w7")
        .pipe_in("a\t1")
        .succeeds()
        .stdout_is("a\n\t\n1");
}

#[test]
fn test_fold_at_word_boundary() {
    new_ucmd!()
        .args(&["-w4", "-s"])
        .pipe_in("one two")
        .succeeds()
        .stdout_is("one \ntwo");
}

#[test]
fn test_fold_at_leading_word_boundary() {
    new_ucmd!()
        .args(&["-w3", "-s"])
        .pipe_in(" aaa")
        .succeeds()
        .stdout_is(" \naaa");
}

#[test]
fn test_fold_at_word_boundary_preserve_final_newline() {
    new_ucmd!()
        .args(&["-w4", "-s"])
        .pipe_in("one two\n")
        .succeeds()
        .stdout_is("one \ntwo\n");
}

#[test]
fn test_fold_at_tab() {
    new_ucmd!()
        .arg("-w8")
        .pipe_in("a\tbbb\n")
        .succeeds()
        .stdout_is("a\t\nbbb\n");
}

#[test]
fn test_fold_after_tab() {
    new_ucmd!()
        .arg("-w10")
        .pipe_in("a\tbbb\n")
        .succeeds()
        .stdout_is("a\tbb\nb\n");
}

#[test]
fn test_fold_at_tab_as_word_boundary() {
    new_ucmd!()
        .args(&["-w8", "-s"])
        .pipe_in("a\tbbb\n")
        .succeeds()
        .stdout_is("a\t\nbbb\n");
}

#[test]
fn test_fold_after_tab_as_word_boundary() {
    new_ucmd!()
        .args(&["-w10", "-s"])
        .pipe_in("a\tbbb\n")
        .succeeds()
        .stdout_is("a\t\nbbb\n");
}

#[test]
fn test_fold_at_word_boundary_only_whitespace() {
    new_ucmd!()
        .args(&["-w2", "-s"])
        .pipe_in("    ")
        .succeeds()
        .stdout_is("  \n  ");
}

#[test]
fn test_fold_at_word_boundary_only_whitespace_preserve_final_newline() {
    new_ucmd!()
        .args(&["-w2", "-s"])
        .pipe_in("    \n")
        .succeeds()
        .stdout_is("  \n  \n");
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
fn test_backspace_should_decrease_column_count() {
    new_ucmd!()
        .arg("-w2")
        .pipe_in("1\x08345")
        .succeeds()
        .stdout_is("1\x0834\n5");
}

#[test]
fn test_backspace_should_not_decrease_column_count_past_zero() {
    new_ucmd!()
        .arg("-w2")
        .pipe_in("1\x08\x083456")
        .succeeds()
        .stdout_is("1\x08\x0834\n56");
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
fn test_carriage_return_should_reset_column_count() {
    new_ucmd!()
        .arg("-w6")
        .pipe_in("12345\r123456789abcdef")
        .succeeds()
        .stdout_is("12345\r123456\n789abc\ndef");
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
fn test_bytewise_should_preserve_empty_line_without_final_newline() {
    new_ucmd!()
        .args(&["-w2", "-b"])
        .pipe_in("123\n\n45")
        .succeeds()
        .stdout_is("12\n3\n\n45");
}

#[test]
fn test_bytewise_should_preserve_empty_line_and_final_newline() {
    new_ucmd!()
        .args(&["-w2", "-b"])
        .pipe_in("12\n\n34\n")
        .succeeds()
        .stdout_is("12\n\n34\n");
}

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
fn test_bytewise_should_not_add_newline_when_line_less_than_fold() {
    new_ucmd!()
        .arg("-b")
        .pipe_in("1234")
        .succeeds()
        .stdout_is("1234");
}

#[test]
fn test_bytewise_should_not_add_newline_when_line_longer_than_fold() {
    new_ucmd!()
        .args(&["-w2", "-b"])
        .pipe_in("1234")
        .succeeds()
        .stdout_is("12\n34");
}

#[test]
fn test_bytewise_should_not_add_newline_when_line_equal_to_fold() {
    new_ucmd!()
        .args(&["-w1", "-b"])
        .pipe_in(" ")
        .succeeds()
        .stdout_is(" ");
}

#[test]
fn test_bytewise_should_preserve_final_newline_when_line_less_than_fold() {
    new_ucmd!()
        .arg("-b")
        .pipe_in("1234\n")
        .succeeds()
        .stdout_is("1234\n");
}

#[test]
fn test_bytewise_should_preserve_final_newline_when_line_longer_than_fold() {
    new_ucmd!()
        .args(&["-w2", "-b"])
        .pipe_in("1234\n")
        .succeeds()
        .stdout_is("12\n34\n");
}

#[test]
fn test_bytewise_should_preserve_final_newline_when_line_equal_to_fold() {
    new_ucmd!()
        .args(&["-w2", "-b"])
        .pipe_in("1\n")
        .succeeds()
        .stdout_is("1\n");
}

#[test]
fn test_bytewise_single_tab_should_not_add_extra_newline() {
    new_ucmd!()
        .args(&["-w1", "-b"])
        .pipe_in("\t")
        .succeeds()
        .stdout_is("\t");
}

#[test]
fn test_tab_counts_as_one_byte() {
    new_ucmd!()
        .args(&["-w2", "-b"])
        .pipe_in("1\t2\n")
        .succeeds()
        .stdout_is("1\t\n2\n");
}

#[test]
fn test_bytewise_fold_before_tab_with_narrow_width() {
    new_ucmd!()
        .args(&["-w7", "-b"])
        .pipe_in("a\t1")
        .succeeds()
        .stdout_is("a\t1");
}

#[test]
fn test_bytewise_fold_at_word_boundary_only_whitespace() {
    new_ucmd!()
        .args(&["-w2", "-s", "-b"])
        .pipe_in("    ")
        .succeeds()
        .stdout_is("  \n  ");
}

#[test]
fn test_bytewise_fold_at_word_boundary_only_whitespace_preserve_final_newline() {
    new_ucmd!()
        .args(&["-w2", "-s", "-b"])
        .pipe_in("    \n")
        .succeeds()
        .stdout_is("  \n  \n");
}

#[test]
fn test_bytewise_backspace_should_be_preserved() {
    new_ucmd!()
        .arg("-b")
        .pipe_in("\x08")
        .succeeds()
        .stdout_is("\x08");
}

#[test]
fn test_bytewise_backspaced_char_should_be_preserved() {
    new_ucmd!()
        .arg("-b")
        .pipe_in("x\x08")
        .succeeds()
        .stdout_is("x\x08");
}

#[test]
fn test_bytewise_backspace_should_not_decrease_column_count() {
    new_ucmd!()
        .args(&["-w2", "-b"])
        .pipe_in("1\x08345")
        .succeeds()
        .stdout_is("1\x08\n34\n5");
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
fn test_bytewise_carriage_return_should_be_preserved() {
    new_ucmd!()
        .arg("-b")
        .pipe_in("\r")
        .succeeds()
        .stdout_is("\r");
}

#[test]
fn test_bytewise_carriage_return_overwritten_char_should_be_preserved() {
    new_ucmd!()
        .arg("-b")
        .pipe_in("x\ry")
        .succeeds()
        .stdout_is("x\ry");
}

#[test]
fn test_bytewise_carriage_return_should_not_reset_column_count() {
    new_ucmd!()
        .args(&["-w6", "-b"])
        .pipe_in("12345\r123456789abcdef")
        .succeeds()
        .stdout_is("12345\r\n123456\n789abc\ndef");
}

#[test]
fn test_bytewise_carriage_return_is_not_word_boundary() {
    new_ucmd!()
        .args(&["-w6", "-s", "-b"])
        .pipe_in("fizz\rbuzz\rfizzbuzz") // spell-checker:disable-line
        .succeeds()
        .stdout_is("fizz\rb\nuzz\rfi\nzzbuzz"); // spell-checker:disable-line
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
