//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (words) ints

use crate::common::util::*;

fn test_helper(file_name: &str, possible_args: &[&str]) {
    for args in possible_args {
        new_ucmd!()
            .arg(format!("{}.txt", file_name))
            .args(&args.split_whitespace().collect::<Vec<&str>>())
            .succeeds()
            .stdout_is_fixture(format!("{}.expected", file_name));

        new_ucmd!()
            .arg(format!("{}.txt", file_name))
            .arg("--debug")
            .args(&args.split_whitespace().collect::<Vec<&str>>())
            .succeeds()
            .stdout_is_fixture(format!("{}.expected.debug", file_name));
    }
}

#[test]
fn test_buffer_sizes() {
    let buffer_sizes = ["0", "50K", "50k", "1M", "100M"];
    for buffer_size in &buffer_sizes {
        new_ucmd!()
            .arg("-n")
            .arg("-S")
            .arg(buffer_size)
            .arg("ext_sort.txt")
            .succeeds()
            .stdout_is_fixture("ext_sort.expected");

        #[cfg(not(target_pointer_width = "32"))]
        {
            let buffer_sizes = ["1000G", "10T"];
            for buffer_size in &buffer_sizes {
                new_ucmd!()
                    .arg("-n")
                    .arg("-S")
                    .arg(buffer_size)
                    .arg("ext_sort.txt")
                    .succeeds()
                    .stdout_is_fixture("ext_sort.expected");
            }
        }
    }
}

#[test]
fn test_invalid_buffer_size() {
    let buffer_sizes = ["asd", "100f"];
    for invalid_buffer_size in &buffer_sizes {
        new_ucmd!()
            .arg("-S")
            .arg(invalid_buffer_size)
            .fails()
            .code_is(2)
            .stderr_only(format!(
                "sort: invalid --buffer-size argument '{}'",
                invalid_buffer_size
            ));
    }
    #[cfg(not(target_pointer_width = "128"))]
    new_ucmd!()
        .arg("-n")
        .arg("-S")
        .arg("1Y")
        .arg("ext_sort.txt")
        .fails()
        .code_is(2)
        .stderr_only("sort: --buffer-size argument '1Y' too large");

    #[cfg(target_pointer_width = "32")]
    {
        let buffer_sizes = ["1000G", "10T"];
        for buffer_size in &buffer_sizes {
            new_ucmd!()
                .arg("-n")
                .arg("-S")
                .arg(buffer_size)
                .arg("ext_sort.txt")
                .fails()
                .code_is(2)
                .stderr_only(format!(
                    "sort: --buffer-size argument '{}' too large",
                    buffer_size
                ));
        }
    }
}

#[test]
fn test_ext_sort_stable() {
    new_ucmd!()
        .arg("-n")
        .arg("--stable")
        .arg("-S")
        .arg("0M")
        .arg("ext_stable.txt")
        .succeeds()
        .stdout_only_fixture("ext_stable.expected");
}

#[test]
fn test_ext_sort_zero_terminated() {
    new_ucmd!()
        .arg("-z")
        .arg("-S")
        .arg("10K")
        .arg("zero-terminated.txt")
        .succeeds()
        .stdout_is_fixture("zero-terminated.expected");
}

#[test]
fn test_months_whitespace() {
    test_helper("months-whitespace", &["-M", "--month-sort", "--sort=month"]);
}

#[test]
fn test_version_empty_lines() {
    new_ucmd!()
        .arg("-V")
        .arg("version-empty-lines.txt")
        .succeeds()
        .stdout_is("\n\n\n\n\n\n\n1.2.3-alpha\n1.2.3-alpha2\n\t\t\t1.12.4\n11.2.3\n");
}

#[test]
fn test_human_numeric_whitespace() {
    test_helper(
        "human-numeric-whitespace",
        &["-h", "--human-numeric-sort", "--sort=human-numeric"],
    );
}

// This tests where serde often fails when reading back JSON
// if it finds a null value
#[test]
fn test_ext_sort_as64_bailout() {
    new_ucmd!()
        .arg("-g")
        .arg("-S 5K")
        .arg("multiple_decimals_general.txt")
        .succeeds()
        .stdout_is_fixture("multiple_decimals_general.expected");
}

#[test]
fn test_multiple_decimals_general() {
    test_helper(
        "multiple_decimals_general",
        &["-g", "--general-numeric-sort", "--sort=general-numeric"],
    )
}

#[test]
fn test_multiple_decimals_numeric() {
    test_helper(
        "multiple_decimals_numeric",
        &["-n", "--numeric-sort", "--sort=numeric"],
    )
}

#[test]
fn test_numeric_with_trailing_invalid_chars() {
    test_helper(
        "numeric_trailing_chars",
        &["-n", "--numeric-sort", "--sort=numeric"],
    )
}

#[test]
fn test_check_zero_terminated_failure() {
    new_ucmd!()
        .arg("-z")
        .arg("-c")
        .arg("zero-terminated.txt")
        .fails()
        .stdout_is("sort: zero-terminated.txt:2: disorder: ../../fixtures/du\n");
}

#[test]
fn test_check_zero_terminated_success() {
    new_ucmd!()
        .arg("-z")
        .arg("-c")
        .arg("zero-terminated.expected")
        .succeeds();
}

#[test]
fn test_random_shuffle_len() {
    // check whether output is the same length as the input
    const FILE: &str = "default_unsorted_ints.expected";
    let (at, _ucmd) = at_and_ucmd!();
    let result = new_ucmd!().arg("-R").arg(FILE).run().stdout_move_str();
    let expected = at.read(FILE);

    assert_ne!(result, expected);
    assert_eq!(result.len(), expected.len());
}

#[test]
fn test_random_shuffle_contains_all_lines() {
    // check whether lines of input are all in output
    const FILE: &str = "default_unsorted_ints.expected";
    let (at, _ucmd) = at_and_ucmd!();
    let result = new_ucmd!().arg("-R").arg(FILE).run().stdout_move_str();
    let expected = at.read(FILE);
    let result_sorted = new_ucmd!().pipe_in(result.clone()).run().stdout_move_str();

    assert_ne!(result, expected);
    assert_eq!(result_sorted, expected);
}

#[test]
fn test_random_shuffle_two_runs_not_the_same() {
    // check to verify that two random shuffles are not equal; this has the
    // potential to fail in the very unlikely event that the random order is the same
    // as the starting order, or if both random sorts end up having the same order.
    const FILE: &str = "default_unsorted_ints.expected";
    let (at, _ucmd) = at_and_ucmd!();
    let result = new_ucmd!().arg("-R").arg(FILE).run().stdout_move_str();
    let expected = at.read(FILE);
    let unexpected = new_ucmd!().arg("-R").arg(FILE).run().stdout_move_str();

    assert_ne!(result, expected);
    assert_ne!(result, unexpected);
}

#[test]
fn test_random_shuffle_contains_two_runs_not_the_same() {
    // check to verify that two random shuffles are not equal; this has the
    // potential to fail in the unlikely event that random order is the same
    // as the starting order, or if both random sorts end up having the same order.
    const FILE: &str = "default_unsorted_ints.expected";
    let (at, _ucmd) = at_and_ucmd!();
    let result = new_ucmd!().arg("-R").arg(FILE).run().stdout_move_str();
    let expected = at.read(FILE);
    let unexpected = new_ucmd!().arg("-R").arg(FILE).run().stdout_move_str();

    assert_ne!(result, expected);
    assert_ne!(result, unexpected);
}

#[test]
fn test_numeric_floats_and_ints() {
    test_helper(
        "numeric_floats_and_ints",
        &["-n", "--numeric-sort", "--sort=numeric"],
    );
}

#[test]
fn test_numeric_floats() {
    test_helper(
        "numeric_floats",
        &["-n", "--numeric-sort", "--sort=numeric"],
    );
}

#[test]
fn test_numeric_floats_with_nan() {
    test_helper(
        "numeric_floats_with_nan",
        &["-n", "--numeric-sort", "--sort=numeric"],
    );
}

#[test]
fn test_numeric_unfixed_floats() {
    test_helper(
        "numeric_unfixed_floats",
        &["-n", "--numeric-sort", "--sort=numeric"],
    );
}

#[test]
fn test_numeric_fixed_floats() {
    test_helper(
        "numeric_fixed_floats",
        &["-n", "--numeric-sort", "--sort=numeric"],
    );
}

#[test]
fn test_numeric_unsorted_ints() {
    test_helper(
        "numeric_unsorted_ints",
        &["-n", "--numeric-sort", "--sort=numeric"],
    );
}

#[test]
fn test_human_block_sizes() {
    test_helper(
        "human_block_sizes",
        &["-h", "--human-numeric-sort", "--sort=human-numeric"],
    );
}

#[test]
fn test_month_default() {
    test_helper("month_default", &["-M", "--month-sort", "--sort=month"]);
}

#[test]
fn test_month_stable() {
    test_helper("month_stable", &["-Ms"]);
}

#[test]
fn test_default_unsorted_ints() {
    test_helper("default_unsorted_ints", &[""]);
}

#[test]
fn test_numeric_unique_ints() {
    test_helper("numeric_unsorted_ints_unique", &["-nu"]);
}

#[test]
fn test_version() {
    test_helper("version", &["-V"]);
}

#[test]
fn test_ignore_case() {
    test_helper("ignore_case", &["-f"]);
}

#[test]
fn test_dictionary_order() {
    test_helper("dictionary_order", &["-d"]);
}

#[test]
fn test_dictionary_order2() {
    for non_dictionary_order2_param in &["-d"] {
        new_ucmd!()
            .pipe_in("a👦🏻aa	b\naaaa	b") // spell-checker:disable-line
            .arg(non_dictionary_order2_param) // spell-checker:disable-line
            .succeeds()
            .stdout_only("a👦🏻aa	b\naaaa	b\n"); // spell-checker:disable-line
    }
}

#[test]
fn test_non_printing_chars() {
    for non_printing_chars_param in &["-i"] {
        new_ucmd!()
            .pipe_in("a👦🏻aa\naaaa") // spell-checker:disable-line
            .arg(non_printing_chars_param) // spell-checker:disable-line
            .succeeds()
            .stdout_only("a👦🏻aa\naaaa\n"); // spell-checker:disable-line
    }
}

#[test]
fn test_exponents_positive_general_fixed() {
    test_helper("exponents_general", &["-g"]);
}

#[test]
fn test_exponents_positive_numeric() {
    test_helper(
        "exponents-positive-numeric",
        &["-n", "--numeric-sort", "--sort=numeric"],
    );
}

#[test]
fn test_months_dedup() {
    test_helper("months-dedup", &["-Mu"]);
}

#[test]
fn test_mixed_floats_ints_chars_numeric() {
    test_helper(
        "mixed_floats_ints_chars_numeric",
        &["-n", "--numeric-sort", "--sort=numeric"],
    );
}

#[test]
fn test_mixed_floats_ints_chars_numeric_unique() {
    test_helper("mixed_floats_ints_chars_numeric_unique", &["-nu"]);
}

#[test]
fn test_words_unique() {
    test_helper("words_unique", &["-u"]);
}

#[test]
fn test_numeric_unique() {
    test_helper("numeric_unique", &["-nu"]);
}

#[test]
fn test_mixed_floats_ints_chars_numeric_reverse() {
    test_helper("mixed_floats_ints_chars_numeric_unique_reverse", &["-nur"]);
}

#[test]
fn test_mixed_floats_ints_chars_numeric_stable() {
    test_helper("mixed_floats_ints_chars_numeric_stable", &["-ns"]);
}

#[test]
fn test_numeric_floats_and_ints2() {
    for numeric_sort_param in &["-n", "--numeric-sort"] {
        let input = "1.444\n8.013\n1\n-8\n1.04\n-1";
        new_ucmd!()
            .arg(numeric_sort_param)
            .pipe_in(input)
            .succeeds()
            .stdout_only("-8\n-1\n1\n1.04\n1.444\n8.013\n");
    }
}

#[test]
fn test_numeric_floats2() {
    for numeric_sort_param in &["-n", "--numeric-sort"] {
        let input = "1.444\n8.013\n1.58590\n-8.90880\n1.040000000\n-.05";
        new_ucmd!()
            .arg(numeric_sort_param)
            .pipe_in(input)
            .succeeds()
            .stdout_only("-8.90880\n-.05\n1.040000000\n1.444\n1.58590\n8.013\n");
    }
}

#[test]
fn test_numeric_floats_with_nan2() {
    test_helper(
        "numeric-floats-with-nan2",
        &["-n", "--numeric-sort", "--sort=numeric"],
    );
}

#[test]
fn test_human_block_sizes2() {
    for human_numeric_sort_param in &["-h", "--human-numeric-sort", "--sort=human-numeric"] {
        let input = "8981K\n909991M\n-8T\n21G\n0.8M";
        new_ucmd!()
            .arg(human_numeric_sort_param)
            .pipe_in(input)
            .succeeds()
            .stdout_only("-8T\n0.8M\n8981K\n21G\n909991M\n");
    }
}

#[test]
fn test_month_default2() {
    for month_sort_param in &["-M", "--month-sort", "--sort=month"] {
        let input = "JAn\nMAY\n000may\nJun\nFeb";
        new_ucmd!()
            .arg(month_sort_param)
            .pipe_in(input)
            .succeeds()
            .stdout_only("000may\nJAn\nFeb\nMAY\nJun\n");
    }
}

#[test]
fn test_default_unsorted_ints2() {
    let input = "9\n1909888\n000\n1\n2";
    new_ucmd!()
        .pipe_in(input)
        .succeeds()
        .stdout_only("000\n1\n1909888\n2\n9\n");
}

#[test]
fn test_numeric_unique_ints2() {
    for numeric_unique_sort_param in &["-nu"] {
        let input = "9\n9\n8\n1\n";
        new_ucmd!()
            .arg(numeric_unique_sort_param)
            .pipe_in(input)
            .succeeds()
            .stdout_only("1\n8\n9\n");
    }
}

#[test]
fn test_keys_open_ended() {
    test_helper("keys_open_ended", &["-k 2.3"]);
}

#[test]
fn test_keys_closed_range() {
    test_helper("keys_closed_range", &["-k 2.2,2.2"]);
}

#[test]
fn test_keys_multiple_ranges() {
    test_helper("keys_multiple_ranges", &["-k 2,2 -k 3,3"]);
}

#[test]
fn test_keys_no_field_match() {
    test_helper("keys_no_field_match", &["-k 4,4"]);
}

#[test]
fn test_keys_no_char_match() {
    test_helper("keys_no_char_match", &["-k 1.2"]);
}

#[test]
fn test_keys_custom_separator() {
    test_helper("keys_custom_separator", &["-k 2.2,2.2 -t x"]);
}

#[test]
fn test_keys_invalid_field() {
    new_ucmd!()
        .args(&["-k", "1."])
        .fails()
        .stderr_only("sort: failed to parse key `1.`: failed to parse character index ``: cannot parse integer from empty string");
}

#[test]
fn test_keys_invalid_field_option() {
    new_ucmd!()
        .args(&["-k", "1.1x"])
        .fails()
        .stderr_only("sort: failed to parse key `1.1x`: invalid option: `x`");
}

#[test]
fn test_keys_invalid_field_zero() {
    new_ucmd!()
        .args(&["-k", "0.1"])
        .fails()
        .stderr_only("sort: failed to parse key `0.1`: field index can not be 0");
}

#[test]
fn test_keys_invalid_char_zero() {
    new_ucmd!()
        .args(&["-k", "1.0"])
        .fails()
        .stderr_only("sort: failed to parse key `1.0`: invalid character index 0 for the start position of a field");
}

#[test]
fn test_keys_with_options() {
    let input = "aa 3 cc\ndd 1 ff\ngg 2 cc\n";
    for param in &[
        &["-k", "2,2n"][..],
        &["-k", "2n,2"][..],
        &["-k", "2,2", "-n"][..],
    ] {
        new_ucmd!()
            .args(param)
            .pipe_in(input)
            .succeeds()
            .stdout_only("dd 1 ff\ngg 2 cc\naa 3 cc\n");
    }
}

#[test]
fn test_keys_with_options_blanks_start() {
    let input = "aa   3 cc\ndd  1 ff\ngg         2 cc\n";
    for param in &[&["-k", "2b,2"][..], &["-k", "2,2", "-b"][..]] {
        new_ucmd!()
            .args(param)
            .pipe_in(input)
            .succeeds()
            .stdout_only("dd  1 ff\ngg         2 cc\naa   3 cc\n");
    }
}

#[test]
fn test_keys_blanks_with_char_idx() {
    test_helper("keys_blanks", &["-k 1.2b"])
}

#[test]
fn test_keys_with_options_blanks_end() {
    let input = "a  b
a b
a   b
";
    new_ucmd!()
        .args(&["-k", "1,2.1b", "-s"])
        .pipe_in(input)
        .succeeds()
        .stdout_only(
            "a   b
a  b
a b
",
        );
}

#[test]
fn test_keys_stable() {
    let input = "a  b
a b
a   b
";
    new_ucmd!()
        .args(&["-k", "1,2.1", "-s"])
        .pipe_in(input)
        .succeeds()
        .stdout_only(
            "a  b
a b
a   b
",
        );
}

#[test]
fn test_keys_empty_match() {
    let input = "a a a a
aaaa
";
    new_ucmd!()
        .args(&["-k", "1,1", "-t", "a"])
        .pipe_in(input)
        .succeeds()
        .stdout_only(input);
}

#[test]
fn test_keys_negative_size_match() {
    // If the end of a field is before its start, we should not crash.
    // Debug output should report "no match for key" at the start position (i.e. the later position).
    test_helper("keys_negative_size", &["-k 3,1"]);
}

#[test]
fn test_keys_ignore_flag() {
    test_helper("keys_ignore_flag", &["-k 1n -b"])
}

#[test]
fn test_does_not_inherit_key_settings() {
    let input = " 1
2
   10
";
    new_ucmd!()
        .args(&["-k", "1b", "-n"])
        .pipe_in(input)
        .succeeds()
        .stdout_only(
            " 1
   10
2
",
        );
}

#[test]
fn test_inherits_key_settings() {
    let input = " 1
2
   10
";
    new_ucmd!()
        .args(&["-k", "1", "-n"])
        .pipe_in(input)
        .succeeds()
        .stdout_only(
            " 1
2
   10
",
        );
}

#[test]
fn test_zero_terminated() {
    test_helper("zero-terminated", &["-z"]);
}

#[test]
fn test_multiple_files() {
    new_ucmd!()
        .arg("-n")
        .arg("multiple_files1.txt")
        .arg("multiple_files2.txt")
        .succeeds()
        .stdout_only_fixture("multiple_files.expected");
}

#[test]
fn test_merge_interleaved() {
    new_ucmd!()
        .arg("-m")
        .arg("merge_ints_interleaved_1.txt")
        .arg("merge_ints_interleaved_2.txt")
        .arg("merge_ints_interleaved_3.txt")
        .succeeds()
        .stdout_only_fixture("merge_ints_interleaved.expected");
}

#[test]
fn test_merge_unique() {
    new_ucmd!()
        .arg("-m")
        .arg("--unique")
        .arg("merge_ints_interleaved_1.txt")
        .arg("merge_ints_interleaved_2.txt")
        .arg("merge_ints_interleaved_3.txt")
        .arg("merge_ints_interleaved_3.txt")
        .arg("merge_ints_interleaved_2.txt")
        .arg("merge_ints_interleaved_1.txt")
        .succeeds()
        .stdout_only_fixture("merge_ints_interleaved.expected");
}

#[test]
fn test_merge_stable() {
    new_ucmd!()
        .arg("-m")
        .arg("--stable")
        .arg("-n")
        .arg("merge_stable_1.txt")
        .arg("merge_stable_2.txt")
        .succeeds()
        .stdout_only_fixture("merge_stable.expected");
}

#[test]
fn test_merge_reversed() {
    new_ucmd!()
        .arg("-m")
        .arg("--reverse")
        .arg("merge_ints_reversed_1.txt")
        .arg("merge_ints_reversed_2.txt")
        .arg("merge_ints_reversed_3.txt")
        .succeeds()
        .stdout_only_fixture("merge_ints_reversed.expected");
}

#[test]
fn test_pipe() {
    // TODO: issue 1608 reports a panic when we attempt to read from stdin,
    // which was closed by the other side of the pipe. This test does not
    // protect against regressions in that case; we should add one at some
    // point.
    new_ucmd!()
        .pipe_in("one\ntwo\nfour")
        .succeeds()
        .stdout_is("four\none\ntwo\n")
        .stderr_is("");
}

#[test]
fn test_check() {
    new_ucmd!()
        .arg("-c")
        .arg("check_fail.txt")
        .fails()
        .stdout_is("sort: check_fail.txt:6: disorder: 5\n");

    new_ucmd!()
        .arg("-c")
        .arg("multiple_files.expected")
        .succeeds()
        .stdout_is("");
}

#[test]
fn test_check_silent() {
    new_ucmd!()
        .arg("-C")
        .arg("check_fail.txt")
        .fails()
        .stdout_is("");
}

#[test]
fn test_dictionary_and_nonprinting_conflicts() {
    let conflicting_args = ["n", "h", "g", "M"];
    for restricted_arg in &["d", "i"] {
        for conflicting_arg in &conflicting_args {
            new_ucmd!()
                .arg(&format!("-{}{}", restricted_arg, conflicting_arg))
                .fails();
        }
        for conflicting_arg in &conflicting_args {
            new_ucmd!()
                .args(&[
                    format!("-{}", restricted_arg).as_str(),
                    "-k",
                    &format!("1,1{}", conflicting_arg),
                ])
                .succeeds();
        }
        for conflicting_arg in &conflicting_args {
            new_ucmd!()
                .args(&["-k", &format!("1{},1{}", restricted_arg, conflicting_arg)])
                .fails();
        }
    }
}

#[test]
fn test_trailing_separator() {
    new_ucmd!()
        .args(&["-t", "x", "-k", "1,1"])
        .pipe_in("aax\naaa\n")
        .succeeds()
        .stdout_is("aax\naaa\n");
}

#[test]
fn test_nonexistent_file() {
    new_ucmd!()
        .arg("nonexistent.txt")
        .fails()
        .status_code(2)
        .stderr_only(
            #[cfg(not(windows))]
            "sort: cannot read: \"nonexistent.txt\": No such file or directory (os error 2)",
            #[cfg(windows)]
            "sort: cannot read: \"nonexistent.txt\": The system cannot find the file specified. (os error 2)",
        );
}

#[test]
fn test_blanks() {
    test_helper("blanks", &["-b", "--ignore-blanks"]);
}

#[test]
fn sort_multiple() {
    new_ucmd!()
        .args(&["no_trailing_newline1.txt", "no_trailing_newline2.txt"])
        .succeeds()
        .stdout_is("a\nb\nb\n");
}

#[test]
fn sort_empty_chunk() {
    new_ucmd!()
        .args(&["-S", "40B"])
        .pipe_in("a\na\n")
        .succeeds()
        .stdout_is("a\na\n");
}

#[test]
#[cfg(target_os = "linux")]
fn test_compress() {
    new_ucmd!()
        .args(&[
            "ext_sort.txt",
            "-n",
            "--compress-program",
            "gzip",
            "-S",
            "10",
        ])
        .succeeds()
        .stdout_only_fixture("ext_sort.expected");
}

#[test]
fn test_compress_fail() {
    new_ucmd!()
        .args(&[
            "ext_sort.txt",
            "-n",
            "--compress-program",
            "nonexistent-program",
            "-S",
            "10",
        ])
        .fails()
        .stderr_only("sort: couldn't execute compress program: errno 2");
}

#[test]
fn test_merge_batches() {
    new_ucmd!()
        .args(&[
            "ext_sort.txt",
            "-n",
            "-S",
            "150B",
        ])
        .succeeds()
        .stdout_only_fixture("ext_sort.expected");
}
