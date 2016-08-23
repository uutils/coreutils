use common::util::*;


static GIBBERISH: &'static str = "supercalifragilisticexpialidocious";

#[test]
fn test_canonicalize() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg("-f")
        .arg(".")
        .run()
        .stdout_is(at.root_dir_resolved());
}

#[test]
fn test_canonicalize_existing() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg("-e")
        .arg(".")
        .run()
        .stdout_is(at.root_dir_resolved());
}

#[test]
fn test_canonicalize_missing() {
    let (at, mut ucmd) = at_and_ucmd!();
    let expected = path_concat!(at.root_dir_resolved(), GIBBERISH);
    ucmd.arg("-m")
        .arg(GIBBERISH)
        .run()
        .stdout_is(expected);
}

#[test]
fn test_long_redirection_to_current_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    // Create a 256-character path to current directory
    let dir = path_concat!(".", ..128);
    ucmd.arg("-n")
        .arg("-m")
        .arg(dir)
        .run()
        .stdout_is(at.root_dir_resolved());
}

#[test]
fn test_long_redirection_to_root() {
    // Create a 255-character path to root
    let dir = path_concat!("..", ..85);
    new_ucmd!()
        .arg("-n")
        .arg("-m")
        .arg(dir)
        .run()
        .stdout_is(get_root_path());
}
