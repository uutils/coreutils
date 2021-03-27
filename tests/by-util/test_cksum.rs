use crate::common::util::*;

#[test]
fn test_single_file() {
    new_ucmd!()
        .arg("lorem_ipsum.txt")
        .succeeds()
        .stdout_is_fixture("single_file.expected");
}

#[test]
fn test_multiple_files() {
    new_ucmd!()
        .arg("lorem_ipsum.txt")
        .arg("alice_in_wonderland.txt")
        .succeeds()
        .stdout_is_fixture("multiple_files.expected");
}

#[test]
fn test_stdin() {
    new_ucmd!()
        .pipe_in_fixture("lorem_ipsum.txt")
        .succeeds()
        .stdout_is_fixture("stdin.expected");
}

#[test]
fn test_empty() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch("a");

    ucmd.arg("a").succeeds().stdout.ends_with("0 a");
}

#[test]
fn test_arg_overrides_stdin() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch("a");

    ucmd.pipe_in("foobarfoobar")
        .arg("a")
        .succeeds()
        .stdout
        .ends_with("0 a");
}

#[test]
fn test_invalid_file() {
    let (at, mut ucmd) = at_and_ucmd!();

    let folder_name = "a".to_string();
    at.mkdir(&folder_name);

    ucmd.arg(&folder_name)
        .fails()
        .stderr
        .contains("cksum: error: 'a'");
}

// Make sure crc is correct for files larger than 32 bytes
// but <128 bytes (1 fold pclmul)
#[test]
fn test_crc_for_bigger_than_32_bytes() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("chars.txt").run();

    let stdout_splitted: Vec<&str> = result.stdout.split(" ").collect();

    let cksum: i64 = stdout_splitted[0].parse().unwrap();
    let bytes_cnt: i64 = stdout_splitted[1].parse().unwrap();

    assert!(result.success);
    assert_eq!(cksum, 586047089);
    assert_eq!(bytes_cnt, 16);
}

#[test]
fn test_stdin_larger_than_128_bytes() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("larger_than_2056_bytes.txt").run();

    let stdout_splitted: Vec<&str> = result.stdout.split(" ").collect();

    let cksum: i64 = stdout_splitted[0].parse().unwrap();
    let bytes_cnt: i64 = stdout_splitted[1].parse().unwrap();

    assert!(result.success);
    assert_eq!(cksum, 945881979);
    assert_eq!(bytes_cnt, 2058);
}
