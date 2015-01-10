#![allow(unstable)]

use std::io::process::Command;

static EXE: &'static str = "./test";

#[test]
fn test_op_prec_and_or_1() {
    let status = Command::new(EXE).arg(" ").arg("-o").arg("").arg("-a").arg("").status();
    assert_eq!(true, status.unwrap().success());
}

#[test]
fn test_op_prec_and_or_2() {
    let status = Command::new(EXE).arg("")
                                   .arg("-a")
                                   .arg("")
                                   .arg("-o")
                                   .arg(" ")
                                   .arg("-a")
                                   .arg(" ")
                                   .status();
    assert_eq!(true, status.unwrap().success());
}

#[test]
fn test_or_as_filename() {
    let status = Command::new(EXE).arg("x").arg("-a").arg("-z").arg("-o").status();
    assert!(status.unwrap().matches_exit_status(1));
}

