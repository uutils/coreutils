use crate::common::util::*;

#[test]
fn test_missing_operand() {
    let result = new_ucmd!().run();

    assert_eq!(
        true,
        result
            .stderr
            .starts_with("error: The following required arguments were not provided")
    );

    assert_eq!(true, result.stderr.contains("<newroot>"));
}

#[test]
fn test_no_such_directory() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch(&at.plus_as_string("a"));

    ucmd.arg("a")
        .fails()
        .stderr_is("chroot: error: cannot change root directory to `a`: no such directory");
}
