use common::util::*;

utility_test!();

#[test]
fn test_exit_code() {
    new_ucmd().fails();
}
