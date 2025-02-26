// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[cfg(target_os = "linux")]
use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::{vec_of_size, TestScenario};
use uutests::util_name;

// spell-checker:ignore (flags) lwmcL clmwL ; (path) bogusfile emptyfile manyemptylines moby notrailingnewline onelongemptyline onelongword weirdchars
#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

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
        let expected = format!("{n}\n");
        new_ucmd!()
            .args(&["-c"])
            .pipe_in(data)
            .succeeds()
            .stdout_is_bytes(expected.as_bytes());
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
fn test_utf8_words() {
    new_ucmd!()
        .arg("-w")
        .pipe_in_fixture("UTF_8_weirdchars.txt")
        .run()
        .stdout_is("89\n");
}

#[test]
fn test_utf8_line_length_words() {
    new_ucmd!()
        .arg("-Lw")
        .pipe_in_fixture("UTF_8_weirdchars.txt")
        .run()
        .stdout_is("     89      48\n");
}

#[test]
fn test_utf8_line_length_chars() {
    new_ucmd!()
        .arg("-Lm")
        .pipe_in_fixture("UTF_8_weirdchars.txt")
        .run()
        .stdout_is("    442      48\n");
}

#[test]
fn test_utf8_line_length_chars_words() {
    new_ucmd!()
        .arg("-Lmw")
        .pipe_in_fixture("UTF_8_weirdchars.txt")
        .run()
        .stdout_is("     89     442      48\n");
}

#[test]
fn test_utf8_chars() {
    new_ucmd!()
        .arg("-m")
        .pipe_in_fixture("UTF_8_weirdchars.txt")
        .run()
        .stdout_is("442\n");
}

#[test]
fn test_utf8_bytes_chars() {
    new_ucmd!()
        .arg("-cm")
        .pipe_in_fixture("UTF_8_weirdchars.txt")
        .run()
        .stdout_is("    442     513\n");
}

#[test]
fn test_utf8_bytes_lines() {
    new_ucmd!()
        .arg("-cl")
        .pipe_in_fixture("UTF_8_weirdchars.txt")
        .run()
        .stdout_is("     25     513\n");
}

#[test]
fn test_utf8_bytes_chars_lines() {
    new_ucmd!()
        .arg("-cml")
        .pipe_in_fixture("UTF_8_weirdchars.txt")
        .run()
        .stdout_is("     25     442     513\n");
}

#[test]
fn test_utf8_chars_words() {
    new_ucmd!()
        .arg("-mw")
        .pipe_in_fixture("UTF_8_weirdchars.txt")
        .run()
        .stdout_is("     89     442\n");
}

#[test]
fn test_utf8_line_length_lines() {
    new_ucmd!()
        .arg("-Ll")
        .pipe_in_fixture("UTF_8_weirdchars.txt")
        .run()
        .stdout_is("     25      48\n");
}

#[test]
fn test_utf8_line_length_lines_words() {
    new_ucmd!()
        .arg("-Llw")
        .pipe_in_fixture("UTF_8_weirdchars.txt")
        .run()
        .stdout_is("     25      89      48\n");
}

#[test]
fn test_utf8_lines_chars() {
    new_ucmd!()
        .arg("-ml")
        .pipe_in_fixture("UTF_8_weirdchars.txt")
        .run()
        .stdout_is("     25     442\n");
}

#[test]
fn test_utf8_lines_words_chars() {
    new_ucmd!()
        .arg("-mlw")
        .pipe_in_fixture("UTF_8_weirdchars.txt")
        .run()
        .stdout_is("     25      89     442\n");
}

#[test]
fn test_utf8_line_length_lines_chars() {
    new_ucmd!()
        .arg("-Llm")
        .pipe_in_fixture("UTF_8_weirdchars.txt")
        .run()
        .stdout_is("     25     442      48\n");
}

#[test]
fn test_utf8_all() {
    new_ucmd!()
        .arg("-lwmcL")
        .pipe_in_fixture("UTF_8_weirdchars.txt")
        .run()
        .stdout_is("     25      89     442     513      48\n");
}

#[test]
fn test_ascii_control() {
    // GNU coreutils "d1" test
    new_ucmd!()
        .arg("-w")
        .pipe_in(*b"\x01\n")
        .run()
        .stdout_is("1\n");
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
fn test_single_only_bytes() {
    new_ucmd!()
        .args(&["-c", "lorem_ipsum.txt"])
        .run()
        .stdout_is("772 lorem_ipsum.txt\n");
}

#[test]
fn test_single_all_counts() {
    new_ucmd!()
        .args(&["-c", "-l", "-L", "-m", "-w", "alice_in_wonderland.txt"])
        .run()
        .stdout_is("  5  57 302 302  66 alice_in_wonderland.txt\n");
}

#[cfg(unix)]
#[test]
fn test_gnu_compatible_quotation() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("some-dir1");
    at.touch("some-dir1/12\n34.txt");
    scene
        .ucmd()
        .args(&["some-dir1/12\n34.txt"])
        .run()
        .stdout_is("0 0 0 'some-dir1/12'$'\\n''34.txt'\n");
}

#[cfg(feature = "test_risky_names")]
#[test]
fn test_non_unicode_names() {
    let scene = TestScenario::new(util_name!());
    let target1 = uucore::os_str_from_bytes(b"some-dir1/1\xC0\n.txt")
        .expect("Only unix platforms can test non-unicode names");
    let target2 = uucore::os_str_from_bytes(b"some-dir1/2\xC0\t.txt")
        .expect("Only unix platforms can test non-unicode names");
    let at = &scene.fixtures;
    at.mkdir("some-dir1");
    at.touch(&target1);
    at.touch(&target2);
    scene
        .ucmd()
        .args(&[target1, target2])
        .run()
        .stdout_is_bytes(
            [
                b"0 0 0 'some-dir1/1'$'\\300\\n''.txt'\n".to_vec(),
                b"0 0 0 some-dir1/2\xC0\t.txt\n".to_vec(),
                b"0 0 0 total\n".to_vec(),
            ]
            .concat(),
        );
}

#[test]
fn test_multiple_default() {
    new_ucmd!()
        .args(&[
            "lorem_ipsum.txt",
            "moby_dick.txt",
            "alice_in_wonderland.txt",
            "alice in wonderland.txt",
        ])
        .run()
        .stdout_is(concat!(
            "  13  109  772 lorem_ipsum.txt\n",
            "  18  204 1115 moby_dick.txt\n",
            "   5   57  302 alice_in_wonderland.txt\n",
            "   5   57  302 alice in wonderland.txt\n",
            "  41  427 2491 total\n",
        ));
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
    // . is a directory, so minimum_width should get set to 7
    #[cfg(not(windows))]
    const STDOUT: &str = concat!(
        "      0       0       0 emptyfile.txt\n",
        "      0       0       0 .\n",
        "      0       0       0 total\n",
    );
    #[cfg(windows)]
    const STDOUT: &str = concat!(
        "      0       0       0 emptyfile.txt\n",
        "      0       0       0 total\n",
    );

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
        .stdout_is(concat!(
            "   5   57  302 alice_in_wonderland.txt\n",
            "  13  109  772 lorem_ipsum.txt\n",
            "  18  166 1074 total\n",
        ));

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
    const STDERR: &str = ".: Permission denied";

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
    new_ucmd!()
        .args(&["bogusfile"])
        .fails()
        .stderr_only("wc: bogusfile: No such file or directory\n");
}

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn test_files_from_pseudo_filesystem() {
    use pretty_assertions::assert_ne;
    let result = new_ucmd!().arg("-c").arg("/proc/cpuinfo").succeeds();
    assert_ne!(result.stdout_str(), "0 /proc/cpuinfo\n");

    // the following block fails on Android with a "Permission denied" error
    #[cfg(target_os = "linux")]
    {
        let (at, mut ucmd) = at_and_ucmd!();
        let result = ucmd.arg("-c").arg("/sys/kernel/profiling").succeeds();
        let actual = at.read("/sys/kernel/profiling").len();
        assert_eq!(
            result.stdout_str(),
            format!("{actual} /sys/kernel/profiling\n")
        );
    }
}

#[test]
fn test_files0_disabled_files_argument() {
    const MSG: &str =
        "extra operand 'lorem_ipsum.txt'\nfile operands cannot be combined with --files0-from";
    new_ucmd!()
        .args(&["--files0-from=files0_list.txt"])
        .arg("lorem_ipsum.txt")
        .fails()
        .stderr_contains(MSG)
        .stdout_is("");
}

#[test]
fn test_files0_from() {
    // file
    new_ucmd!()
        .args(&["--files0-from=files0_list.txt"])
        .run()
        .success()
        .stdout_is(concat!(
            "  13  109  772 lorem_ipsum.txt\n",
            "  18  204 1115 moby_dick.txt\n",
            "   5   57  302 alice_in_wonderland.txt\n",
            "  36  370 2189 total\n",
        ));

    // stream
    new_ucmd!()
        .args(&["--files0-from=-"])
        .pipe_in_fixture("files0_list.txt")
        .run()
        .success()
        .stdout_is(concat!(
            "13 109 772 lorem_ipsum.txt\n",
            "18 204 1115 moby_dick.txt\n",
            "5 57 302 alice_in_wonderland.txt\n",
            "36 370 2189 total\n",
        ));
}

#[test]
fn test_files0_from_with_stdin() {
    new_ucmd!()
        .args(&["--files0-from=-"])
        .pipe_in("lorem_ipsum.txt")
        .run()
        .stdout_is("13 109 772 lorem_ipsum.txt\n");
}

#[test]
fn test_files0_from_with_stdin_in_file() {
    new_ucmd!()
        .args(&["--files0-from=files0_list_with_stdin.txt"])
        .pipe_in_fixture("alice_in_wonderland.txt")
        .run()
        .stdout_is(concat!(
            "     13     109     772 lorem_ipsum.txt\n",
            "     18     204    1115 moby_dick.txt\n",
            "      5      57     302 -\n", // alice_in_wonderland.txt
            "     36     370    2189 total\n",
        ));
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

#[test]
fn test_total_auto() {
    new_ucmd!()
        .args(&["lorem_ipsum.txt", "--total=auto"])
        .run()
        .stdout_is(" 13 109 772 lorem_ipsum.txt\n");
    new_ucmd!()
        .args(&["lorem_ipsum.txt", "--tot=au"])
        .run()
        .stdout_is(" 13 109 772 lorem_ipsum.txt\n");

    new_ucmd!()
        .args(&["lorem_ipsum.txt", "moby_dick.txt", "--total=auto"])
        .run()
        .stdout_is(concat!(
            "  13  109  772 lorem_ipsum.txt\n",
            "  18  204 1115 moby_dick.txt\n",
            "  31  313 1887 total\n",
        ));
}

#[test]
fn test_total_always() {
    new_ucmd!()
        .args(&["lorem_ipsum.txt", "--total=always"])
        .run()
        .stdout_is(concat!(
            " 13 109 772 lorem_ipsum.txt\n",
            " 13 109 772 total\n",
        ));
    new_ucmd!()
        .args(&["lorem_ipsum.txt", "--total=al"])
        .run()
        .stdout_is(concat!(
            " 13 109 772 lorem_ipsum.txt\n",
            " 13 109 772 total\n",
        ));

    new_ucmd!()
        .args(&["lorem_ipsum.txt", "moby_dick.txt", "--total=always"])
        .run()
        .stdout_is(concat!(
            "  13  109  772 lorem_ipsum.txt\n",
            "  18  204 1115 moby_dick.txt\n",
            "  31  313 1887 total\n",
        ));
}

#[test]
fn test_total_never() {
    new_ucmd!()
        .args(&["lorem_ipsum.txt", "--total=never"])
        .run()
        .stdout_is(" 13 109 772 lorem_ipsum.txt\n");

    new_ucmd!()
        .args(&["lorem_ipsum.txt", "moby_dick.txt", "--total=never"])
        .run()
        .stdout_is(concat!(
            "  13  109  772 lorem_ipsum.txt\n",
            "  18  204 1115 moby_dick.txt\n",
        ));
    new_ucmd!()
        .args(&["lorem_ipsum.txt", "moby_dick.txt", "--total=n"])
        .run()
        .stdout_is(concat!(
            "  13  109  772 lorem_ipsum.txt\n",
            "  18  204 1115 moby_dick.txt\n",
        ));
}

#[test]
fn test_total_only() {
    new_ucmd!()
        .args(&["lorem_ipsum.txt", "--total=only"])
        .run()
        .stdout_is("13 109 772\n");

    new_ucmd!()
        .args(&["lorem_ipsum.txt", "moby_dick.txt", "--total=only"])
        .run()
        .stdout_is("31 313 1887\n");
    new_ucmd!()
        .args(&["lorem_ipsum.txt", "moby_dick.txt", "--t=o"])
        .run()
        .stdout_is("31 313 1887\n");
}

#[test]
fn test_zero_length_files() {
    // A trailing zero is ignored, but otherwise empty file names are an error...
    const LIST: &str = "\0moby_dick.txt\0\0alice_in_wonderland.txt\0\0lorem_ipsum.txt\0";

    // Try with and without the last \0
    for l in [LIST.len(), LIST.len() - 1] {
        new_ucmd!()
            .args(&["--files0-from=-"])
            .pipe_in(&LIST[..l])
            .run()
            .failure()
            .stdout_is(concat!(
                "18 204 1115 moby_dick.txt\n",
                "5 57 302 alice_in_wonderland.txt\n",
                "13 109 772 lorem_ipsum.txt\n",
                "36 370 2189 total\n",
            ))
            .stderr_is(concat!(
                "wc: -:1: invalid zero-length file name\n",
                "wc: -:3: invalid zero-length file name\n",
                "wc: -:5: invalid zero-length file name\n",
            ));
    }

    // But, just as important, a zero-length file name may still be at the end...
    new_ucmd!()
        .args(&["--files0-from=-"])
        .pipe_in(
            LIST.as_bytes()
                .iter()
                .chain(b"\0")
                .copied()
                .collect::<Vec<_>>(),
        )
        .run()
        .failure()
        .stdout_is(concat!(
            "18 204 1115 moby_dick.txt\n",
            "5 57 302 alice_in_wonderland.txt\n",
            "13 109 772 lorem_ipsum.txt\n",
            "36 370 2189 total\n",
        ))
        .stderr_is(concat!(
            "wc: -:1: invalid zero-length file name\n",
            "wc: -:3: invalid zero-length file name\n",
            "wc: -:5: invalid zero-length file name\n",
            "wc: -:7: invalid zero-length file name\n",
        ));
}

#[test]
fn test_files0_errors_quoting() {
    new_ucmd!()
        .args(&["--files0-from=files0 with nonexistent.txt"])
        .run()
        .failure()
        .stderr_is(concat!(
            "wc: this_file_does_not_exist.txt: No such file or directory\n",
            "wc: 'files0 with nonexistent.txt':2: invalid zero-length file name\n",
            "wc: 'this file does not exist.txt': No such file or directory\n",
            "wc: \"this files doesn't exist either.txt\": No such file or directory\n",
        ))
        .stdout_is("0 0 0 total\n");
}

#[test]
fn test_files0_progressive_stream() {
    use std::process::Stdio;
    // You should be able to run wc and have a back-and-forth exchange with wc...
    let mut child = new_ucmd!()
        .args(&["--files0-from=-"])
        .set_stdin(Stdio::piped())
        .set_stdout(Stdio::piped())
        .set_stderr(Stdio::piped())
        .run_no_wait();

    macro_rules! chk {
        ($fn:ident, $exp:literal) => {
            assert_eq!(child.$fn($exp.len()), $exp.as_bytes());
        };
    }

    // File in, count out...
    child.write_in("moby_dick.txt\0");
    chk!(stdout_exact_bytes, "18 204 1115 moby_dick.txt\n");
    child.write_in("lorem_ipsum.txt\0");
    chk!(stdout_exact_bytes, "13 109 772 lorem_ipsum.txt\n");

    // Introduce an error!
    child.write_in("\0");
    chk!(
        stderr_exact_bytes,
        "wc: -:3: invalid zero-length file name\n"
    );

    // wc is quick to forgive, let's move on...
    child.write_in("alice_in_wonderland.txt\0");
    chk!(stdout_exact_bytes, "5 57 302 alice_in_wonderland.txt\n");

    // Fin.
    child
        .wait()
        .expect("wc should finish")
        .failure()
        .stdout_only("36 370 2189 total\n");
}

#[test]
fn files0_from_dir() {
    // On Unix, `read(open("."))` fails. On Windows, `open(".")` fails. Thus, the errors happen in
    // different contexts.
    #[cfg(not(windows))]
    macro_rules! dir_err {
        ($p:literal) => {
            concat!("wc: ", $p, ": read error: Is a directory\n")
        };
    }
    #[cfg(windows)]
    macro_rules! dir_err {
        ($p:literal) => {
            concat!("wc: cannot open ", $p, " for reading: Permission denied\n")
        };
    }
    #[cfg(windows)]
    const DOT_ERR: &str = dir_err!("'.'");
    #[cfg(not(windows))]
    const DOT_ERR: &str = dir_err!(".");

    new_ucmd!()
        .args(&["--files0-from=dir with spaces"])
        .fails()
        .stderr_only(dir_err!("'dir with spaces'"));

    // Those contexts have different rules about quoting in errors...
    new_ucmd!()
        .args(&["--files0-from=."])
        .fails()
        .stderr_only(DOT_ERR);

    // That also means you cannot `< . wc --files0-from=-` on Windows.
    #[cfg(not(windows))]
    new_ucmd!()
        .args(&["--files0-from=-"])
        .set_stdin(std::fs::File::open(".").unwrap())
        .fails()
        .stderr_only(dir_err!("-"));
}

#[test]
fn test_args_override() {
    new_ucmd!()
        .args(&["-ll", "-l", "alice_in_wonderland.txt"])
        .run()
        .stdout_is("5 alice_in_wonderland.txt\n");

    new_ucmd!()
        .args(&["--total=always", "--total=never", "alice_in_wonderland.txt"])
        .run()
        .stdout_is("  5  57 302 alice_in_wonderland.txt\n");
}
