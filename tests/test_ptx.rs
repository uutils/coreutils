use common::util::*;

static UTIL_NAME: &'static str = "ptx";
fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[test]
fn gnu_ext_disabled_roff_no_ref() {
    let opts = vec!["-G", "-R"];
    test_ptx(&opts, "gnu_ext_disabled_roff_no_ref.expected");
}

#[test]
fn gnu_ext_disabled_roff_input_ref() {
    let opts = vec!["-G", "-r", "-R"];
    test_ptx(&opts, "gnu_ext_disabled_roff_input_ref.expected");
}

#[test]
fn gnu_ext_disabled_roff_auto_ref() {
    let opts = vec!["-G", "-A", "-R"];
    test_ptx(&opts, "gnu_ext_disabled_roff_auto_ref.expected");
}

#[test]
fn gnu_ext_disabled_tex_no_ref() {
    let opts = vec!["-G", "-T", "-R"];
    test_ptx(&opts, "gnu_ext_disabled_tex_no_ref.expected");
}

#[test]
fn gnu_ext_disabled_tex_input_ref() {
    let opts = vec!["-G", "-T", "-r", "-R"];
    test_ptx(&opts, "gnu_ext_disabled_tex_input_ref.expected");
}

#[test]
fn gnu_ext_disabled_tex_auto_ref() {
    let opts = vec!["-G", "-T", "-A", "-R"];
    test_ptx(&opts, "gnu_ext_disabled_tex_auto_ref.expected");
}

#[test]
fn gnu_ext_disabled_ignore_and_only_file() {
    let opts = vec!["-G", "-o", "only", "-i", "ignore"];
    test_ptx(&opts, "gnu_ext_disabled_ignore_and_only_file.expected");
}

fn test_ptx(opts: &Vec<&str>, expected: &str) {
    new_ucmd().args(opts).arg("input").succeeds().stdout_only_fixture(expected);
}
