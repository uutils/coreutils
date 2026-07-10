// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore defg

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

// 你 (U+4F60) encodes as the two bytes 0xC4 0xE3 in GB18030.
#[cfg(target_os = "linux")]
const GB18030_NI: &[u8] = b"\xC4\xE3";
// 好 (U+597D) encodes as the two bytes 0xBA 0xC3 in GB18030.
#[cfg(target_os = "linux")]
const GB18030_HAO: &[u8] = b"\xBA\xC3";

#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(wasi_runner, ignore = "WASI: argv must be valid UTF-8")]
fn test_gb18030_multibyte_delimiter() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    // Use a 2-byte GB18030 character as the field delimiter.
    let delim = OsString::from_vec(GB18030_NI.to_vec());

    // Drop the middle field: "a<delim>b<delim>c" keeping fields 1 and 3.
    new_ucmd!()
        .env("LC_ALL", "zh_CN.gb18030")
        .arg("-d")
        .arg(&delim)
        .args(&["-f1,3", "--output-delimiter=-"])
        .pipe_in(&b"a\xC4\xE3b\xC4\xE3c\n"[..])
        .succeeds()
        .stdout_only_bytes(b"a-c\n");

    // Leading empty fields: two delimiters then a trailing field, all selected.
    new_ucmd!()
        .env("LC_ALL", "zh_CN.gb18030")
        .arg("-d")
        .arg(&delim)
        .args(&["-f1-3", "--output-delimiter=|"])
        .pipe_in(&b"\xC4\xE3\xC4\xE3z\n"[..])
        .succeeds()
        .stdout_only_bytes(b"||z\n");
}

#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(wasi_runner, ignore = "WASI: argv must be valid UTF-8")]
fn test_gb18030_complement_multibyte_delimiter() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    let delim = OsString::from_vec(GB18030_NI.to_vec());

    // --complement of field 2 keeps the surrounding fields and the delimiters.
    new_ucmd!()
        .env("LC_ALL", "zh_CN.gb18030")
        .arg("-d")
        .arg(&delim)
        .args(&["--complement", "-f2"])
        .pipe_in(&b"a\xC4\xE3b\xC4\xE3c\n"[..])
        .succeeds()
        .stdout_only_bytes(b"a\xC4\xE3c\n");
}

#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(wasi_runner, ignore = "WASI: argv must be valid UTF-8")]
fn test_gb18030_single_byte_delimiter_is_accepted() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    // 0xFE never starts a GB18030 character, yet any single byte is still a
    // legal delimiter.
    let delim = OsString::from_vec(vec![0xFE]);
    new_ucmd!()
        .env("LC_ALL", "zh_CN.gb18030")
        .arg("-d")
        .arg(&delim)
        .args(&["-f2,3", "--output-delimiter=-"])
        .pipe_in(&b"p\xFEq\xFEr\n"[..])
        .succeeds()
        .stdout_only_bytes(b"q-r\n");
}

#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(wasi_runner, ignore = "WASI: LC_ALL is not propagated to the guest")]
fn test_gb18030_character_mode() {
    // Input: the 2-byte character 好 followed by the ASCII byte 'y'.
    let mut input = GB18030_HAO.to_vec();
    input.extend_from_slice(b"y\n");

    // Character 1 is the whole multibyte character.
    new_ucmd!()
        .env("LC_ALL", "zh_CN.gb18030")
        .args(&["-c1"])
        .pipe_in(&input[..])
        .succeeds()
        .stdout_only_bytes(b"\xBA\xC3\n");
    // Character 2 is the trailing ASCII byte.
    new_ucmd!()
        .env("LC_ALL", "zh_CN.gb18030")
        .args(&["-c2"])
        .pipe_in(&input[..])
        .succeeds()
        .stdout_only_bytes(b"y\n");
}

#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(wasi_runner, ignore = "WASI: LC_ALL is not propagated to the guest")]
fn test_gb18030_byte_mode_no_split() {
    // With -n a multibyte character is never split: it is printed only when the
    // selected byte range reaches its final byte.
    let mut input = GB18030_HAO.to_vec();
    input.extend_from_slice(b"y\n");

    // Byte 1 falls in the middle of 好, so nothing is emitted for it.
    new_ucmd!()
        .env("LC_ALL", "zh_CN.gb18030")
        .args(&["-b1", "-n"])
        .pipe_in(&input[..])
        .succeeds()
        .stdout_only_bytes(b"\n");
    // Byte 2 completes 好, so the full character is emitted.
    new_ucmd!()
        .env("LC_ALL", "zh_CN.gb18030")
        .args(&["-b2", "-n"])
        .pipe_in(&input[..])
        .succeeds()
        .stdout_only_bytes(b"\xBA\xC3\n");
}

#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(wasi_runner, ignore = "WASI: LC_ALL is not propagated to the guest")]
fn test_gb18030_byte_mode_no_split_output_delimiter() {
    // A -n range that selects no complete character must not emit an output
    // delimiter: byte 1 (mid-character) yields nothing, so no leading delimiter
    // precedes the 'z' selected by byte 3.
    let mut input = GB18030_HAO.to_vec();
    input.extend_from_slice(b"z\n");
    new_ucmd!()
        .env("LC_ALL", "zh_CN.gb18030")
        .args(&["-b1,3", "-n", "--output-delimiter=|"])
        .pipe_in(&input[..])
        .succeeds()
        .stdout_only_bytes(b"z\n");

    // Two ranges that both select nothing produce an empty line, not a stray
    // delimiter.
    let mut two = GB18030_HAO.to_vec();
    two.extend_from_slice(GB18030_HAO);
    two.push(b'\n');
    new_ucmd!()
        .env("LC_ALL", "zh_CN.gb18030")
        .args(&["-b1,3", "-n", "--output-delimiter=|"])
        .pipe_in(&two[..])
        .succeeds()
        .stdout_only_bytes(b"\n");

    // The delimiter is still emitted between two ranges that each select a full
    // character.
    new_ucmd!()
        .env("LC_ALL", "zh_CN.gb18030")
        .args(&["-b2,4", "-n", "--output-delimiter=|"])
        .pipe_in(&two[..])
        .succeeds()
        .stdout_only_bytes(b"\xBA\xC3|\xBA\xC3\n");
}

// In the C/POSIX locale there are no multibyte characters: every byte is its own
// unit, so `-c` and `-b -n` must select individual bytes and never decode a
// UTF-8 sequence. The bytes 0xC3 0xB1 spell ñ in UTF-8, but under LC_ALL=C they
// are two unrelated bytes.
#[test]
#[cfg(target_os = "linux")]
fn test_c_locale_is_byte_wise() {
    // `-c2` selects the single second byte, not a decoded two-byte character.
    new_ucmd!()
        .env("LC_ALL", "C")
        .args(&["-c2"])
        .pipe_in(&b"A\xC3\xB1Z\n"[..])
        .succeeds()
        .stdout_only_bytes(b"\xC3\n");

    // `-b2 -n` also selects one byte: there is no multibyte character to keep
    // whole, so `-n` is a no-op.
    new_ucmd!()
        .env("LC_ALL", "C")
        .args(&["-b2", "-n"])
        .pipe_in(&b"A\xC3\xB1Z\n"[..])
        .succeeds()
        .stdout_only_bytes(b"\xC3\n");
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
