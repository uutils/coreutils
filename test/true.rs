use std::process::Command;

static PROGNAME: &'static str = "./true";

#[test]
fn test_exit_code() {
    let exit_status = Command::new(PROGNAME).status().unwrap().success();
    assert_eq!(exit_status, true);
}
