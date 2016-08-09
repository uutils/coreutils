use common::util::*;

static UTIL_NAME: &'static str = "dirname";

fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[test]
fn test_path_with_trailing_slashes() {
    let dir = "/root/alpha/beta/gamma/delta/epsilon/omega//";
    let out = new_ucmd().arg(dir).run().stdout;

    assert_eq!(out.trim_right(), "/root/alpha/beta/gamma/delta/epsilon");
}

#[test]
fn test_path_without_trailing_slashes() {
    let dir = "/root/alpha/beta/gamma/delta/epsilon/omega";
    let out = new_ucmd().arg(dir).run().stdout;

    assert_eq!(out.trim_right(), "/root/alpha/beta/gamma/delta/epsilon");
}

#[test]
fn test_root() {
    let dir = "/";
    let out = new_ucmd().arg(dir).run().stdout;

    assert_eq!(out.trim_right(), "/");
}

#[test]
fn test_pwd() {
    let dir = ".";
    let out = new_ucmd().arg(dir).run().stdout;

    assert_eq!(out.trim_right(), ".");
}

#[test]
fn test_empty() {
    let dir = "";
    let out = new_ucmd().arg(dir).run().stdout;

    assert_eq!(out.trim_right(), ".");
}
