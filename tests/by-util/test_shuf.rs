use crate::common::util::*;

#[test]
fn test_output_is_random_permutation() {
    let input_seq = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let input = input_seq
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join("\n");

    let result = new_ucmd!()
        .pipe_in(input.as_bytes())
        .succeeds()
        .no_stderr()
        .stdout_str();

    let mut result_seq: Vec<i32> = result
        .split("\n")
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    result_seq.sort();
    assert_ne!(result, input, "Output is not randomised");
    assert_eq!(result_seq, input_seq, "Output is not a permutation");
}

#[test]
fn test_zero_termination() {
    let input_seq = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let result = new_ucmd!()
        .arg("-z")
        .arg("-i1-10")
        .succeeds()
        .no_stderr()
        .stdout_str();

    let mut result_seq: Vec<i32> = result
        .split("\0")
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    result_seq.sort();
    assert_eq!(result_seq, input_seq, "Output is not a permutation");
}

#[test]
fn test_echo() {
    let input_seq = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let result = new_ucmd!()
        .arg("-e")
        .args(
            &input_seq
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>(),
        )
        .succeeds()
        .no_stderr()
        .stdout_str();

    let mut result_seq: Vec<i32> = result
        .split("\n")
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    result_seq.sort();
    assert_eq!(result_seq, input_seq, "Output is not a permutation");
}

#[test]
fn test_head_count() {
    let repeat_limit = 5;
    let input_seq = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let input = input_seq
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join("\n");

    let result = new_ucmd!()
        .args(&["-n", &repeat_limit.to_string()])
        .pipe_in(input.as_bytes())
        .succeeds()
        .no_stderr()
        .stdout_str();

    let mut result_seq: Vec<i32> = result
        .split("\n")
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    result_seq.sort();
    assert_eq!(result_seq.len(), repeat_limit, "Output is not limited");
    assert!(
        result_seq.iter().all(|x| input_seq.contains(x)),
        format!("Output includes element not from input: {}", result)
    )
}

#[test]
fn test_repeat() {
    let repeat_limit = 15000;
    let input_seq = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let input = input_seq
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join("\n");

    let result = new_ucmd!()
        .arg("-r")
        .args(&["-n", &repeat_limit.to_string()])
        .pipe_in(input.as_bytes())
        .succeeds()
        .no_stderr()
        .stdout_str();

    let result_seq: Vec<i32> = result
        .split("\n")
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
        format!(
            "Output includes element not from input: {:?}",
            result_seq
                .iter()
                .filter(|x| !input_seq.contains(x))
                .collect::<Vec<&i32>>()
        )
    )
}

#[test]
fn test_file_input() {
    let expected_seq = vec![11, 12, 13, 14, 15, 16, 17, 18, 19, 20];

    let result = new_ucmd!()
        .arg("file_input.txt")
        .succeeds()
        .no_stderr()
        .stdout_str();

    let mut result_seq: Vec<i32> = result
        .split("\n")
        .filter(|x| !x.is_empty())
        .map(|x| x.parse().unwrap())
        .collect();
    result_seq.sort();
    assert_eq!(result_seq, expected_seq, "Output is not a permutation");
}

#[test]
fn test_shuf_echo_and_input_range_not_allowed() {
    let result = new_ucmd!().args(&["-e", "0", "-i", "0-2"]).fails();

     assert!(result
        .stderr_str()
        .contains("The argument '--input-range <LO-HI>' cannot be used with '--echo <ARG>...'"));
}

#[test]
fn test_shuf_input_range_and_file_not_allowed() {
    let result = new_ucmd!().args(&["-i", "0-9", "file"]).fails();

    assert!(result
        .stderr_str()
        .contains("The argument '<file>' cannot be used with '--input-range <LO-HI>'"));
}

#[test]
fn test_shuf_invalid_input_range_one() {
    let result = new_ucmd!().args(&["-i", "0"]).fails();

    assert!(result.stderr_str().contains("invalid input range"));
}

#[test]
fn test_shuf_invalid_input_range_two() {
    let result = new_ucmd!().args(&["-i", "a-9"]).fails();

    assert!(result.stderr_str().contains("invalid input range: 'a'"));
}

#[test]
fn test_shuf_invalid_input_range_three() {
    let result = new_ucmd!().args(&["-i", "0-b"]).fails();

    assert!(result.stderr_str().contains("invalid input range: 'b'"));
}

#[test]
fn test_shuf_invalid_input_line_count() {
    let result = new_ucmd!().args(&["-n", "a"]).fails();

    assert!(result.stderr_str().contains("invalid line count: 'a'"));
}
