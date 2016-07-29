use common::util::*;

static UTIL_NAME: &'static str = "readlink";
fn at_and_ucmd() -> (AtPath, UCommand) {
    let ts = TestScenario::new(UTIL_NAME);
    let ucmd = ts.ucmd();
    (ts.fixtures, ucmd)
}
fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

static GIBBERISH: &'static str = "supercalifragilisticexpialidocious";

#[test]
fn test_canonicalize() {
    let (at, mut ucmd) = at_and_ucmd();
    let out = ucmd.arg("-f")
                  .arg(".")
                  .run()
                  .stdout;

    assert_eq!(out.trim_right(), at.root_dir_resolved());
}

#[test]
fn test_canonicalize_existing() {
    let (at, mut ucmd) = at_and_ucmd();
    let out = ucmd.arg("-e")
                  .arg(".")
                  .run()
                  .stdout;

    assert_eq!(out.trim_right(), at.root_dir_resolved());
}

#[test]
fn test_canonicalize_missing() {
    let (at, mut ucmd) = at_and_ucmd();
    let expected = path_concat!(at.root_dir_resolved(), GIBBERISH);

    let out = ucmd.arg("-m")
                  .arg(GIBBERISH)
                  .run()
                  .stdout;

    assert_eq!(out.trim_right(), expected);
}

#[test]
fn test_long_redirection_to_current_dir() {
    let (at, mut ucmd) = at_and_ucmd();
    // Create a 256-character path to current directory
    let dir = path_concat!(".", ..128);
    let out = ucmd.arg("-n")
                  .arg("-m")
                  .arg(dir)
                  .run()
                  .stdout;

    assert_eq!(out, at.root_dir_resolved());
}

#[test]
fn test_long_redirection_to_root() {
    // Create a 255-character path to root
    let dir = path_concat!("..", ..85);
    let out = new_ucmd()
                  .arg("-n")
                  .arg("-m")
                  .arg(dir)
                  .run()
                  .stdout;

    assert_eq!(out, get_root_path());
}
