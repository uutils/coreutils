// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) unwritable
use std::fmt::Write;

use uutests::at_and_ucmd;
use uutests::new_ucmd;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_output_is_random_permutation() {
    let input_seq = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let input = input_seq
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>()
        .join("\n");

    let result = new_ucmd!().pipe_in(input.as_bytes()).succeeds();
    result.no_stderr();

    let mut result_seq: Vec<i32> = result
        .stdout_str()
        .split('\n')
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    result_seq.sort_unstable();
    assert_ne!(result.stdout_str(), input, "Output is not randomized");
    assert_eq!(result_seq, input_seq, "Output is not a permutation");
}

#[test]
fn test_explicit_stdin_file() {
    let input_seq = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let input = input_seq
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>()
        .join("\n");

    let result = new_ucmd!().arg("-").pipe_in(input.as_bytes()).succeeds();
    result.no_stderr();

    let mut result_seq: Vec<i32> = result
        .stdout_str()
        .split('\n')
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    result_seq.sort_unstable();
    assert_eq!(result_seq, input_seq, "Output is not a permutation");
}

#[test]
fn test_zero_termination() {
    let input_seq = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let result = new_ucmd!().arg("-z").arg("-i1-10").succeeds();
    result.no_stderr();

    let mut result_seq: Vec<i32> = result
        .stdout_str()
        .split('\0')
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    result_seq.sort_unstable();
    assert_eq!(result_seq, input_seq, "Output is not a permutation");
}

#[test]
fn test_zero_termination_multi() {
    let input_seq = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let result = new_ucmd!().arg("-z").arg("-z").arg("-i1-10").succeeds();
    result.no_stderr();

    let mut result_seq: Vec<i32> = result
        .stdout_str()
        .split('\0')
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    result_seq.sort_unstable();
    assert_eq!(result_seq, input_seq, "Output is not a permutation");
}

#[test]
fn test_very_large_range() {
    let num_samples = 256;
    let result = new_ucmd!()
        .arg("-n")
        .arg(num_samples.to_string())
        .arg("-i1-100000000000")
        .succeeds();
    result.no_stderr();

    let result_seq: Vec<u64> = result
        .stdout_str()
        .split('\n')
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    assert_eq!(result_seq.len(), num_samples, "Miscounted output length!");
    assert!(
        result_seq.iter().all(|x| (0..=100_000_000_000).contains(x)),
        "Output includes element not from range: {}",
        result.stdout_str()
    );
}

#[test]
fn test_very_large_range_offset() {
    let num_samples = 10;
    let result = new_ucmd!()
        .arg("-n")
        .arg(num_samples.to_string())
        .arg("-i1234567890-2147483647")
        .succeeds();
    result.no_stderr();

    let result_seq: Vec<isize> = result
        .stdout_str()
        .split('\n')
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    assert_eq!(result_seq.len(), num_samples, "Miscounted output length!");
    assert!(
        result_seq
            .iter()
            .all(|x| (1_234_567_890..=2_147_483_647).contains(x)),
        "Output includes element not from range: {}",
        result.stdout_str()
    );
}

#[test]
fn test_range_repeat_no_overflow_1_max() {
    let upper_bound = usize::MAX;
    let result = new_ucmd!()
        .arg("-rn1")
        .arg(format!("-i1-{upper_bound}"))
        .succeeds();
    result.no_stderr();

    let result_seq: Vec<usize> = result
        .stdout_str()
        .split('\n')
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    assert_eq!(result_seq.len(), 1, "Miscounted output length!");
}

#[test]
fn test_range_repeat_no_overflow_0_max_minus_1() {
    let upper_bound = usize::MAX - 1;
    let result = new_ucmd!()
        .arg("-rn1")
        .arg(format!("-i0-{upper_bound}"))
        .succeeds();
    result.no_stderr();

    let result_seq: Vec<usize> = result
        .stdout_str()
        .split('\n')
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    assert_eq!(result_seq.len(), 1, "Miscounted output length!");
}

#[test]
fn test_range_permute_no_overflow_1_max() {
    let upper_bound = usize::MAX;
    let result = new_ucmd!()
        .arg("-n1")
        .arg(format!("-i1-{upper_bound}"))
        .succeeds();
    result.no_stderr();

    let result_seq: Vec<usize> = result
        .stdout_str()
        .split('\n')
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    assert_eq!(result_seq.len(), 1, "Miscounted output length!");
}

#[test]
fn test_range_permute_no_overflow_0_max_minus_1() {
    let upper_bound = usize::MAX - 1;
    let result = new_ucmd!()
        .arg("-n1")
        .arg(format!("-i0-{upper_bound}"))
        .succeeds();
    result.no_stderr();

    let result_seq: Vec<usize> = result
        .stdout_str()
        .split('\n')
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    assert_eq!(result_seq.len(), 1, "Miscounted output length!");
}

#[test]
fn test_range_permute_no_overflow_0_max() {
    // NOTE: This is different from GNU shuf!
    // GNU shuf accepts -i0-MAX-1 and -i1-MAX, but not -i0-MAX.
    // This feels like a bug in GNU shuf.
    let upper_bound = usize::MAX;
    let result = new_ucmd!()
        .arg("-n1")
        .arg(format!("-i0-{upper_bound}"))
        .succeeds();
    result.no_stderr();

    let result_seq: Vec<usize> = result
        .stdout_str()
        .split('\n')
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    assert_eq!(result_seq.len(), 1, "Miscounted output length!");
}

#[test]
fn test_very_high_range_full() {
    let input_seq = vec![
        2_147_483_641,
        2_147_483_642,
        2_147_483_643,
        2_147_483_644,
        2_147_483_645,
        2_147_483_646,
        2_147_483_647,
    ];
    let result = new_ucmd!().arg("-i2147483641-2147483647").succeeds();
    result.no_stderr();

    let mut result_seq: Vec<isize> = result
        .stdout_str()
        .split('\n')
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    result_seq.sort_unstable();
    assert_eq!(result_seq, input_seq, "Output is not a permutation");
}

#[test]
fn test_range_repeat() {
    let num_samples = 500;
    let result = new_ucmd!()
        .arg("-r")
        .arg("-n")
        .arg(num_samples.to_string())
        .arg("-i12-34")
        .succeeds();
    result.no_stderr();

    let result_seq: Vec<isize> = result
        .stdout_str()
        .split('\n')
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    assert_eq!(result_seq.len(), num_samples, "Miscounted output length!");
    assert!(
        result_seq.iter().all(|x| (12..=34).contains(x)),
        "Output includes element not from range: {}",
        result.stdout_str()
    );
}

#[test]
fn test_empty_input() {
    let result = new_ucmd!().pipe_in(vec![]).succeeds();
    result.no_stderr();
    result.no_stdout();
}

#[test]
fn test_echo() {
    let input_seq = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let result = new_ucmd!()
        .arg("-e")
        .args(
            &input_seq
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<String>>(),
        )
        .succeeds();
    result.no_stderr();

    let mut result_seq: Vec<i32> = result
        .stdout_str()
        .split('\n')
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    result_seq.sort_unstable();
    assert_eq!(result_seq, input_seq, "Output is not a permutation");
}

#[test]
fn test_echo_multi() {
    let result = new_ucmd!()
        .arg("-e")
        .arg("a")
        .arg("b")
        .arg("-e")
        .arg("c")
        .succeeds();
    result.no_stderr();

    let mut result_seq: Vec<String> = result
        .stdout_str()
        .split('\n')
        .filter(|x| !x.is_empty())
        .map(Into::into)
        .collect();
    result_seq.sort_unstable();
    assert_eq!(result_seq, ["a", "b", "c"], "Output is not a permutation");
}

#[test]
fn test_echo_postfix() {
    let result = new_ucmd!().arg("a").arg("b").arg("c").arg("-e").succeeds();
    result.no_stderr();

    let mut result_seq: Vec<String> = result
        .stdout_str()
        .split('\n')
        .filter(|x| !x.is_empty())
        .map(Into::into)
        .collect();
    result_seq.sort_unstable();
    assert_eq!(result_seq, ["a", "b", "c"], "Output is not a permutation");
}

#[test]
fn test_echo_short_collapsed_zero() {
    let result = new_ucmd!().arg("-ez").arg("a").arg("b").arg("c").succeeds();
    result.no_stderr();

    let mut result_seq: Vec<String> = result
        .stdout_str()
        .split('\0')
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    result_seq.sort_unstable();
    assert_eq!(result_seq, ["a", "b", "c"], "Output is not a permutation");
}

#[test]
fn test_echo_separators_in_arguments() {
    // We used to split arguments themselves on newlines, but this was wrong.
    // shuf should behave as though it's shuffling two arguments and therefore
    // output all of them.
    // (Note that arguments can't contain null bytes so we don't need to test that.)
    let result = new_ucmd!()
        .arg("-e")
        .arg("-n2")
        .arg("a\nb")
        .arg("c\nd")
        .succeeds();
    result.no_stderr();
    assert_eq!(result.stdout_str().len(), 8, "Incorrect output length");
}

#[cfg(unix)]
#[test]
fn test_echo_invalid_unicode_in_arguments() {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    let result = new_ucmd!()
        .arg("-e")
        .arg(OsStr::from_bytes(b"a\xFFb"))
        .arg("ok")
        .succeeds();
    result.no_stderr();
    assert!(result.stdout().contains(&b'\xFF'));
}

#[cfg(any(unix, target_os = "wasi"))]
#[cfg(not(target_os = "macos"))]
#[test]
fn test_invalid_unicode_in_filename() {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    let (at, mut ucmd) = at_and_ucmd!();
    let filename = OsStr::from_bytes(b"a\xFFb");
    at.append(filename, "foo\n");

    let result = ucmd.arg(filename).succeeds();
    result.no_stderr();
    assert_eq!(result.stdout(), b"foo\n");
}

#[test]
fn test_head_count() {
    let repeat_limit = 5;
    let input_seq = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let input = input_seq
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>()
        .join("\n");

    let result = new_ucmd!()
        .args(&["-n", &repeat_limit.to_string()])
        .pipe_in(input.as_bytes())
        .succeeds();
    result.no_stderr();

    let mut result_seq: Vec<i32> = result
        .stdout_str()
        .split('\n')
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    result_seq.sort_unstable();
    assert_eq!(result_seq.len(), repeat_limit, "Output is not limited");
    assert!(
        result_seq.iter().all(|x| input_seq.contains(x)),
        "Output includes element not from input: {}",
        result.stdout_str()
    );
}

#[test]
fn test_zero_head_count_pipe() {
    let result = new_ucmd!().arg("-n0").pipe_in(vec![]).succeeds();
    // Output must be completely empty, not even a newline!
    result.no_output();
}

#[test]
fn test_zero_head_count_pipe_explicit() {
    let result = new_ucmd!().arg("-n0").arg("-").pipe_in(vec![]).succeeds();
    result.no_output();
}

#[test]
fn test_zero_head_count_file_unreadable() {
    new_ucmd!()
        .arg("-n0")
        .arg("/invalid/unreadable")
        .pipe_in(vec![])
        .succeeds()
        .no_output();
}

#[test]
fn test_zero_head_count_file_touch_output_negative() {
    new_ucmd!()
        .arg("-n0")
        .arg("-o")
        .arg("/invalid/unwritable")
        .pipe_in(vec![])
        .fails()
        .stderr_contains("failed to open '/invalid/unwritable' for writing:");
}

#[test]
fn test_zero_head_count_file_touch_output_positive_new() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.args(&["-n0", "-o", "file"]).succeeds().no_output();
    assert_eq!(
        at.read_bytes("file"),
        Vec::new(),
        "Output file must exist and be completely empty"
    );
}

#[test]
fn test_zero_head_count_file_touch_output_positive_existing() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("file");
    ucmd.args(&["-n0", "-o", "file"]).succeeds().no_output();
    assert_eq!(
        at.read_bytes("file"),
        Vec::new(),
        "Output file must exist and be completely empty"
    );
}

#[test]
fn test_zero_head_count_echo() {
    new_ucmd!()
        .arg("-n0")
        .arg("-e")
        .arg("hello")
        .pipe_in(vec![])
        .succeeds()
        .no_output();
}

#[test]
fn test_zero_head_count_range() {
    new_ucmd!().arg("-n0").arg("-i4-8").succeeds().no_output();
}

#[test]
fn test_head_count_multi_big_then_small() {
    let repeat_limit = 5;
    let input_seq = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let input = input_seq
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>()
        .join("\n");

    let result = new_ucmd!()
        .arg("-n")
        .arg((repeat_limit + 1).to_string())
        .arg("-n")
        .arg(repeat_limit.to_string())
        .pipe_in(input.as_bytes())
        .succeeds();
    result.no_stderr();

    let result_seq: Vec<i32> = result
        .stdout_str()
        .split('\n')
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    assert_eq!(result_seq.len(), repeat_limit, "Output is not limited");
    assert!(
        result_seq.iter().all(|x| input_seq.contains(x)),
        "Output includes element not from input: {}",
        result.stdout_str()
    );
}

#[test]
fn test_head_count_multi_small_then_big() {
    let repeat_limit = 5;
    let input_seq = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let input = input_seq
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>()
        .join("\n");

    let result = new_ucmd!()
        .arg("-n")
        .arg(repeat_limit.to_string())
        .arg("-n")
        .arg((repeat_limit + 1).to_string())
        .pipe_in(input.as_bytes())
        .succeeds();
    result.no_stderr();

    let result_seq: Vec<i32> = result
        .stdout_str()
        .split('\n')
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    assert_eq!(result_seq.len(), repeat_limit, "Output is not limited");
    assert!(
        result_seq.iter().all(|x| input_seq.contains(x)),
        "Output includes element not from input: {}",
        result.stdout_str()
    );
}

#[test]
fn test_repeat() {
    let repeat_limit = 15000;
    let input_seq = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let input = input_seq
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>()
        .join("\n");

    let result = new_ucmd!()
        .arg("-r")
        .args(&["-n", &repeat_limit.to_string()])
        .pipe_in(input.as_bytes())
        .succeeds();
    result.no_stderr();

    let result_seq: Vec<i32> = result
        .stdout_str()
        .split('\n')
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    assert_eq!(
        result_seq.len(),
        repeat_limit,
        "Output is not repeating forever"
    );
    assert!(
        result_seq.iter().all(|x| input_seq.contains(x)),
        "Output includes element not from input: {:?}",
        result_seq
            .iter()
            .filter(|x| !input_seq.contains(x))
            .collect::<Vec<&i32>>()
    );
}

#[test]
fn test_repeat_multi() {
    let repeat_limit = 15000;
    let input_seq = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let input = input_seq
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>()
        .join("\n");

    let result = new_ucmd!()
        .arg("-r")
        .arg("-r") // The only difference to test_repeat()
        .args(&["-n", &repeat_limit.to_string()])
        .pipe_in(input.as_bytes())
        .succeeds();
    result.no_stderr();

    let result_seq: Vec<i32> = result
        .stdout_str()
        .split('\n')
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    assert_eq!(
        result_seq.len(),
        repeat_limit,
        "Output is not repeating forever"
    );
    assert!(
        result_seq.iter().all(|x| input_seq.contains(x)),
        "Output includes element not from input: {:?}",
        result_seq
            .iter()
            .filter(|x| !input_seq.contains(x))
            .collect::<Vec<&i32>>()
    );
}

#[test]
fn test_file_input() {
    let expected_seq = vec![11, 12, 13, 14, 15, 16, 17, 18, 19, 20];

    let result = new_ucmd!().arg("file_input.txt").succeeds();
    result.no_stderr();

    let mut result_seq: Vec<i32> = result
        .stdout_str()
        .split('\n')
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    result_seq.sort_unstable();
    assert_eq!(result_seq, expected_seq, "Output is not a permutation");
}

#[test]
fn test_shuf_echo_and_input_range_not_allowed() {
    new_ucmd!()
        .args(&["-e", "0", "-i", "0-2"])
        .fails()
        .stderr_contains("cannot be used with");
}

#[test]
fn test_shuf_input_range_and_file_not_allowed() {
    new_ucmd!()
        .args(&["-i", "0-9", "file"])
        .fails()
        .stderr_contains("cannot be used with");
}

#[test]
fn test_shuf_invalid_input_range_one() {
    new_ucmd!()
        .args(&["-i", "0"])
        .fails()
        .stderr_contains("invalid value '0' for '--input-range <LO-HI>': missing '-'");
}

#[test]
fn test_shuf_invalid_input_range_two() {
    new_ucmd!().args(&["-i", "a-9"]).fails().stderr_contains(
        "invalid value 'a-9' for '--input-range <LO-HI>': invalid digit found in string",
    );
}

#[test]
fn test_shuf_invalid_input_range_three() {
    new_ucmd!().args(&["-i", "0-b"]).fails().stderr_contains(
        "invalid value '0-b' for '--input-range <LO-HI>': invalid digit found in string",
    );
}

#[test]
fn test_shuf_multiple_input_ranges() {
    new_ucmd!()
        .args(&["-i", "2-9", "-i", "2-9"])
        .fails()
        .stderr_contains("--input-range")
        .stderr_contains("cannot be used multiple times");
}

#[test]
fn test_shuf_multiple_outputs() {
    new_ucmd!()
        .args(&["-o", "file_a", "-o", "file_b"])
        .fails()
        .stderr_contains("--output")
        .stderr_contains("cannot be used multiple times");
}

#[test]
fn test_shuf_two_input_files() {
    new_ucmd!()
        .args(&["file_a", "file_b"])
        .fails()
        .stderr_contains("unexpected argument 'file_b' found");
}

#[test]
fn test_shuf_three_input_files() {
    new_ucmd!()
        .args(&["file_a", "file_b", "file_c"])
        .fails()
        .stderr_contains("unexpected argument 'file_b' found");
}

#[test]
fn test_shuf_invalid_input_line_count() {
    new_ucmd!().args(&["-n", "a"]).fails().stderr_contains(
        "invalid value 'a' for '--head-count <COUNT>': invalid digit found in string",
    );
}

#[test]
fn test_shuf_multiple_input_line_count() {
    let result = new_ucmd!()
        .args(&["-i10-200", "-n", "10", "-n", "5"])
        .succeeds();

    result.no_stderr();

    let result_count = result
        .stdout_str()
        .split('\n')
        .filter(|x| !x.is_empty())
        .count();
    assert_eq!(result_count, 5, "Output should have 5 items");
}

#[test]
fn test_shuf_repeat_empty_range() {
    new_ucmd!()
        .arg("-ri4-3")
        .fails()
        .no_stdout()
        .stderr_only("shuf: no lines to repeat\n");
}

#[test]
fn test_shuf_repeat_empty_echo() {
    new_ucmd!()
        .arg("-re")
        .fails()
        .no_stdout()
        .stderr_only("shuf: no lines to repeat\n");
}

#[test]
fn test_shuf_repeat_empty_input() {
    new_ucmd!()
        .arg("-r")
        .pipe_in("")
        .fails()
        .no_stdout()
        .stderr_only("shuf: no lines to repeat\n");
}

#[test]
fn test_range_one_elem() {
    new_ucmd!()
        .arg("-i5-5")
        .succeeds()
        .no_stderr()
        .stdout_only("5\n");
}

#[test]
fn test_range_empty() {
    new_ucmd!().arg("-i5-4").succeeds().no_output();
}

#[test]
fn test_range_empty_minus_one() {
    new_ucmd!()
        .arg("-i5-3")
        .fails()
        .no_stdout()
        .stderr_contains("invalid value '5-3' for '--input-range <LO-HI>': start exceeds end\n");
}

#[test]
fn test_range_repeat_one_elem() {
    new_ucmd!()
        .arg("-n1")
        .arg("-ri5-5")
        .succeeds()
        .no_stderr()
        .stdout_only("5\n");
}

#[test]
fn test_range_repeat_empty() {
    new_ucmd!()
        .arg("-n1")
        .arg("-ri5-4")
        .fails()
        .no_stdout()
        .stderr_only("shuf: no lines to repeat\n");
}

#[test]
fn test_range_repeat_empty_minus_one() {
    new_ucmd!()
        .arg("-n1")
        .arg("-ri5-3")
        .fails()
        .no_stdout()
        .stderr_contains("invalid value '5-3' for '--input-range <LO-HI>': start exceeds end\n");
}

// This test fails if we forget to flush the `BufWriter`.
#[test]
#[cfg(target_os = "linux")]
fn write_errors_are_reported() {
    new_ucmd!()
        .arg("-i1-10")
        .arg("-o/dev/full")
        .fails()
        .no_stdout()
        .stderr_is("shuf: write failed: No space left on device\n");
}

// On 32-bit platforms, if we cast carelessly, this will give no output.
#[test]
fn test_head_count_does_not_overflow_file() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.append("input.txt", "hello\n");

    ucmd.arg(format!("-n{}", u64::from(u32::MAX) + 1))
        .arg("input.txt")
        .succeeds()
        .stdout_is("hello\n")
        .no_stderr();
}

#[test]
fn test_head_count_does_not_overflow_args() {
    new_ucmd!()
        .arg(format!("-n{}", u64::from(u32::MAX) + 1))
        .arg("-e")
        .arg("goodbye")
        .succeeds()
        .stdout_is("goodbye\n")
        .no_stderr();
}

#[test]
fn test_head_count_does_not_overflow_range() {
    new_ucmd!()
        .arg(format!("-n{}", u64::from(u32::MAX) + 1))
        .arg("-i1-1")
        .succeeds()
        .stdout_is("1\n")
        .no_stderr();
}

// Test reproducibility and compatibility of --random-source.
// These hard-coded results match those of GNU shuf. They should not be changed.

#[test]
fn test_gnu_compat_range_repeat() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.append_bytes(
        "random_bytes.bin",
        b"\xfb\x83\x8f\x21\x9b\x3c\x2d\xc5\x73\xa5\x58\x6c\x54\x2f\x59\xf8",
    );

    ucmd.arg("--random-source=random_bytes.bin")
        .arg("-r")
        .arg("-i1-99")
        .fails_with_code(1)
        .stderr_is("shuf: end of random source\n")
        .stdout_is("38\n30\n10\n26\n23\n61\n46\n99\n75\n43\n10\n89\n10\n44\n24\n59\n22\n51\n");
}

#[test]
fn test_gnu_compat_args_no_repeat() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.append_bytes(
        "random_bytes.bin",
        b"\xd1\xfd\xb9\x9a\xf5\x81\x71\x42\xf9\x7a\x59\x79\xd4\x9c\x8c\x7d",
    );

    ucmd.arg("--random-source=random_bytes.bin")
        .arg("-e")
        .args(&["1", "2", "3", "4", "5", "6", "7"][..])
        .succeeds()
        .no_stderr()
        .stdout_is("7\n1\n2\n5\n3\n4\n6\n");
}

#[test]
fn test_gnu_compat_from_stdin() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.append_bytes(
        "random_bytes.bin",
        b"\xd1\xfd\xb9\x9a\xf5\x81\x71\x42\xf9\x7a\x59\x79\xd4\x9c\x8c\x7d",
    );

    at.append("input.txt", "1\n2\n3\n4\n5\n6\n7\n");

    ucmd.arg("--random-source=random_bytes.bin")
        .set_stdin(at.open("input.txt"))
        .succeeds()
        .no_stderr()
        .stdout_is("7\n1\n2\n5\n3\n4\n6\n");
}

#[test]
fn test_gnu_compat_from_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.append_bytes(
        "random_bytes.bin",
        b"\xd1\xfd\xb9\x9a\xf5\x81\x71\x42\xf9\x7a\x59\x79\xd4\x9c\x8c\x7d",
    );

    at.append("input.txt", "1\n2\n3\n4\n5\n6\n7\n");

    ucmd.arg("--random-source=random_bytes.bin")
        .arg("input.txt")
        .succeeds()
        .no_stderr()
        .stdout_is("7\n1\n2\n5\n3\n4\n6\n");
}

#[test]
fn test_gnu_compat_limited_from_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.append_bytes(
        "random_bytes.bin",
        b"\xd1\xfd\xb9\x9a\xf5\x81\x71\x42\xf9\x7a\x59\x79\xd4\x9c\x8c\x7d",
    );

    at.append("input.txt", "1\n2\n3\n4\n5\n6\n7\n");

    ucmd.arg("--random-source=random_bytes.bin")
        .arg("-n5")
        .arg("input.txt")
        .succeeds()
        .no_stderr()
        .stdout_is("7\n1\n2\n5\n3\n");
}

// This specific case causes GNU to give different results than other modes.
#[ignore = "disabled until fixed"]
#[test]
fn test_gnu_compat_limited_from_stdin() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.append_bytes(
        "random_bytes.bin",
        b"\xd1\xfd\xb9\x9a\xf5\x81\x71\x42\xf9\x7a\x59\x79\xd4\x9c\x8c\x7d",
    );

    at.append("input.txt", "1\n2\n3\n4\n5\n6\n7\n");

    ucmd.arg("--random-source=random_bytes.bin")
        .arg("-n7")
        .set_stdin(at.open("input.txt"))
        .succeeds()
        .no_stderr()
        .stdout_is("6\n5\n1\n3\n2\n7\n4\n");
}

#[test]
fn test_gnu_compat_range_no_repeat() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.append_bytes(
        "random_bytes.bin",
        b"\xd1\xfd\xb9\x9a\xf5\x81\x71\x42\xf9\x7a\x59\x79\xd4\x9c\x8c\x7d",
    );

    ucmd.arg("--random-source=random_bytes.bin")
        .arg("-i1-10")
        .succeeds()
        .no_stderr()
        .stdout_is("10\n2\n8\n7\n3\n9\n6\n5\n1\n4\n");
}

// Test reproducibility of --random-seed.
// These results are arbitrary but they should not change unless we choose to break compatibility.

#[test]
fn test_seed_args_repeat() {
    new_ucmd!()
        .arg("--random-seed=🌱")
        .arg("-e")
        .arg("-r")
        .arg("-n10")
        .args(&["foo", "bar", "baz", "qux"])
        .succeeds()
        .no_stderr()
        .stdout_is("qux\nbar\nbaz\nfoo\nbaz\nqux\nqux\nfoo\nqux\nqux\n");
}

#[test]
fn test_seed_args_no_repeat() {
    new_ucmd!()
        .arg("--random-seed=🌱")
        .arg("-e")
        .args(&["foo", "bar", "baz", "qux"])
        .succeeds()
        .no_stderr()
        .stdout_is("qux\nbaz\nfoo\nbar\n");
}

#[test]
fn test_seed_range_repeat() {
    new_ucmd!()
        .arg("--random-seed=🦀")
        .arg("-r")
        .arg("-i1-99")
        .arg("-n10")
        .succeeds()
        .no_stderr()
        .stdout_is("60\n44\n38\n41\n63\n43\n31\n71\n46\n90\n");
}

#[test]
fn test_seed_range_no_repeat() {
    let expected = "8\n9\n1\n5\n2\n6\n4\n3\n10\n7\n";

    new_ucmd!()
        .arg("--random-seed=12345")
        .arg("-i1-10")
        .succeeds()
        .no_stderr()
        .stdout_is(expected);

    // Piping from e.g. seq gives identical results.
    new_ucmd!()
        .arg("--random-seed=12345")
        .pipe_in("1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n")
        .succeeds()
        .no_stderr()
        .stdout_is(expected);
}

// Test a longer input to exercise some more code paths in the sparse representation.
#[test]
fn test_seed_long_range_no_repeat() {
    let expected = "\
        1\n3\n35\n37\n36\n45\n72\n17\n18\n40\n67\n74\n81\n77\n14\n90\n\
        7\n12\n80\n54\n23\n61\n29\n41\n15\n56\n6\n32\n82\n76\n11\n2\n100\n\
        50\n60\n97\n73\n79\n91\n89\n85\n86\n66\n70\n22\n55\n8\n83\n39\n27\n";

    new_ucmd!()
        .arg("--random-seed=67890")
        .arg("-i1-100")
        .arg("-n50")
        .succeeds()
        .no_stderr()
        .stdout_is(expected);

    let mut test_input = String::new();
    for n in 1..=100 {
        writeln!(&mut test_input, "{n}").unwrap();
    }

    new_ucmd!()
        .arg("--random-seed=67890")
        .pipe_in(test_input.as_bytes())
        .arg("-n50")
        .succeeds()
        .no_stderr()
        .stdout_is(expected);
}

#[test]
fn test_empty_range_no_repeat() {
    new_ucmd!().arg("-i4-3").succeeds().no_stderr().no_stdout();
}
