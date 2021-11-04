use crate::common::util::*;

static GIBBERISH: &str = "supercalifragilisticexpialidocious";

#[test]
fn test_resolve() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("foo");
    at.symlink_file("foo", "bar");

    scene.ucmd().arg("bar").succeeds().stdout_contains("foo\n");
}

#[test]
fn test_canonicalize() {
    let (at, mut ucmd) = at_and_ucmd!();
    let actual = ucmd.arg("-f").arg(".").run().stdout_move_str();
    let expect = at.root_dir_resolved() + "\n";
    println!("actual: {:?}", actual);
    println!("expect: {:?}", expect);
    assert_eq!(actual, expect);
}

#[test]
fn test_canonicalize_existing() {
    let (at, mut ucmd) = at_and_ucmd!();
    let actual = ucmd.arg("-e").arg(".").run().stdout_move_str();
    let expect = at.root_dir_resolved() + "\n";
    println!("actual: {:?}", actual);
    println!("expect: {:?}", expect);
    assert_eq!(actual, expect);
}

#[test]
fn test_canonicalize_missing() {
    let (at, mut ucmd) = at_and_ucmd!();
    let actual = ucmd.arg("-m").arg(GIBBERISH).run().stdout_move_str();
    let expect = path_concat!(at.root_dir_resolved(), GIBBERISH) + "\n";
    println!("actual: {:?}", actual);
    println!("expect: {:?}", expect);
    assert_eq!(actual, expect);
}

#[test]
fn test_long_redirection_to_current_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    // Create a 256-character path to current directory
    let dir = path_concat!(".", ..128);
    let actual = ucmd.arg("-n").arg("-m").arg(dir).run().stdout_move_str();
    let expect = at.root_dir_resolved();
    println!("actual: {:?}", actual);
    println!("expect: {:?}", expect);
    assert_eq!(actual, expect);
}

#[test]
fn test_long_redirection_to_root() {
    // Create a 255-character path to root
    let dir = path_concat!("..", ..85);
    let actual = new_ucmd!()
        .arg("-n")
        .arg("-m")
        .arg(dir)
        .run()
        .stdout_move_str();
    let expect = get_root_path();
    println!("actual: {:?}", actual);
    println!("expect: {:?}", expect);
    assert_eq!(actual, expect);
}
