// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore defg naïve nave

use uutests::at_and_ucmd;
use uutests::new_ucmd;

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

static COMPLEX_SEQUENCE: &TestedSequence = &TestedSequence {
    name: "",
    sequence: "9-,6-7,-2,4",
};

#[test]
fn test_no_args() {
    new_ucmd!().fails().stderr_is(
        "cut: invalid usage: expects one of --fields (-f), --chars (-c) or --bytes (-b)\n",
    );
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
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
        .args(&["-w", "-f", COMPLEX_SEQUENCE.sequence, INPUT])
        .succeeds()
        .stdout_only_fixture("whitespace_delimited.expected");
}

#[test]
fn test_whitespace_with_explicit_delimiter() {
    new_ucmd!()
        .args(&["-w", "-f", COMPLEX_SEQUENCE.sequence, "-d:"])
        .fails_with_code(1);
}

#[test]
fn test_whitespace_with_byte() {
    new_ucmd!()
        .args(&["-w", "-b", COMPLEX_SEQUENCE.sequence])
        .fails_with_code(1);
}

#[test]
fn test_whitespace_with_char() {
    new_ucmd!()
        .args(&["-c", COMPLEX_SEQUENCE.sequence, "-w"])
        .fails_with_code(1);
}

#[test]
fn test_delimiter_with_byte_and_char() {
    for conflicting_arg in ["-c", "-b"] {
        new_ucmd!()
            .args(&[conflicting_arg, COMPLEX_SEQUENCE.sequence, "-d="])
            .fails_with_code(1)
            .stderr_is("cut: invalid input: The '--delimiter' ('-d') option can only be used when printing a sequence of fields\n")
;
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
            .args(&[param, ":", "-f", COMPLEX_SEQUENCE.sequence, INPUT])
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
    // we use -d here to ensure output delimiter
    // is applied to the current, and not just the default, input delimiter
    new_ucmd!()
        .args(&[
            "-d:",
            "--output-delimiter=@",
            "-f",
            COMPLEX_SEQUENCE.sequence,
            INPUT,
        ])
        .succeeds()
        .stdout_only_fixture("output_delimiter.expected");

    new_ucmd!()
        .args(&[
            "-d:",
            "--output-del=@",
            "-f",
            COMPLEX_SEQUENCE.sequence,
            INPUT,
        ])
        .succeeds()
        .stdout_only_fixture("output_delimiter.expected");
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
        .stderr_is("cut: invalid usage: expects no more than one of --fields (-f), --chars (-c) or --bytes (-b)\n");
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
fn test_emoji_delim() {
    new_ucmd!()
        .args(&["-d🗿", "-f1"])
        .pipe_in("💐🗿🌹\n")
        .succeeds()
        .stdout_only("💐\n");
    new_ucmd!()
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
