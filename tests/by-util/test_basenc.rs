use crate::common::util::*;

#[test]
fn test_z85_not_padded() {
    // The z85 crate deviates from the standard in some cases; we have to catch those
    new_ucmd!()
        .args(&["--z85", "-d"])
        .pipe_in("##########")
        .fails()
        .stderr_only("basenc: error: invalid input");
    new_ucmd!()
        .args(&["--z85"])
        .pipe_in("123")
        .fails()
        .stderr_only("basenc: error: invalid input (length must be multiple of 4 characters)");
}
