// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore defg naïve nave närd nøys ntøys nfjärd undelimited xbfw

use uutests::{at_and_ucmd, new_ucmd};

static INPUT: &str = "lists.txt";

struct TestedSequence<'b> {
    name: &'b str,
    sequence: &'b str,
}

static EXAMPLE_SEQUENCES: &[TestedSequence] = &[
    TestedSequence {
        name: "singular",
        sequence: "2",
    },
    TestedSequence {
        name: "prefix",
        sequence: "-2",
    },
    TestedSequence {
        name: "suffix",
        sequence: "2-",
    },
    TestedSequence {
        name: "range",
        sequence: "2-4",
    },
    TestedSequence {
        name: "aggregate",
        sequence: "9-,6-7,-2,4",
    },
    TestedSequence {
        name: "subsumed",
        sequence: "2-,3",
    },
];

const COMPLEX_SEQUENCE: &str = "9-,6-7,-2,4";

#[test]
fn test_no_args() {
    new_ucmd!()
        .fails()
        .stderr_contains("cut: you must specify a list of bytes, characters, or fields");
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_range_error_messages() {
    // Mode-aware diagnostics for invalid ranges.
    let cases: &[(&[&str], &str)] = &[
        (
            &["-c0"],
            "cut: byte/character positions are numbered from 1",
        ),
        (
            &["-b0-7"],
            "cut: byte/character positions are numbered from 1",
        ),
        (&["-f0-9"], "cut: fields are numbered from 1"),
        (&["-f", ""], "cut: fields are numbered from 1"),
        (
            &["-c", ""],
            "cut: byte/character positions are numbered from 1",
        ),
        (&["-f", "q"], "cut: invalid field value 'q'"),
        (&["-c", "zz"], "cut: invalid byte/character position 'zz'"),
        (&["-f", "9-4"], "cut: invalid decreasing range"),
        (&["-c", "-"], "cut: invalid range with no endpoint: -"),
        (&["-f", "8,-"], "cut: invalid range with no endpoint: -"),
        // The offending text is what could not be consumed, not the whole item.
        (&["-f", "7k"], "cut: invalid field value 'k'"),
        (&["-f", "4-w2"], "cut: invalid field value 'w2'"),
        (
            &["-c", "3x-5"],
            "cut: invalid byte/character position 'x-5'",
        ),
        // A leading sign is not part of a number here.
        (&["-f", "+6"], "cut: invalid field value '+6'"),
        // A second dash makes it a malformed range instead.
        (&["-f", "2-5-8"], "cut: invalid field range"),
        (&["-c", "3--6"], "cut: invalid byte or character range"),
        // `usize::MAX` itself is rejected, and so is anything above it.
        (
            &["-f", "18446744073709551615"],
            "cut: field number '18446744073709551615' is too large",
        ),
        (
            &["-c", "4-77777777777777777777777"],
            "cut: byte/character offset '77777777777777777777777' is too large",
        ),
    ];
    for (args, expected) in cases {
        new_ucmd!()
            .args(args)
            .fails_with_code(1)
            .stderr_contains(*expected);
    }
}

#[test]
fn test_field_only_options_without_fields() {
    new_ucmd!()
        .args(&["-s", "-c7"])
        .fails_with_code(1)
        .stderr_contains(
            "cut: suppressing non-delimited lines makes sense\n\tonly when operating on fields",
        );
}

#[test]
fn test_delimiter_and_whitespace_are_exclusive() {
    new_ucmd!()
        .args(&["-w", "-d,", "-f3"])
        .fails_with_code(1)
        .stderr_contains("cut: -d and -w are mutually exclusive");
}

#[test]
fn test_byte_sequence() {
    for param in ["-b", "--bytes", "--byt"] {
        for example_seq in EXAMPLE_SEQUENCES {
            new_ucmd!()
                .args(&[param, example_seq.sequence, INPUT])
                .succeeds()
                .stdout_only_fixture(format!("sequences/byte_{}.expected", example_seq.name));
        }
    }
}

#[test]
fn test_char_sequence() {
    for param in ["-c", "--characters", "--char"] {
        for example_seq in EXAMPLE_SEQUENCES {
            //as of coreutils 8.25 a char range is effectively the same as a byte range; there is no distinct treatment of utf8 chars.
            new_ucmd!()
                .args(&[param, example_seq.sequence, INPUT])
                .succeeds()
                .stdout_only_fixture(format!("sequences/byte_{}.expected", example_seq.name));
        }
    }
}

#[test]
fn test_field_sequence() {
    for param in ["-f", "--fields", "--fie"] {
        for example_seq in EXAMPLE_SEQUENCES {
            new_ucmd!()
                .args(&[param, example_seq.sequence, INPUT])
                .succeeds()
                .stdout_only_fixture(format!("sequences/field_{}.expected", example_seq.name));
        }
    }
}

#[test]
fn test_whitespace_delimited() {
    new_ucmd!()
        .args(&["-w", "-f", COMPLEX_SEQUENCE, INPUT])
        .succeeds()
        .stdout_only_fixture("whitespace_delimited.expected");
}

#[test]
fn test_whitespace_with_explicit_delimiter() {
    new_ucmd!()
        .args(&["-w", "-f", COMPLEX_SEQUENCE, "-d:"])
        .fails_with_code(1);
}

#[test]
fn test_whitespace_with_byte() {
    // `-w` counts as an input delimiter for the purpose of this diagnostic.
    new_ucmd!()
        .args(&["-w", "-b", COMPLEX_SEQUENCE])
        .fails_with_code(1)
        .stderr_contains("cut: an input delimiter makes sense\n\tonly when operating on fields");
}

#[test]
fn test_whitespace_with_char() {
    new_ucmd!()
        .args(&["-c", COMPLEX_SEQUENCE, "-w"])
        .fails_with_code(1)
        .stderr_contains("cut: an input delimiter makes sense\n\tonly when operating on fields");
}

#[test]
fn test_delimiter_with_byte_and_char() {
    for conflicting_arg in ["-c", "-b"] {
        new_ucmd!()
            .args(&[conflicting_arg, COMPLEX_SEQUENCE, "-d="])
            .fails_with_code(1)
            .stderr_contains(
                "cut: an input delimiter makes sense\n\tonly when operating on fields",
            );
    }
}

#[test]
#[cfg_attr(wasi_runner, ignore = "WASI sandbox: host paths not visible")]
fn test_too_large() {
    new_ucmd!()
        .args(&["-b1-18446744073709551615", "/dev/null"])
        .fails_with_code(1);
}

#[test]
fn test_delimiter() {
    for param in ["-d", "--delimiter", "--del"] {
        new_ucmd!()
            .args(&[param, ":", "-f", COMPLEX_SEQUENCE, INPUT])
            .succeeds()
            .stdout_only_fixture("delimiter_specified.expected");
    }
}

#[test]
fn test_delimiter_with_more_than_one_char() {
    new_ucmd!()
        .args(&["-d", "ab", "-f1"])
        .fails()
        .stderr_contains("cut: the delimiter must be a single character")
        .no_stdout();
}

#[test]
fn test_output_delimiter() {
    for param in ["--output-delimiter=@", "--output-del=@", "-O@"] {
        // with default field delimiter (tab)
        new_ucmd!()
            .arg(param)
            .arg("-f1,2")
            .pipe_in("a:\tb:\tc:\n")
            .succeeds()
            .stdout_only("a:@b:\n");

        // with custom field delimiter
        new_ucmd!()
            .arg(param)
            .arg("-f1,2")
            .arg("-d:")
            .pipe_in("a:\tb:\tc\n")
            .succeeds()
            .stdout_only("a@\tb\n");

        // with no field delimiter
        new_ucmd!()
            .arg(param)
            .arg("-f1,2")
            .pipe_in("a:b:c\n")
            .succeeds()
            .stdout_only("a:b:c\n");
    }
}

#[test]
fn test_complement() {
    for param in ["--complement", "--com"] {
        new_ucmd!()
            .args(&["-d_", param, "-f", "2"])
            .pipe_in("9_1\n8_2\n7_3")
            .succeeds()
            .stdout_only("9\n8\n7\n");
    }
}

#[test]
fn test_zero_terminated() {
    new_ucmd!()
        .args(&["-d_", "-z", "-f", "1"])
        .pipe_in("9_1\n8_2\n\x007_3")
        .succeeds()
        .stdout_only("9\x007\0");
}

#[test]
fn test_only_delimited() {
    for param in ["-s", "--only-delimited", "--only-del"] {
        new_ucmd!()
            .args(&["-d_", param, "-f", "1"])
            .pipe_in("91\n82\n7_3")
            .succeeds()
            .stdout_only("7\n");
    }
}

#[test]
fn test_zero_terminated_only_delimited() {
    new_ucmd!()
        .args(&["-d_", "-z", "-s", "-f", "1"])
        .pipe_in("91\n\082\n7_3")
        .succeeds()
        .stdout_only("82\n7\0");
}

#[test]
fn test_suppresses_unterminated_segment() {
    new_ucmd!()
        .args(&["-z", "-d", "", "-s", "-f", "1"])
        .pipe_in("unterminated")
        .succeeds()
        .stdout_only_bytes("");

    new_ucmd!()
        .args(&["-z", "-d", "", "-s", "-f", "1"])
        .pipe_in("terminated\0unterminated")
        .succeeds()
        .stdout_only_bytes("terminated\0");
}

#[test]
fn test_is_a_directory() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("some");

    ucmd.arg("-b1")
        .arg("some")
        .fails_with_code(1)
        .stderr_is("cut: some: Is a directory\n");
}

#[test]
fn test_no_such_file() {
    new_ucmd!()
        .arg("-b1")
        .arg("some")
        .fails_with_code(1)
        .stderr_is("cut: some: No such file or directory\n");
}

#[test]
fn test_equal_as_delimiter() {
    for arg in ["-d=", "--delimiter=="] {
        new_ucmd!()
            .args(&["-f2", arg])
            .pipe_in("--dir=./out/lib")
            .succeeds()
            .stdout_only("./out/lib\n");
    }
}

#[test]
fn test_empty_string_as_delimiter() {
    new_ucmd!()
        .args(&["-f2", "--delimiter="])
        .pipe_in("a\0b\n")
        .succeeds()
        .stdout_only("b\n");
}

#[test]
fn test_single_quote_pair_as_delimiter_is_invalid() {
    for args in [&["-d", "''", "-f2"][..], &["--delimiter=''", "-f2"][..]] {
        new_ucmd!()
            .args(args)
            .ignore_stdin_write_error()
            .pipe_in("a''b\n")
            .fails()
            .stderr_contains("cut: the delimiter must be a single character")
            .no_stdout();
    }
}

#[test]
fn test_empty_string_as_delimiter_with_output_delimiter() {
    new_ucmd!()
        .args(&["-f", "1,2", "--delimiter=", "--output-delimiter=Z"])
        .pipe_in("ab\0cd\n")
        .succeeds()
        .stdout_only_bytes("abZcd\n");
}

#[test]
fn test_single_quote_pair_as_output_delimiter_is_literal() {
    new_ucmd!()
        .args(&["-f", "1,2", "-d:", "--output-delimiter=''"])
        .pipe_in("ab:cd\n")
        .succeeds()
        .stdout_only_bytes("ab''cd\n");
}

#[test]
fn test_newline_as_delimiter() {
    for (field, expected_output) in [("1", "a:1\n"), ("2", "b:\n")] {
        new_ucmd!()
            .args(&["-f", field, "-d", "\n"])
            .pipe_in("a:1\nb:")
            .succeeds()
            .stdout_only_bytes(expected_output);
    }
}

#[test]
fn test_newline_as_delimiter_with_output_delimiter() {
    new_ucmd!()
        .args(&["-f1-", "-d", "\n", "--output-delimiter=:"])
        .pipe_in("a\nb\n")
        .succeeds()
        .stdout_only_bytes("a:b\n");
}

#[test]
fn test_newline_as_delimiter_no_delimiter_suppressed() {
    for param in ["-s", "--only-delimited", "--only-del"] {
        new_ucmd!()
            .args(&["-d", "\n", param, "-f", "1"])
            .pipe_in("abc")
            .succeeds()
            .no_output();
    }
}

#[test]
fn test_newline_as_delimiter_found_not_suppressed() {
    // Has an internal \n delimiter, so -s shouldn't suppress it
    for param in ["-s", "--only-delimited", "--only-del"] {
        new_ucmd!()
            .args(&["-d", "\n", param, "-f", "1"])
            .pipe_in("abc\ndef\n")
            .succeeds()
            .stdout_only("abc\n");
    }
}

#[test]
fn test_newline_as_delimiter_multiple_fields() {
    // Check field selection when \n is the delimiter
    new_ucmd!()
        .args(&["-d", "\n", "-f", "2"])
        .pipe_in("abc\ndef\n")
        .succeeds()
        .stdout_only("def\n");
}

#[test]
fn test_newline_as_delimiter_double_newline() {
    // Field 2 is the empty space between newlines
    new_ucmd!()
        .args(&["-d", "\n", "-s", "-f", "2"])
        .pipe_in("abc\n\n")
        .succeeds()
        .stdout_only("\n");

    // Requesting both fields
    new_ucmd!()
        .args(&["-d", "\n", "-s", "-f", "1,2"])
        .pipe_in("abc\n\n")
        .succeeds()
        .stdout_only("abc\n\n");
}

#[test]
fn test_newline_as_delimiter_only_newlines() {
    // Extracting empty fields from a string of just newlines
    new_ucmd!()
        .args(&["-d", "\n", "-s", "-f", "1"])
        .pipe_in("\n\n")
        .succeeds()
        .stdout_only("\n");

    new_ucmd!()
        .args(&["-d", "\n", "-s", "-f", "2"])
        .pipe_in("\n\n")
        .succeeds()
        .stdout_only("\n");

    new_ucmd!()
        .args(&["-d", "\n", "-s", "-f", "1,2"])
        .pipe_in("\n\n")
        .succeeds()
        .stdout_only("\n\n");
}

#[test]
fn test_newline_as_delimiter_last_field_no_newline() {
    // The last chunk is Field 2 even without a final newline
    new_ucmd!()
        .args(&["-d", "\n", "-f", "2"])
        .pipe_in("abc\ndef")
        .succeeds()
        .stdout_only("def\n");
}

#[test]
fn test_newline_as_delimiter_complement() {
    // Select everything except the second line
    new_ucmd!()
        .args(&["-d", "\n", "-f", "2", "--complement"])
        .pipe_in("line1\nline2\nline3\n")
        .succeeds()
        .stdout_only("line1\nline3\n");
}

#[test]
fn test_newline_as_delimiter_out_of_bounds() {
    // GNU cut: print an empty string + terminator for missing fields
    new_ucmd!()
        .args(&["-d", "\n", "-f", "3"])
        .pipe_in("a\nb\n")
        .succeeds()
        .stdout_only("\n");

    // GNU cut avoids trailing delimiters for out-of-bounds fields when delimiter is \n
    new_ucmd!()
        .args(&["-d", "\n", "-f", "1,3"])
        .pipe_in("a\nb\n")
        .succeeds()
        .stdout_only("a\n");
}

#[test]
fn test_newline_as_delimiter_no_delimiter_prints_all() {
    // GNU cut: If no delimiter is found, the entire line (the whole file)
    // is printed regardless of the field requested, unless -s is used.
    new_ucmd!()
        .args(&["-d", "\n", "-f", "2"])
        .pipe_in("a")
        .succeeds()
        .stdout_only("a\n");
}

#[test]
fn test_newline_as_delimiter_empty_input() {
    new_ucmd!()
        .args(&["-d", "\n", "-f", "1"])
        .pipe_in("")
        .succeeds()
        .no_output();
}

#[test]
fn test_newline_as_delimiter_s_flag_no_newline_at_all() {
    new_ucmd!()
        .args(&["-d", "\n", "-s", "-f", "1"])
        .pipe_in("abc")
        .succeeds()
        .no_output();
}

#[test]
fn test_newline_as_delimiter_single_field_included() {
    for param in ["-s", "--only-delimited", "--only-del"] {
        new_ucmd!()
            .args(&["-d", "\n", param, "-f", "1"])
            .pipe_in("abc\n")
            .succeeds()
            .stdout_only("abc\n"); // GNU cut outputs the field + terminator
    }
}

#[test]
fn test_newline_as_delimiter_intervening_skipped_fields() {
    // Selecting non-adjacent lines (Fields 1 and 3)
    new_ucmd!()
        .args(&["-d", "\n", "-f", "1,3"])
        .pipe_in("line1\nline2\nline3\n")
        .succeeds()
        .stdout_only("line1\nline3\n");
}

#[test]
fn test_newline_as_delimiter_multibyte_normalization() {
    // Ensure multibyte records at EOF still get a normalized newline
    new_ucmd!()
        .args(&["-d", "\n", "-f", "2"])
        .pipe_in("\n😼")
        .succeeds()
        .stdout_only("😼\n");
}

#[test]
fn test_newline_as_delimiter_empty_first_record() {
    // Select Field 2 when Field 1 is empty
    new_ucmd!()
        .args(&["-d", "\n", "-f", "2"])
        .pipe_in("\nb")
        .succeeds()
        .stdout_only("b\n");
}

#[test]
fn test_newline_as_delimiter_overlapping_unordered_ranges() {
    // Request fields out of order and with overlapping ranges
    new_ucmd!()
        .args(&["-d", "\n", "-f", "2-3,1,2"])
        .pipe_in("a\nb\nc\n")
        .succeeds()
        .stdout_only("a\nb\nc\n");
}

#[test]
fn test_newline_as_delimiter_complement_last_record() {
    // Test --complement on the final record
    new_ucmd!()
        .args(&["-d", "\n", "-f", "1", "--complement"])
        .pipe_in("a\nb")
        .succeeds()
        .stdout_only("b\n");
}

#[test]
fn test_multiple_delimiters() {
    new_ucmd!()
        .args(&["-f2", "-d:", "-d="])
        .pipe_in("a:=b\n")
        .succeeds()
        .stdout_only("b\n");

    new_ucmd!()
        .args(&["-f2", "-d=", "-d:"])
        .pipe_in("a:=b\n")
        .succeeds()
        .stdout_only("=b\n");
}

#[test]
fn test_multiple_mode_args() {
    for args in [
        vec!["-b1", "-b2"],
        vec!["-c1", "-c2"],
        vec!["-f1", "-f2"],
        vec!["-b1", "-c2"],
        vec!["-b1", "-f2"],
        vec!["-c1", "-f2"],
        vec!["-b1", "-c2", "-f3"],
    ] {
        new_ucmd!()
            .args(&args)
            .fails()
            .stderr_contains("cut: only one list may be specified");
    }
}

#[test]
#[cfg(unix)]
#[cfg_attr(wasi_runner, ignore = "WASI: argv/filenames must be valid UTF-8")]
fn test_8bit_non_utf8_delimiter() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let delim = OsStr::from_bytes(b"\xAD".as_slice());
    new_ucmd!()
        .arg("-d")
        .arg(delim)
        .args(&["--out=_", "-f2,3", "8bit-delim.txt"])
        .succeeds()
        .stdout_check(|out| out == "b_c\n".as_bytes());
}

#[test]
fn test_newline_preservation_with_f1_option() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.write("1", "a\nb");
    let expected = "a\nb\n";
    ucmd.args(&["-f1-", "1"]).succeeds().stdout_is(expected);
}

#[test]
fn test_output_delimiter_with_character_ranges() {
    new_ucmd!()
        .args(&["-c2-3,4-", "--output-delim=:"])
        .pipe_in("abcdefg\n")
        .succeeds()
        .stdout_only("bc:defg\n");
}

#[test]
fn test_output_delimiter_with_adjacent_ranges() {
    new_ucmd!()
        .args(&["-b1-2,3-4", "--output-d=:"])
        .pipe_in("abcd\n")
        .succeeds()
        .stdout_only("ab:cd\n");
}

#[test]
fn test_fields_merged() {
    // -F: merge adjacent delimiters, default delimiter whitespace, output a space.
    new_ucmd!()
        .args(&["-F", "1,3"])
        .pipe_in("one\ttwo   three\n")
        .succeeds()
        .stdout_only("one three\n");
    new_ucmd!()
        .args(&["-F", "1,3", "-O", "+"])
        .pipe_in("one\ttwo   three\n")
        .succeeds()
        .stdout_only("one+three\n");
    // -F with an explicit delimiter still uses a space as the output delimiter.
    new_ucmd!()
        .args(&["-F", "2,4", "-d", ";"])
        .pipe_in("p;q;r;s\n")
        .succeeds()
        .stdout_only("q s\n");
}

#[test]
fn test_fields_merged_conflicts_with_fields() {
    new_ucmd!()
        .args(&["-f", "3", "-F", "5"])
        .fails_with_code(1)
        .stderr_contains("cut: only one list may be specified");
}

#[test]
fn test_whitespace_delimited_long_and_trimmed() {
    // Long form behaves like -w (leading blanks make an empty first field).
    new_ucmd!()
        .args(&["--whitespace-delimited", "-f1,2"])
        .pipe_in("   alpha beta\n")
        .succeeds()
        .stdout_only("\talpha\n");
    // =trimmed (or it's shortcuts) strips leading/trailing blanks before splitting.
    for trimmed in ["trimmed", "tri", ""] {
        new_ucmd!()
            .arg(format!("--whitespace-delimited={trimmed}"))
            .arg("-f1,2")
            .pipe_in("  hello world  \n")
            .succeeds()
            .stdout_only("hello\tworld\n");
    }
    // With -s a single (undelimited) field is suppressed.
    new_ucmd!()
        .args(&["-s", "--whitespace-delimited=trimmed", "-f1"])
        .pipe_in("   solo   \n")
        .succeeds()
        .stdout_only("");
    // Without -s an undelimited line is printed whole, whatever field is asked
    // for, while a blank-only line collapses to an empty one.
    new_ucmd!()
        .args(&["--whitespace-delimited=trimmed", "-f4"])
        .pipe_in("  loner\n\t\n one two\n")
        .succeeds()
        .stdout_only("loner\n\n\n");
    // Only `trimmed` is a valid value.
    new_ucmd!()
        .args(&["--whitespace-delimited=middle", "-f1"])
        .fails_with_code(1);
}

#[test]
fn test_whitespace_delimited_trimmed_zero_terminated() {
    // The record terminator must not count as a non-blank when trimming, or the
    // trailing blanks of a record would survive and add a phantom field. NUL is
    // the interesting case: unlike `\n` it is not whitespace to begin with.
    new_ucmd!()
        .args(&["-z", "--whitespace-delimited=trimmed", "-f2"])
        .pipe_in(&b"  red  blue  \0 green  pink \0"[..])
        .succeeds()
        .stdout_only_bytes(&b"blue\0pink\0"[..]);
    // Asking past the last field yields an empty record, not the stray blanks.
    new_ucmd!()
        .args(&["-z", "--whitespace-delimited=trimmed", "-f3"])
        .pipe_in(&b"  red  blue  \0"[..])
        .succeeds()
        .stdout_only_bytes(&b"\0"[..]);
    // A blank-only record has no delimiter left after trimming, so -s drops it.
    new_ucmd!()
        .args(&["-z", "-s", "--whitespace-delimited=trimmed", "-f1"])
        .pipe_in(&b"\t \0 amber violet \0"[..])
        .succeeds()
        .stdout_only_bytes(&b"amber\0"[..]);
}

#[test]
#[cfg_attr(wasi_runner, ignore = "WASI: the guest does not inherit LC_ALL")]
#[cfg(target_os = "linux")]
fn test_byte_no_split_partially_selected_char() {
    // -b -n: the selected bytes of a character must reach its end without a
    // hole. "🗿" (f0 9f 97 bf) spans bytes 1-4, "w" is byte 5.
    let stone = &b"\xf0\x9f\x97\xbfw\n"[..];
    // Byte 3 is left out, so the character is split and dropped.
    new_ucmd!()
        .env("LC_ALL", "C.UTF-8")
        .args(&["-b1-2,4-5", "-n"])
        .pipe_in(stone)
        .succeeds()
        .stdout_only_bytes(b"w\n");
    // The same holds when the hole comes from a single-byte range.
    new_ucmd!()
        .env("LC_ALL", "C.UTF-8")
        .args(&["-b2,4", "-n"])
        .pipe_in(stone)
        .succeeds()
        .stdout_only_bytes(b"\n");
    // Selecting only the tail of the character still prints it whole.
    new_ucmd!()
        .env("LC_ALL", "C.UTF-8")
        .args(&["-b3-", "-n"])
        .pipe_in(stone)
        .succeeds()
        .stdout_only_bytes(stone);
    // Adjacent ranges cover it without a hole, and the boundary inside the
    // character emits no output delimiter.
    new_ucmd!()
        .env("LC_ALL", "C.UTF-8")
        .args(&["-b1-3,4-5", "-n", "--output-d=|"])
        .pipe_in(stone)
        .succeeds()
        .stdout_only_bytes(stone);
    // "q€é r": q is byte 1, € (e2 82 ac) bytes 2-4, é (c3 a9) bytes 5-6.
    let mixed = &b"q\xe2\x82\xac\xc3\xa9r\n"[..];
    // The delimiter a range owes is carried to whatever prints next, and only
    // a boundary between two printed characters produces one.
    new_ucmd!()
        .env("LC_ALL", "C.UTF-8")
        .args(&["-b1,2,5-7", "-n", "--output-d=|"])
        .pipe_in(mixed)
        .succeeds()
        .stdout_only_bytes(&b"q|\xc3\xa9r\n"[..]);
    new_ucmd!()
        .env("LC_ALL", "C.UTF-8")
        .args(&["-b1,2-4,5-6", "-n", "--output-d=|"])
        .pipe_in(mixed)
        .succeeds()
        .stdout_only_bytes(&b"q|\xe2\x82\xac|\xc3\xa9\n"[..]);
}

#[test]
fn test_unset_locale_is_byte_oriented() {
    // With no locale set the POSIX default is C, so characters are bytes.
    // "ж" is d0 b6, and -c3 must take just the b6.
    new_ucmd!()
        .env("LC_ALL", "")
        .args(&["-c3"])
        .pipe_in(&b"p\xd0\xb6t\n"[..])
        .succeeds()
        .stdout_only_bytes(b"\xb6\n");
}

#[test]
fn test_newline_delim_suppress_missing_field() {
    // -s with the newline as delimiter must not emit a spurious blank line.
    new_ucmd!()
        .args(&["-s", "-d", "\n", "-f3"])
        .pipe_in("solo\n")
        .succeeds()
        .stdout_only("");
}

#[test]
#[cfg_attr(wasi_runner, ignore = "WASI: the guest does not inherit LC_ALL")]
#[cfg(target_os = "linux")]
fn test_byte_no_split_with_output_delimiter() {
    // -b -n with an output delimiter: a range covering only part of a
    // multibyte character contributes nothing and emits no delimiter.
    // "ü" (c3 bc) spans bytes 1-2; byte 1 alone selects no whole character.
    new_ucmd!()
        .env("LC_ALL", "C.UTF-8")
        .args(&["-b1,3", "-n", "--output-d=|"])
        .pipe_in(&b"\xc3\xbcZ\n"[..])
        .succeeds()
        .stdout_only_bytes(b"Z\n");
}

#[test]
#[cfg_attr(wasi_runner, ignore = "WASI: the guest does not inherit LC_ALL")]
#[cfg(target_os = "linux")]
fn test_field_delimiter_not_split_inside_multibyte_char() {
    use std::os::unix::ffi::OsStrExt;
    // In a UTF-8 locale, a delimiter byte that is part of a multibyte character
    // must not split it. Here U+20AC (€ = e2 82 ac) contains 0xac, and 0xac is
    // also used as a standalone delimiter byte.
    new_ucmd!()
        .env("LC_ALL", "C.UTF-8")
        .arg("-d")
        .arg(std::ffi::OsStr::from_bytes(b"\xac"))
        .arg("-f2")
        .pipe_in(&b"1\xe2\x82\xac2\xac3\n"[..])
        .succeeds()
        .stdout_only_bytes(b"3\n");
    new_ucmd!()
        .env("LC_ALL", "C.UTF-8")
        .arg("-d")
        .arg(std::ffi::OsStr::from_bytes(b"\xac"))
        .arg("-f1")
        .pipe_in(&b"1\xe2\x82\xac2\xac3\n"[..])
        .succeeds()
        .stdout_only_bytes(b"1\xe2\x82\xac2\n");
}

#[test]
#[cfg_attr(wasi_runner, ignore = "WASI: the guest does not inherit LC_ALL")]
#[cfg(target_os = "linux")]
fn test_whitespace_delimiter_unicode_blank() {
    // U+2002 (EN SPACE) is a Unicode blank and splits fields under -w.
    new_ucmd!()
        .env("LC_ALL", "C.UTF-8")
        .args(&["-w", "-f2"])
        .pipe_in(&b"x\xe2\x80\x82y\n"[..])
        .succeeds()
        .stdout_only_bytes(b"y\n");
    // U+2007 (FIGURE SPACE) is not a blank: the line stays a single field and
    // is suppressed by -s.
    new_ucmd!()
        .env("LC_ALL", "C.UTF-8")
        .args(&["-s", "-w", "-f2"])
        .pipe_in(&b"x\xe2\x80\x87y\n"[..])
        .succeeds()
        .stdout_only_bytes(b"");
}

#[test]
fn test_delimiter_multibyte_rejected_in_c_locale() {
    // In the C locale a valid UTF-8 multibyte sequence is several characters.
    // The delimiter is passed as ordinary text so this also runs on Windows.
    new_ucmd!()
        .env("LC_ALL", "C")
        .args(&["-d", "\u{20ac}", "-f1"])
        .fails_with_code(1)
        .stderr_contains("cut: the delimiter must be a single character");
}

#[test]
#[cfg_attr(wasi_runner, ignore = "WASI: the guest does not inherit LC_ALL")]
fn test_emoji_delim() {
    // A multibyte delimiter is only a single character in a UTF-8 locale.
    new_ucmd!()
        .env("LC_ALL", "C.UTF-8")
        .args(&["-d🗿", "-f1"])
        .pipe_in("💐🗿🌹\n")
        .succeeds()
        .stdout_only("💐\n");
    new_ucmd!()
        .env("LC_ALL", "C.UTF-8")
        .args(&["-d🗿", "-f2"])
        .pipe_in("💐🗿🌹\n")
        .succeeds()
        .stdout_only("🌹\n");
}

#[cfg(target_os = "linux")]
#[test]
fn test_failed_write_is_reported() {
    new_ucmd!()
        .arg("-d=")
        .arg("-f1")
        .pipe_in("key=value")
        .set_stdout(std::fs::File::create("/dev/full").unwrap())
        .fails()
        .stderr_is("cut: write error: No space left on device\n");
}

#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(wasi_runner, ignore = "WASI: argv/filenames must be valid UTF-8")]
fn test_cut_non_utf8_paths() {
    use std::fs::File;
    use std::io::Write;
    use std::os::unix::ffi::OsStrExt;
    use uutests::util::TestScenario;
    use uutests::util_name;

    let ts = TestScenario::new(util_name!());
    let test_dir = ts.fixtures.subdir.as_path();

    // Create file directly with non-UTF-8 name
    let file_name = std::ffi::OsStr::from_bytes(b"test_\xFF\xFE.txt");
    let mut file = File::create(test_dir.join(file_name)).unwrap();
    file.write_all(b"a\tb\tc\n1\t2\t3\n").unwrap();

    // Test that cut can handle non-UTF-8 filenames
    ts.ucmd()
        .arg("-f1,3")
        .arg(file_name)
        .succeeds()
        .stdout_only("a\tc\n1\t3\n");
}

// We exercise the GB18030 path with two real two-byte characters that are not
// valid UTF-8: 啊 (0xB0 0xA1) and 中 (0xD6 0xD0). The active encoding comes
// straight from `LC_ALL`, so the host does not need the locale installed.
#[cfg(target_os = "linux")]
const GB_LOCALE: &str = "zh_CN.gb18030";
#[cfg(target_os = "linux")]
const A: &[u8] = b"\xB0\xA1"; // 啊

#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(wasi_runner, ignore = "WASI: argv must be valid UTF-8")]
fn test_cut_fields_gb18030_delimiter() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    // Three words separated by the two-byte 啊: "red啊green啊blue".
    let line = b"red\xB0\xA1green\xB0\xA1blue\n";
    let delim = OsString::from_vec(A.to_vec());

    // Pick the last field; the chosen output delimiter replaces the input one.
    new_ucmd!()
        .env("LC_ALL", GB_LOCALE)
        .arg("-d")
        .arg(&delim)
        .args(&["-f3", "--output-delimiter=/"])
        .pipe_in(line.to_vec())
        .succeeds()
        .stdout_only("blue\n");

    // Two non-adjacent fields; with no override the multibyte delimiter itself
    // is re-emitted between them.
    new_ucmd!()
        .env("LC_ALL", GB_LOCALE)
        .arg("-d")
        .arg(&delim)
        .arg("-f1,3")
        .pipe_in(line.to_vec())
        .succeeds()
        .stdout_only_bytes(b"red\xB0\xA1blue\n");
}

#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(wasi_runner, ignore = "WASI: argv must be valid UTF-8")]
fn test_cut_fields_gb18030_complement_and_gaps() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let delim = OsString::from_vec(A.to_vec());

    // --complement of the middle field leaves the two outer ones, rejoined.
    new_ucmd!()
        .env("LC_ALL", GB_LOCALE)
        .arg("--complement")
        .arg("-d")
        .arg(&delim)
        .arg("-f2")
        .pipe_in(b"red\xB0\xA1green\xB0\xA1blue\n".to_vec())
        .succeeds()
        .stdout_only_bytes(b"red\xB0\xA1blue\n");

    // A line that is only delimiters yields empty fields around a trailing one.
    new_ucmd!()
        .env("LC_ALL", GB_LOCALE)
        .arg("-d")
        .arg(&delim)
        .args(&["-f1-3", "--output-delimiter=|"])
        .pipe_in(b"\xB0\xA1\xB0\xA1z\n".to_vec())
        .succeeds()
        .stdout_only("||z\n");
}

#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(
    wasi_runner,
    ignore = "WASI sandbox: non-UTF-8 arguments can't be passed through wasmtime"
)]
fn test_cut_fields_single_byte_delimiter_in_mb_locale() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    // 0x80 never starts a GB18030 sequence, yet a lone byte is a fine delimiter.
    let delim = OsString::from_vec(vec![0x80]);
    new_ucmd!()
        .env("LC_ALL", GB_LOCALE)
        .arg("-d")
        .arg(&delim)
        .args(&["-f1,3", "--output-delimiter=-"])
        .pipe_in(b"a\x80b\x80c\n".to_vec())
        .succeeds()
        .stdout_only("a-c\n");
}

#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(wasi_runner, ignore = "WASI: argv must be valid UTF-8")]
fn test_cut_chars_gb18030() {
    // "啊w中": -c counts characters, so the second one is the ASCII 'w'.
    let line = b"\xB0\xA1w\xD6\xD0\n";

    new_ucmd!()
        .env("LC_ALL", GB_LOCALE)
        .arg("-c2")
        .pipe_in(line.to_vec())
        .succeeds()
        .stdout_only("w\n");

    // Selecting the trailing multibyte character returns it whole.
    new_ucmd!()
        .env("LC_ALL", GB_LOCALE)
        .arg("-c3")
        .pipe_in(line.to_vec())
        .succeeds()
        .stdout_only_bytes(b"\xD6\xD0\n");

    // A range that spans the leading and ASCII characters.
    new_ucmd!()
        .env("LC_ALL", GB_LOCALE)
        .arg("-c1-2")
        .pipe_in(line.to_vec())
        .succeeds()
        .stdout_only_bytes(b"\xB0\xA1w\n");
}

#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(wasi_runner, ignore = "WASI: argv must be valid UTF-8")]
fn test_cut_chars_gb18030_ranges_and_complement() {
    // "啊w中": list of two single-char ranges joined by a custom delimiter.
    new_ucmd!()
        .env("LC_ALL", GB_LOCALE)
        .args(&["-c1,3", "--output-delimiter=+"])
        .pipe_in(b"\xB0\xA1w\xD6\xD0\n".to_vec())
        .succeeds()
        .stdout_only_bytes(b"\xB0\xA1+\xD6\xD0\n");

    // Complement of the ASCII middle character keeps both multibyte ones.
    new_ucmd!()
        .env("LC_ALL", GB_LOCALE)
        .args(&["--complement", "-c2"])
        .pipe_in(b"\xB0\xA1w\xD6\xD0\n".to_vec())
        .succeeds()
        .stdout_only_bytes(b"\xB0\xA1\xD6\xD0\n");
}

#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(wasi_runner, ignore = "WASI: argv must be valid UTF-8")]
fn test_cut_bytes_no_split_gb18030() {
    // -n forbids splitting a multibyte character: a byte index landing inside
    // 啊 only produces output once its final byte is included.
    let line = b"\xB0\xA1w\n";

    // Byte 1 is the first half of 啊 -> nothing is emitted.
    new_ucmd!()
        .env("LC_ALL", GB_LOCALE)
        .args(&["-b1", "-n"])
        .pipe_in(line.to_vec())
        .succeeds()
        .stdout_only("\n");

    // Byte 2 completes 啊 -> the whole character comes out.
    new_ucmd!()
        .env("LC_ALL", GB_LOCALE)
        .args(&["-b2", "-n"])
        .pipe_in(line.to_vec())
        .succeeds()
        .stdout_only_bytes(b"\xB0\xA1\n");
}

// `-c` also operates on whole characters in a UTF-8 locale. The harness runs
// under `LC_ALL=C`, so the locale is forced here. "naïve" is n a ï(2 bytes) v e.
#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(wasi_runner, ignore = "WASI: argv must be valid UTF-8")]
fn test_cut_chars_utf8() {
    // The third character is the accented 'ï', returned in full.
    new_ucmd!()
        .env("LC_ALL", "en_US.UTF-8")
        .arg("-c3")
        .pipe_in("naïve\n".as_bytes().to_vec())
        .succeeds()
        .stdout_only("ï\n");

    // Complement of that character removes it and nothing else.
    new_ucmd!()
        .env("LC_ALL", "en_US.UTF-8")
        .args(&["--complement", "-c3"])
        .pipe_in("naïve\n".as_bytes().to_vec())
        .succeeds()
        .stdout_only("nave\n");

    // A list straddling 'ï' joins the two picked characters.
    new_ucmd!()
        .env("LC_ALL", "en_US.UTF-8")
        .args(&["-c1,4", "--output-delimiter=+"])
        .pipe_in("naïve\n".as_bytes().to_vec())
        .succeeds()
        .stdout_only("n+v\n");
}

// Lines with no byte above 0x7F take a byte-wise shortcut in a multi-byte
// locale; mixing them with multi-byte ones checks both paths agree on offsets.
#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(wasi_runner, ignore = "WASI: argv must be valid UTF-8")]
fn test_cut_chars_utf8_mixed_ascii_lines() {
    let input = "quokka\nfjärd\nwombat\ntøys\n";

    new_ucmd!()
        .env("LC_ALL", "en_US.UTF-8")
        .arg("-c3-5")
        .pipe_in(input)
        .succeeds()
        .stdout_only("okk\närd\nmba\nys\n");

    // `-b -n` selects a character when its last byte falls in the range, so the
    // accented lines cover a different span than the ASCII ones.
    new_ucmd!()
        .env("LC_ALL", "en_US.UTF-8")
        .args(&["-b", "3-5", "-n"])
        .pipe_in(input)
        .succeeds()
        .stdout_only("okk\när\nmba\nøys\n");
}

#[test]
#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[cfg_attr(wasi_runner, ignore)]
fn test_read_error() {
    new_ucmd!()
        .args(&["-c1", "/proc/self/mem"])
        .fails_with_code(1)
        .stderr_is("cut: Input/output error\n");
}

#[cfg(unix)]
#[cfg(all(feature = "feat_diagnostics", not(wasi_runner)))]
mod diagnostics {
    use super::*;

    #[test]
    fn test_snippet_points_at_the_inverted_range() {
        let result = new_ucmd!()
            .terminal_sim_stderr()
            .args(&["-f", "1,4-2", "/dev/null"])
            .fails_with_code(1);

        // One item of the list is at fault, not the whole of it.
        let stderr = result.stderr_as_displayed();
        assert!(
            stderr.starts_with(
                "\
cut: invalid decreasing range
   ╭─[ cut:1:10 ]
   │
 1 │ cut -f 1,4-2 /dev/null
   │          ─┬─
   │           ╰─── this range ends before it starts
   │
   │ Help: a list is N, N-M, N- or -M, separated by commas, as in -f1,4-6,9-
───╯"
            ),
            "{stderr}"
        );
        // The caret replaces the message, not the usage hint: a pipe and a
        // terminal must not disagree on whether one was printed.
        assert!(
            stderr.ends_with("cut --help' for more information."),
            "{stderr}"
        );
    }

    #[test]
    fn test_snippet_finds_fields_merged_value() {
        let result = new_ucmd!()
            .terminal_sim_stderr()
            .args(&["-F", "1,4-2", "/dev/null"])
            .fails_with_code(1);
        let stderr = result.stderr_as_displayed();

        assert!(stderr.contains("1 │ cut -F 1,4-2 /dev/null"), "{stderr}");
        assert!(
            stderr.contains("this range ends before it starts"),
            "{stderr}"
        );
    }

    #[test]
    fn test_snippet_points_at_the_zero_bound() {
        let result = new_ucmd!()
            .terminal_sim_stderr()
            .args(&["-f1,0,3", "/dev/null"])
            .fails_with_code(1);
        let stderr = result.stderr_as_displayed();

        // Glued to the option, so the caret counts the two columns it takes.
        assert!(stderr.contains("cut:1:9"), "{stderr}");
        assert!(stderr.contains("counting starts at 1"), "{stderr}");
    }

    #[test]
    fn test_snippet_points_at_the_bound_that_is_not_a_number() {
        let result = new_ucmd!()
            .terminal_sim_stderr()
            .args(&["-c", "1-3,x", "/dev/null"])
            .fails_with_code(1);
        let stderr = result.stderr_as_displayed();

        // The message names the item, so a bare underline is enough.
        assert!(
            stderr.contains("invalid byte/character position 'x'"),
            "{stderr}"
        );
        assert!(stderr.contains("cut:1:12"), "{stderr}");
    }

    #[test]
    fn test_snippet_ignores_a_file_named_like_the_list() {
        let (at, mut ucmd) = at_and_ucmd!();
        at.touch("1,0");

        // The file is named exactly like the list; the caret belongs to the
        // value of -f.
        let result = ucmd
            .terminal_sim_stderr()
            .args(&["-f", "1,0", "1,0"])
            .fails_with_code(1);
        let stderr = result.stderr_as_displayed();

        assert!(stderr.contains("cut:1:10"), "{stderr}");
    }

    #[test]
    fn test_plain_message_when_stderr_is_a_pipe() {
        let result = new_ucmd!()
            .args(&["-f", "1,4-2", "/dev/null"])
            .fails_with_code(1);
        let stderr = result.stderr_str();

        // The message reads as it always has, and nothing is drawn under it;
        // the usage hint that follows is cut's own.
        assert!(
            stderr.starts_with("cut: invalid decreasing range\n"),
            "{stderr}"
        );
        assert!(!stderr.contains('\u{256d}'), "{stderr}");
    }
}
