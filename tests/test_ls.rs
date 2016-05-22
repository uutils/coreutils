use common::util::*;

static UTIL_NAME: &'static str = "ls";

#[test]
fn test_ls_ls() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    
    let result = ucmd.run();
    
    let exit_success = result.success;
    assert_eq!(exit_success, true);
}
