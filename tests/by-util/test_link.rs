use crate::common::util::*;

#[cfg(not(target_os = "android"))]
#[test]
fn test_link_existing_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_link_existing_file";
    let link = "test_link_existing_file_link";

    at.touch(file);
    at.write(file, "foobar");
    assert!(at.file_exists(file));

    ucmd.args(&[file, link]).succeeds().no_stderr();
    assert!(at.file_exists(file));
    assert!(at.file_exists(link));
    assert_eq!(at.read(file), at.read(link));
}

#[test]
fn test_link_no_circular() {
    let (at, mut ucmd) = at_and_ucmd!();
    let link = "test_link_no_circular";

    ucmd.args(&[link, link])
        .fails()
        .stderr_is("link: cannot create link 'test_link_no_circular' to 'test_link_no_circular': No such file or directory");
    assert!(!at.file_exists(link));
}

#[test]
fn test_link_nonexistent_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_link_nonexistent_file";
    let link = "test_link_nonexistent_file_link";

    ucmd.args(&[file, link])
        .fails()
        .stderr_only("link: cannot create link 'test_link_nonexistent_file_link' to 'test_link_nonexistent_file': No such file or directory");
    assert!(!at.file_exists(file));
    assert!(!at.file_exists(link));
}

#[test]
fn test_link_one_argument() {
    let (_, mut ucmd) = at_and_ucmd!();
    let file = "test_link_argument";
    ucmd.args(&[file]).fails().stderr_contains(
        "error: The argument '<FILES>...' requires at least 2 values but only 1 was provided",
    );
}

#[test]
fn test_link_three_arguments() {
    let (_, mut ucmd) = at_and_ucmd!();
    let arguments = vec![
        "test_link_argument1",
        "test_link_argument2",
        "test_link_argument3",
    ];
    ucmd.args(&arguments[..]).fails().stderr_contains(
        format!("error: The value '{}' was provided to '<FILES>...' but it wasn't expecting any more values", arguments[2]),
    );
}
