// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_default_80_column_wrap() {
    new_ucmd!()
        .arg("lorem_ipsum.txt")
        .run()
        .stdout_is_fixture("lorem_ipsum_80_column.expected");
}

#[test]
fn test_40_column_hard_cutoff() {
    new_ucmd!()
        .args(&["-w", "40", "lorem_ipsum.txt"])
        .run()
        .stdout_is_fixture("lorem_ipsum_40_column_hard.expected");
}

#[test]
fn test_40_column_word_boundary() {
    new_ucmd!()
        .args(&["-s", "-w", "40", "lorem_ipsum.txt"])
        .run()
        .stdout_is_fixture("lorem_ipsum_40_column_word.expected");
}

#[test]
fn test_default_wrap_with_newlines() {
    new_ucmd!()
        .arg("lorem_ipsum_new_line.txt")
        .run()
        .stdout_is_fixture("lorem_ipsum_new_line_80_column.expected");
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
