// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore roff
// spell-checker:ignore funnnnnnnnnnnnnnnnn
use uutests::new_ucmd;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}
#[test]
fn test_reference_format_for_stdin() {
    let input = "Rust is good language";
    let expected_output = concat!(
        r#".xx "" "" "Rust is good language" "" ":1""#,
        "\n",
        r#".xx "" "Rust is" "good language" "" ":1""#,
        "\n",
        r#".xx "" "Rust" "is good language" "" ":1""#,
        "\n",
        r#".xx "" "Rust is good" "language" "" ":1""#,
        "\n",
    );
    new_ucmd!()
        .args(&["-G", "-A"])
        .pipe_in(input)
        .succeeds()
        .stdout_only(expected_output);
}
#[test]
fn test_tex_format_no_truncation_markers() {
    let input = "Hello world Rust is a fun language";
    new_ucmd!()
        .args(&["-G", "-w", "30", "--format=tex"])
        .pipe_in(input)
        .succeeds()
        .stdout_only_fixture("test_tex_format_no_truncation_markers.expected");
}
#[test]
fn gnu_ext_disabled_chunk_no_over_reading() {
    let input = "Hello World Rust is a fun language";
    new_ucmd!()
        .args(&["-G", "-w", "30"])
        .pipe_in(input)
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_chunk_no_over_reading.expected");
}

#[test]
fn test_truncation_no_extra_space_in_after() {
    new_ucmd!()
        .args(&["-G", "-w", "30"])
        .pipe_in("Rust is funnnnnnnnnnnnnnnnn")
        .succeeds()
        .stdout_contains(".xx \"\" \"Rust\" \"is/\" \"\"");
}

#[test]
fn gnu_ext_disabled_reference_calculation() {
    let input = "Hello World Rust is good language";
    let expected_output = concat!(
        r#".xx "language" "" "Hello World Rust is good" "" ":1""#,
        "\n",
        r#".xx "" "Hello World" "Rust is good language" "" ":1""#,
        "\n",
        r#".xx "" "Hello" "World Rust is good language" "" ":1""#,
        "\n",
        r#".xx "" "Hello World Rust is" "good language" "" ":1""#,
        "\n",
        r#".xx "" "Hello World Rust" "is good language" "" ":1""#,
        "\n",
        r#".xx "" "Hello World Rust is good" "language" "" ":1""#,
        "\n",
    );
    new_ucmd!()
        .args(&["-G", "-A"])
        .pipe_in(input)
        .succeeds()
        .stdout_only(expected_output);
}

#[test]
fn gnu_ext_disabled_rightward_no_ref() {
    new_ucmd!()
        .args(&["-G", "-R", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_rightward_no_ref.expected");
}

#[test]
fn gnu_ext_disabled_rightward_no_ref_empty_word_regexp() {
    new_ucmd!()
        .args(&["-G", "-R", "-W", "", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_rightward_no_ref.expected");
}

#[test]
fn gnu_ext_disabled_rightward_no_ref_word_regexp_exc_space() {
    new_ucmd!()
        .args(&["-G", "-R", "-W", "[^\t\n]+", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_rightward_no_ref_word_regexp_exc_space.expected");
}

#[test]
fn gnu_ext_disabled_rightward_input_ref() {
    new_ucmd!()
        .args(&["-G", "-r", "-R", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_rightward_input_ref.expected");
}

#[test]
fn gnu_ext_disabled_rightward_auto_ref() {
    new_ucmd!()
        .args(&["-G", "-A", "-R", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_rightward_auto_ref.expected");
}

#[test]
fn gnu_ext_disabled_tex_no_ref() {
    new_ucmd!()
        .args(&["-G", "-T", "-R", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_tex_no_ref.expected");
}

#[test]
fn gnu_ext_disabled_tex_input_ref() {
    new_ucmd!()
        .args(&["-G", "-T", "-r", "-R", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_tex_input_ref.expected");
}

#[test]
fn gnu_ext_disabled_tex_auto_ref() {
    new_ucmd!()
        .args(&["-G", "-T", "-A", "-R", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_tex_auto_ref.expected");
}

#[test]
fn gnu_ext_disabled_ignore_and_only_file() {
    new_ucmd!()
        .args(&["-G", "-o", "only", "-i", "ignore", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_ignore_and_only_file.expected");
}

#[test]
fn gnu_ext_disabled_output_width_50() {
    new_ucmd!()
        .args(&["-G", "-w", "50", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_output_width_50.expected");
}

#[test]
fn gnu_ext_disabled_output_width_70() {
    new_ucmd!()
        .args(&["-G", "-w", "70", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_output_width_70.expected");
}

#[test]
fn gnu_ext_disabled_break_file() {
    new_ucmd!()
        .args(&["-G", "-b", "break_file", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_break_file.expected");
}

#[test]
fn gnu_ext_disabled_empty_word_regexp_ignores_break_file() {
    new_ucmd!()
        .args(&["-G", "-b", "break_file", "-R", "-W", "", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_rightward_no_ref.expected");
}

#[test]
fn test_reject_too_many_operands() {
    new_ucmd!().args(&["-G", "-", "-", "-"]).fails_with_code(1);
}

#[test]
fn test_break_file_regex_escaping() {
    new_ucmd!()
        .pipe_in("\\.+*?()|[]{}^$#&-~")
        .args(&["-G", "-b", "-", "input"])
        .succeeds()
        .stdout_only_fixture("break_file_regex_escaping.expected");
}

#[test]
fn test_ignore_case() {
    new_ucmd!()
        .args(&["-G", "-f"])
        .pipe_in("a _")
        .succeeds()
        .stdout_only(".xx \"\" \"\" \"a _\" \"\"\n.xx \"\" \"a\" \"_\" \"\"\n");
}

#[test]
fn test_format() {
    new_ucmd!()
        .args(&["-G", "-O"])
        .pipe_in("a")
        .succeeds()
        .stdout_only(".xx \"\" \"\" \"a\" \"\"\n");
    new_ucmd!()
        .args(&["-G", "-T"])
        .pipe_in("a")
        .succeeds()
        .stdout_only("\\xx {}{}{a}{}{}\n");
    new_ucmd!()
        .args(&["-G", "--format=roff"])
        .pipe_in("a")
        .succeeds()
        .stdout_only(".xx \"\" \"\" \"a\" \"\"\n");
    new_ucmd!()
        .args(&["-G", "--format=tex"])
        .pipe_in("a")
        .succeeds()
        .stdout_only("\\xx {}{}{a}{}{}\n");
}

#[cfg(target_os = "linux")]
#[test]
fn test_failed_write_is_reported() {
    new_ucmd!()
        .arg("-G")
        .pipe_in("hello")
        .set_stdout(std::fs::File::create("/dev/full").unwrap())
        .fails()
        .stderr_is("ptx: write failed: No space left on device\n");
}

#[test]
fn test_utf8() {
    new_ucmd!()
        .args(&["-G"])
        .pipe_in("it’s disabled\n")
        .succeeds()
        .stdout_only(".xx \"\" \"it’s\" \"disabled\" \"\"\n.xx \"\" \"\" \"it’s disabled\" \"\"\n");
    new_ucmd!()
        .args(&["-G", "-T"])
        .pipe_in("it’s disabled\n")
        .succeeds()
        .stdout_only("\\xx {}{it’s}{disabled}{}{}\n\\xx {}{}{it’s}{ disabled}{}\n");
}

#[test]
fn test_sentence_regexp_basic() {
    new_ucmd!()
        .args(&["-G", "-S", "\\."])
        .pipe_in("Hello. World.")
        .succeeds()
        .stdout_contains("Hello")
        .stdout_contains("World");
}

#[test]
fn test_sentence_regexp_split_behavior() {
    new_ucmd!()
        .args(&["-G", "-w", "50", "-S", "[.!]"])
        .pipe_in("One sentence. Two sentence!")
        .succeeds()
        .stdout_contains("One sentence")
        .stdout_contains("Two sentence");
}

#[test]
fn test_sentence_regexp_empty_match_failure() {
    new_ucmd!()
        .args(&["-G", "-S", "^"])
        .fails()
        .stderr_contains("A regular expression cannot match a length zero string");
}

#[test]
fn test_sentence_regexp_newlines_are_spaces() {
    new_ucmd!()
        .args(&["-G", "-S", "\\."])
        .pipe_in("Start of\nsentence.")
        .succeeds()
        .stdout_contains("Start of sentence");
}

#[test]
fn test_gnu_mode_dumb_format() {
    // Test GNU mode (dumb format) - the default mode without -G flag
    new_ucmd!().pipe_in("a b").succeeds().stdout_only(
        "                                       a b\n                                   a   b\n",
    );
}

#[test]
fn test_gnu_compatibility_narrow_width() {
    new_ucmd!()
        .args(&["-w", "2"])
        .pipe_in("qux")
        .succeeds()
        .stdout_only("      qux\n");
}

#[test]
fn test_gnu_compatibility_truncation_width() {
    new_ucmd!()
        .args(&["-w", "10"])
        .pipe_in("foo bar")
        .succeeds()
        .stdout_only("     /   bar\n        foo/\n");
}

#[test]
fn test_unicode_padding_alignment() {
    let input = "a\né";
    new_ucmd!()
        .args(&["-w", "10"])
        .pipe_in(input)
        .succeeds()
        .stdout_only("        a\n        é\n");
}

#[test]
fn test_unicode_truncation_alignment() {
    new_ucmd!()
        .args(&["-w", "10"])
        .pipe_in("föö bar")
        .succeeds()
        .stdout_only("     /   bar\n        föö/\n");
}
