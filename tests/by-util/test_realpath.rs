use crate::common::util::*;

#[test]
fn test_current_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    let actual = ucmd.arg(".").run().stdout;
    let expect = at.root_dir_resolved() + "\n";
    println!("actual: {:?}", actual);
    println!("expect: {:?}", expect);
    assert_eq!(actual, expect);
}

#[test]
fn test_long_redirection_to_current_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    // Create a 256-character path to current directory
    let dir = path_concat!(".", ..128);
    let actual = ucmd.arg(dir).run().stdout;
    let expect = at.root_dir_resolved() + "\n";
    println!("actual: {:?}", actual);
    println!("expect: {:?}", expect);
    assert_eq!(actual, expect);
}

#[test]
fn test_long_redirection_to_root() {
    // Create a 255-character path to root
    let dir = path_concat!("..", ..85);
    let actual = new_ucmd!().arg(dir).run().stdout;
    let expect = get_root_path().to_owned() + "\n";
    println!("actual: {:?}", actual);
    println!("expect: {:?}", expect);
    assert_eq!(actual, expect);
}

#[test]
fn test_file_and_links() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("foo");
    at.symlink_file("foo", "bar");

    let actual = scene.ucmd().arg("foo").run().stdout;
    println!("actual: {:?}", actual);
    assert!(actual.contains("foo\n"));

    let actual = scene.ucmd().arg("bar").run().stdout;
    println!("actual: {:?}", actual);
    assert!(actual.contains("foo\n"));
}

#[test]
fn test_file_and_links_zero() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("foo");
    at.symlink_file("foo", "bar");

    let actual = scene.ucmd().arg("foo").arg("-z").run().stdout;
    println!("actual: {:?}", actual);
    assert!(actual.contains("foo"));
    assert!(!actual.contains("\n"));

    let actual = scene.ucmd().arg("bar").arg("-z").run().stdout;
    println!("actual: {:?}", actual);
    assert!(actual.contains("foo"));
    assert!(!actual.contains("\n"));
}

#[test]
fn test_file_and_links_strip() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("foo");
    at.symlink_file("foo", "bar");

    let actual = scene.ucmd().arg("foo").arg("-s").run().stdout;
    println!("actual: {:?}", actual);
    assert!(actual.contains("foo\n"));

    let actual = scene.ucmd().arg("bar").arg("-s").run().stdout;
    println!("actual: {:?}", actual);
    assert!(actual.contains("bar\n"));
}

#[test]
fn test_file_and_links_strip_zero() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("foo");
    at.symlink_file("foo", "bar");

    let actual = scene.ucmd().arg("foo").arg("-s").arg("-z").run().stdout;
    println!("actual: {:?}", actual);
    assert!(actual.contains("foo"));
    assert!(!actual.contains("\n"));

    let actual = scene.ucmd().arg("bar").arg("-s").arg("-z").run().stdout;
    println!("actual: {:?}", actual);
    assert!(actual.contains("bar"));
    assert!(!actual.contains("\n"));
}
