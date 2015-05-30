use std::process::Command;
use util::*;

static PROGNAME: &'static str = "./ptx";

#[path = "common/util.rs"]
#[macro_use]
mod util;

#[test]
fn gnu_ext_disabled_roff_no_ref() {
    let opts = vec!["-G", "-R"];
    test_ptx(&opts);
}

#[test]
fn gnu_ext_disabled_roff_input_ref() {
    let opts = vec!["-G", "-r", "-R"];
    test_ptx(&opts);
}

#[test]
fn gnu_ext_disabled_roff_auto_ref() {
    let opts = vec!["-G", "-A", "-R"];
    test_ptx(&opts);
}

#[test]
fn gnu_ext_disabled_tex_no_ref() {
    let opts = vec!["-G", "-T", "-R"];
    test_ptx(&opts);
}

#[test]
fn gnu_ext_disabled_tex_input_ref() {
    let opts = vec!["-G", "-T", "-r", "-R"];
    test_ptx(&opts);
}

#[test]
fn gnu_ext_disabled_tex_auto_ref() {
    let opts = vec!["-G", "-T", "-A", "-R"];
    test_ptx(&opts);
}

#[test]
fn gnu_ext_disabled_ignore_and_only_file() {
    let opts = vec!["-G", "-o", "only", "-i", "ignore"];
    test_ptx(&opts);
}

fn test_ptx(opts: &Vec<&str>) {
    let mut ptx = Command::new(PROGNAME);
    let result = run(&mut ptx.args(opts).arg("input"));
    let mut gnu_ptx = Command::new("ptx");
    let gnu_result = run(&mut gnu_ptx.args(opts).arg("input"));
    assert_eq!(result.success, true);
    assert_eq!(result.stdout, gnu_result.stdout);
    assert_empty_stderr!(&result);
}