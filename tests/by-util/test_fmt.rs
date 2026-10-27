// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore plass samp FFFD

#[cfg(target_os = "linux")]
use std::os::unix::ffi::OsStringExt;
use uutests::new_ucmd;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_invalid_input() {
    new_ucmd!().arg(".").fails_with_code(1);
}

#[test]
fn test_fmt() {
    new_ucmd!()
        .arg("one-word-per-line.txt")
        .succeeds()
        .stdout_is("this is a file with one word per line\n");
}

#[test]
fn test_fmt_quick() {
    for param in ["-q", "--quick", "-qq"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param])
            .succeeds()
            .stdout_is("this is a file with one word per line\n");
    }
}

#[test]
fn test_fmt_width() {
    for param in ["-w", "--width"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "10"])
            .succeeds()
            .stdout_is("this is a\nfile with\none word\nper line\n");
    }
    new_ucmd!()
        .args(&["one-word-per-line.txt", "-w50", "--width", "10"])
        .succeeds()
        .stdout_is("this is a\nfile with\none word\nper line\n");
}

#[test]
fn test_fmt_width_max_display_width() {
    let input = "aa bb cc dd ee";
    new_ucmd!()
        .args(&["-w", "8"])
        .pipe_in(input)
        .succeeds()
        .stdout_is("aa bb cc\ndd ee\n");
    new_ucmd!()
        .args(&["-w", "7"])
        .pipe_in(input)
        .succeeds()
        .stdout_is("aa\nbb cc\ndd ee\n");
}

/// Regression for <https://github.com/uutils/coreutils/issues/10095>
///
/// GNU `fmt` measures `-w` in UTF-8 bytes (not Unicode display columns).
/// These cases match GNU coreutils 9.11.
#[test]
fn test_fmt_width_multibyte_gnu_compatible() {
    new_ucmd!()
        .args(&["-w", "10"])
        .pipe_in("漢 字 test 日 本 語")
        .succeeds()
        .stdout_is("漢 字\ntest 日\n本 語\n");

    new_ucmd!()
        .args(&["-w", "15"])
        .pipe_in("漢字 test 日本語")
        .succeeds()
        .stdout_is("漢字 test\n日本語\n");
}

#[test]
fn test_fmt_width_invalid() {
    new_ucmd!()
        .args(&["one-word-per-line.txt", "-w", "apple"])
        .fails_with_code(1)
        .no_stdout()
        .stderr_is("fmt: invalid width: 'apple'\n");
    // an invalid width can be successfully overwritten later:
    new_ucmd!()
        .args(&["one-word-per-line.txt", "-w", "apple", "-w10"])
        .succeeds()
        .stdout_is("this is a\nfile with\none word\nper line\n");
}

#[test]
fn test_fmt_positional_width() {
    new_ucmd!()
        .args(&["-10", "one-word-per-line.txt"])
        .succeeds()
        .stdout_is("this is a\nfile with\none word\nper line\n");
}

#[test]
fn test_small_width() {
    for width in ["0", "1", "2", "3"] {
        for param in ["-w", "--width"] {
            new_ucmd!()
                .args(&[param, width, "one-word-per-line.txt"])
                .succeeds()
                .stdout_is("this\nis\na\nfile\nwith\none\nword\nper\nline\n");
        }
    }
}

#[test]
fn test_fmt_width_too_big() {
    for param in ["-w", "--width"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "2501"])
            .fails_with_code(1)
            .stderr_is("fmt: invalid width: '2501': Numerical result out of range\n");
    }
    // However, as a temporary value it is okay:
    new_ucmd!()
        .args(&["one-word-per-line.txt", "-w2501", "--width", "10"])
        .succeeds()
        .stdout_is("this is a\nfile with\none word\nper line\n");
}

#[test]
fn test_fmt_invalid_width() {
    for param in ["-w", "--width"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "invalid"])
            .fails_with_code(1)
            .stderr_contains("invalid width: 'invalid'");
    }
}

#[test]
fn test_fmt_positional_width_not_first() {
    new_ucmd!()
        .args(&["one-word-per-line.txt", "-10"])
        .fails_with_code(1)
        .stderr_contains("fmt: invalid option -- 1; -WIDTH is recognized only when it is the first\noption; use -w N instead");
}

#[test]
fn test_fmt_negative_number_file_after_double_dash() {
    use uutests::util::TestScenario;
    use uutests::util_name;

    let scene = TestScenario::new(util_name!());
    scene.fixtures.write("-9", "hidden text\n");
    scene.fixtures.write("f", "the quick brown fox jumps\n");
    for args in [&["--", "-9", "f"][..], &["-u", "--", "-9", "f"]] {
        scene
            .ucmd()
            .args(args)
            .succeeds()
            .stdout_only("hidden text\nthe quick brown fox jumps\n");
    }
    scene
        .ucmd()
        .args(&["f", "--", "-9"])
        .succeeds()
        .stdout_only("the quick brown fox jumps\nhidden text\n");
    scene
        .ucmd()
        .args(&["--", "-9"])
        .succeeds()
        .stdout_only("hidden text\n");
}

#[test]
fn test_fmt_width_not_valid_number() {
    new_ucmd!()
        .args(&["-25x", "one-word-per-line.txt"])
        .fails_with_code(1)
        .stderr_contains("fmt: invalid width: '25x'");
}

#[ignore = "our 'goal' algorithm is very different from GNU; fix this!"]
#[test]
fn test_fmt_goal() {
    for param in ["-g", "--goal"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "7"])
            .succeeds()
            .stdout_is("this is a\nfile with one\nword per line\n");
    }
    new_ucmd!()
        .args(&["one-word-per-line.txt", "-g40", "-g7"])
        .succeeds()
        .stdout_is("this is a\nfile with one\nword per line\n");
}

#[test]
fn test_fmt_goal_too_big() {
    for param in ["-g", "--goal"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", "--width=75", param, "76"])
            .fails_with_code(1)
            .stderr_is("fmt: GOAL cannot be greater than WIDTH.\n");
    }
}

#[test]
fn test_fmt_goal_bigger_than_default_width_of_75() {
    for param in ["-g", "--goal"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "76"])
            .fails_with_code(1)
            .stderr_is("fmt: GOAL cannot be greater than WIDTH.\n");
    }
}

#[ignore = "our 'goal' algorithm is very different from GNU; fix this!"]
#[test]
fn test_fmt_too_big_goal_sometimes_okay() {
    new_ucmd!()
        .args(&["one-word-per-line.txt", "--width=75", "-g76", "-g10"])
        .succeeds()
        .stdout_is("this is a\nfile with one\nword per line\n");
    new_ucmd!()
        .args(&["one-word-per-line.txt", "-g76", "-g10"])
        .succeeds()
        .stdout_is("this is a\nfile with one\nword per line\n");
}

#[test]
fn test_fmt_goal_too_small_to_check_negative_minlength() {
    for param in ["-g", "--goal"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", "--width=75", param, "10"])
            .succeeds()
            .stdout_is("this is a file with one word per line\n");
    }
}

#[test]
fn test_fmt_non_existent_file() {
    new_ucmd!()
        .args(&["non-existing"])
        .fails_with_code(1)
        .stderr_is("fmt: cannot open 'non-existing' for reading: No such file or directory\n");
}

#[test]
fn test_fmt_invalid_goal() {
    for param in ["-g", "--goal"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "invalid"])
            .fails_with_code(1)
            // GNU complains about "invalid width", which is confusing.
            // We intentionally deviate from GNU, and show a more helpful message:
            .stderr_contains("invalid goal: 'invalid'");
    }
}

#[test]
fn test_fmt_invalid_goal_override() {
    new_ucmd!()
        .args(&["one-word-per-line.txt", "-g", "apple", "-g", "74"])
        .succeeds()
        .stdout_is("this is a file with one word per line\n");
}

#[test]
fn test_fmt_invalid_goal_width_priority() {
    new_ucmd!()
        .args(&["one-word-per-line.txt", "-g", "apple", "-w", "banana"])
        .fails_with_code(1)
        .no_stdout()
        .stderr_is("fmt: invalid width: 'banana'\n");
    new_ucmd!()
        .args(&["one-word-per-line.txt", "-w", "banana", "-g", "apple"])
        .fails_with_code(1)
        .no_stdout()
        .stderr_is("fmt: invalid width: 'banana'\n");
}

#[test]
fn test_fmt_set_goal_not_contain_width() {
    for param in ["-g", "--goal"] {
        new_ucmd!()
            .args(&["one-word-per-line.txt", param, "74"])
            .succeeds()
            .stdout_is("this is a file with one word per line\n");
    }
}

#[test]
fn split_does_not_reflow() {
    for arg in ["-s", "-ss", "--split-only"] {
        new_ucmd!()
            .arg("one-word-per-line.txt")
            .arg(arg)
            .succeeds()
            .stdout_is_fixture("one-word-per-line.txt");
    }
}

#[test]
fn prefix_minus() {
    for prefix_args in [
        vec!["-p-"],
        vec!["-p", "-"],
        vec!["--prefix=-"],
        vec!["--prefix", "-"],
        vec!["--pref=-"],
        vec!["--pref", "-"],
        // Test self-overriding:
        vec!["--prefix==", "--prefix=-"],
    ] {
        new_ucmd!()
            .args(&prefix_args)
            .arg("prefixed-one-word-per-line.txt")
            .succeeds()
            .stdout_is_fixture("prefixed-one-word-per-line_p-.txt");
    }
}

#[test]
fn prefix_equal() {
    for prefix_args in [
        vec!["-p="],
        vec!["-up="],
        vec!["-p", "="],
        vec!["--prefix=="],
        vec!["--prefix", "="],
        vec!["--pref=="],
        vec!["--pref", "="],
        // Test self-overriding:
        vec!["--prefix=-", "--prefix=="],
        vec!["-p-", "-p="],
    ] {
        new_ucmd!()
            .args(&prefix_args)
            .arg("prefixed-one-word-per-line.txt")
            .succeeds()
            .stdout_is_fixture("prefixed-one-word-per-line_p=.txt");
    }
}

#[test]
fn prefix_equal_skip_prefix_equal_two() {
    for prefix_args in [
        vec!["--prefix==", "-P=2"],
        vec!["-p=", "-uP=2"],
        vec!["--prefix==", "-P", "=2"],
        vec!["--prefix==", "--skip-prefix==2"],
        vec!["--prefix==", "--skip-prefix", "=2"],
        vec!["--prefix==", "--skip-pref==2"],
        vec!["--prefix==", "--skip-pref", "=2"],
        // Test self-overriding:
        vec!["--prefix==", "--skip-pref", "asdf", "-P", "=2"],
        vec!["-p=", "-Pignored", "-P=2"],
    ] {
        new_ucmd!()
            .args(&prefix_args)
            .arg("prefixed-one-word-per-line.txt")
            .succeeds()
            .stdout_is_fixture("prefixed-one-word-per-line_p=_P=2.txt");
    }
}

#[test]
fn prefix_with_multiple_equals() {
    for prefix_args in [vec!["-p=="], vec!["-up=="], vec!["--prefix==="]] {
        new_ucmd!()
            .args(&prefix_args)
            .pipe_in("=unchanged\n==one\n==two\n")
            .succeeds()
            .stdout_only("=unchanged\n==one two\n");
    }
}

#[test]
fn prefix_containing_short_option_spelling() {
    for prefix_args in [vec!["-p-p="], vec!["--prefix=-p="], vec!["-qp-p="]] {
        new_ucmd!()
            .args(&prefix_args)
            .pipe_in("-p=one\n-p=two\n")
            .succeeds()
            .stdout_only("-p=one two\n");
    }
}

#[test]
fn prefix_option_spelling_after_terminator_is_a_file() {
    let (at, mut cmd) = uutests::at_and_ucmd!();
    at.write("-p=", "one\ntwo\n");
    cmd.args(&["--", "-p="]).succeeds().stdout_only("one two\n");
}

#[test]
fn prefix_ignores_leading_whitespace_without_exact_prefix() {
    for prefix_args in [vec!["-p", "> "], vec!["--prefix", "> "]] {
        new_ucmd!()
            .args(&prefix_args)
            .pipe_in("  > alpha\n  > beta\n")
            .succeeds()
            .stdout_only("  > alpha beta\n");
    }
}

#[test]
fn exact_prefix_requires_the_prefix_to_start_the_line() {
    for prefix_args in [
        vec!["-x", "-p", "> "],
        vec!["--exact-prefix", "--prefix", "> "],
    ] {
        new_ucmd!()
            .args(&prefix_args)
            .pipe_in("  > alpha\n  > beta\n")
            .succeeds()
            .stdout_only("  > alpha\n  > beta\n");
    }
}

#[test]
fn skip_prefix_ignores_leading_whitespace_without_exact_skip_prefix() {
    for prefix_args in [vec!["-P", "#"], vec!["--skip-prefix", "#"]] {
        new_ucmd!()
            .args(&prefix_args)
            .pipe_in("  # note\n  # more\n")
            .succeeds()
            .stdout_only("  # note\n  # more\n");
    }
}

#[test]
fn exact_skip_prefix_requires_the_prefix_to_start_the_line() {
    for prefix_args in [
        vec!["-X", "-P", "#"],
        vec!["--exact-skip-prefix", "--skip-prefix", "#"],
    ] {
        new_ucmd!()
            .args(&prefix_args)
            .pipe_in("  # note\n  # more\n")
            .succeeds()
            .stdout_only("  # note # more\n");
    }
}

#[test]
fn test_fmt_unicode_whitespace_handling() {
    // Character classification fix: Test that Unicode whitespace characters like non-breaking space
    // are NOT treated as whitespace by fmt, maintaining GNU fmt compatibility.
    // GNU fmt only recognizes ASCII whitespace (space, tab, newline, etc.) and excludes
    // Unicode whitespace characters to ensure consistent formatting behavior.
    // This prevents regression of the character classification fix
    let non_breaking_space = "\u{00A0}"; // U+00A0 NO-BREAK SPACE
    let figure_space = "\u{2007}"; // U+2007 FIGURE SPACE
    let narrow_no_break_space = "\u{202F}"; // U+202F NARROW NO-BREAK SPACE

    // When fmt splits on width=1, these characters should NOT cause line breaks
    // because they should not be considered whitespace
    for (name, char) in [
        ("non-breaking space", non_breaking_space),
        ("figure space", figure_space),
        ("narrow no-break space", narrow_no_break_space),
        ("word joiner", "\u{2060}"),
        ("cyrillic kha", "\u{0445}"),
    ] {
        let input = format!("={char}=");
        let result = new_ucmd!()
            .args(&["-s", "-w1"])
            .pipe_in(input.as_bytes())
            .succeeds();

        // Should be 1 line since the Unicode char is not treated as whitespace
        assert_eq!(
            result.stdout_str().lines().count(),
            1,
            "Failed for {name}: Unicode character should not be treated as whitespace"
        );
    }
}

#[test]
fn test_fmt_knuth_plass_line_breaking() {
    // Line breaking algorithm improvements: Test the enhanced Knuth-Plass optimal line breaking
    // algorithm that better handles sentence boundaries, word positioning constraints,
    // and produces more natural line breaks for complex text formatting.
    // This prevents regression of the line breaking algorithm improvements
    let input = "@command{fmt} prefers breaking lines at the end of a sentence, and tries to\n\
                avoid line breaks after the first word of a sentence or before the last word\n\
                of a sentence.  A @dfn{sentence break} is defined as either the end of a\n\
                paragraph or a word ending in any of @samp{.?!}, followed by two spaces or end\n\
                of line, ignoring any intervening parentheses or quotes.  Like @TeX{},\n\
                @command{fmt} reads entire ''paragraphs'' before choosing line breaks; the\n\
                algorithm is a variant of that given by\n\
                Donald E. Knuth and Michael F. Plass\n\
                in ''Breaking Paragraphs Into Lines'',\n\
                @cite{Software---Practice & Experience}\n\
                @b{11}, 11 (November 1981), 1119--1184.";

    let expected = "@command{fmt} prefers breaking lines at the end of a sentence,\n\
                   and tries to avoid line breaks after the first word of a sentence\n\
                   or before the last word of a sentence.  A @dfn{sentence break}\n\
                   is defined as either the end of a paragraph or a word ending\n\
                   in any of @samp{.?!}, followed by two spaces or end of line,\n\
                   ignoring any intervening parentheses or quotes.  Like @TeX{},\n\
                   @command{fmt} reads entire ''paragraphs'' before choosing line\n\
                   breaks; the algorithm is a variant of that given by Donald\n\
                   E. Knuth and Michael F. Plass in ''Breaking Paragraphs Into\n\
                   Lines'', @cite{Software---Practice & Experience} @b{11}, 11\n\
                   (November 1981), 1119--1184.\n";

    new_ucmd!()
        .args(&["-g", "60", "-w", "72"])
        .pipe_in(input)
        .succeeds()
        .stdout_is(expected);
}

#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(wasi_runner, ignore = "WASI: argv/filenames must be valid UTF-8")]
fn test_fmt_non_utf8_paths() {
    use uutests::at_and_ucmd;

    let (at, mut ucmd) = at_and_ucmd!();
    let filename = std::ffi::OsString::from_vec(vec![0xFF, 0xFE]);

    std::fs::write(at.plus(&filename), b"hello world this is a test").unwrap();

    ucmd.arg(&filename).succeeds();
}

#[test]
fn fmt_reflow_unicode() {
    new_ucmd!()
        .args(&["-w", "4"])
        .pipe_in("漢字漢字 💐 日本語の文字\n")
        .succeeds()
        .stdout_is("漢字漢字\n💐\n日本語の文字\n");
}

#[test]
fn test_fmt_invalid_utf8() {
    // Regression test for handling invalid UTF-8 input (e.g. ISO-8859-1)
    // fmt should not drop lines with invalid UTF-8.
    // \xA0 is non-breaking space in ISO-8859-1, but invalid in UTF-8.
    // We expect GNU-compatible passthrough of the raw byte, not lossy replacement.
    let input = b"=\xA0=";
    new_ucmd!()
        .args(&["-s", "-w1"])
        .pipe_in(input)
        .succeeds()
        .stdout_is_bytes(b"=\xA0=\n");
}

#[test]
fn test_fmt_width_multiplication_overflow() {
    new_ucmd!()
        .args(&["-w", "267672676527678256"])
        .fails_with_code(1)
        .stderr_is("fmt: invalid width: '267672676527678256'\n");
}

#[test]
fn test_fmt_goal_only_defaults_width_to_goal_plus_ten() {
    // GNU defaults the width to goal + 10 when only --goal is given, so `-g G`
    // has to lay a paragraph out exactly as `-w G+10 -g G` does.
    for goal in [5, 10, 20, 30, 50, 65] {
        let widened = new_ucmd!()
            .args(&[
                "one-word-per-line.txt",
                "-w",
                &(goal + 10).to_string(),
                "-g",
                &goal.to_string(),
            ])
            .succeeds()
            .stdout_move_str();

        new_ucmd!()
            .args(&["one-word-per-line.txt", "-g", &goal.to_string()])
            .succeeds()
            .stdout_is(&widened);
    }

    // The whole 37-column paragraph therefore fits on one line at goal 30.
    new_ucmd!()
        .args(&["one-word-per-line.txt", "--goal", "30"])
        .succeeds()
        .stdout_is("this is a file with one word per line\n");
}

#[test]
fn test_fmt_prefix_copies_unprefixed_last_line_without_adding_newline() {
    for (input, expected) in [
        ("x", "x"),
        ("  x", "  x"),
        ("a\nmixed", "a\nmixed"),
        // a line with the prefix is reformatted and gets its newline
        ("# hi", "# hi\n"),
        ("x\n# hi", "x\n# hi\n"),
        ("x\n#", "x\n#\n"),
    ] {
        for args in [vec!["-p", "#"], vec!["-s", "-p", "#"]] {
            new_ucmd!()
                .args(&args)
                .pipe_in(input)
                .succeeds()
                .stdout_only(expected);
        }
    }
}

#[test]
fn test_fmt_prefix_unprefixed_last_line_runs_into_next_file() {
    use uutests::at_and_ucmd;

    let (at, mut ucmd) = at_and_ucmd!();
    at.write("f1", "a\nmixed");
    at.write("f2", "hello\n");
    ucmd.args(&["-p", "#", "f1", "f2"])
        .succeeds()
        .stdout_only("a\nmixedhello\n");
}

#[test]
fn test_fmt_anti_prefix_keeps_missing_final_newline() {
    for (args, input, expected) in [
        (vec!["-P", "#"], "a\n#x", "a\n#x"),
        (vec!["-P", "#"], "a\n#x\n", "a\n#x\n"),
        (vec!["-P", "#"], "#x\na\n", "#x\na\n"),
        (vec!["--skip-prefix", "#"], "a\n  #x", "a\n  #x"),
        (vec!["-p", ">", "-P", ">#"], "> a\n>#x", "> a\n>#x"),
        // a line that is reformatted gets its newline
        (vec!["-P", "#"], "a\nb", "a b\n"),
    ] {
        new_ucmd!()
            .args(&args)
            .pipe_in(input)
            .succeeds()
            .stdout_only(expected);
    }
}
