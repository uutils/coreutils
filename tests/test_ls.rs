use common::util::*;

utility_test!();

#[test]
fn test_ls_ls() {
    new_ucmd().succeeds();
}
