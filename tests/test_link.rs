use common::util::*;


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

    ucmd.args(&[link, link]).fails()
        .stderr_is("link: error: No such file or directory (os error 2)\n");
    assert!(!at.file_exists(link));
}

#[test]
fn test_link_nonexistent_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let file = "test_link_nonexistent_file";
    let link = "test_link_nonexistent_file_link";

    ucmd.args(&[file, link]).fails()
        .stderr_is("link: error: No such file or directory (os error 2)\n");
    assert!(!at.file_exists(file));
    assert!(!at.file_exists(link));
}
