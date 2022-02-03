use crate::common::util::*;

use std::path::Path;

static GIBBERISH: &str = "supercalifragilisticexpialidocious";

#[test]
fn test_realpath_current_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    let expect = at.root_dir_resolved() + "\n";
    ucmd.arg(".").succeeds().stdout_is(expect);
}

#[test]
fn test_realpath_long_redirection_to_current_dir() {
    let (at, mut ucmd) = at_and_ucmd!();
    // Create a 256-character path to current directory
    let dir = path_concat!(".", ..128);
    let expect = at.root_dir_resolved() + "\n";
    ucmd.arg(dir).succeeds().stdout_is(expect);
}

#[test]
fn test_realpath_long_redirection_to_root() {
    // Create a 255-character path to root
    let dir = path_concat!("..", ..85);
    let expect = get_root_path().to_owned() + "\n";
    new_ucmd!().arg(dir).succeeds().stdout_is(expect);
}

#[test]
fn test_realpath_file_and_links() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("foo");
    at.symlink_file("foo", "bar");

    scene.ucmd().arg("foo").succeeds().stdout_contains("foo\n");
    scene.ucmd().arg("bar").succeeds().stdout_contains("foo\n");
}

#[test]
fn test_realpath_file_and_links_zero() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("foo");
    at.symlink_file("foo", "bar");

    scene
        .ucmd()
        .arg("foo")
        .arg("-z")
        .succeeds()
        .stdout_contains("foo\u{0}");

    scene
        .ucmd()
        .arg("bar")
        .arg("-z")
        .succeeds()
        .stdout_contains("foo\u{0}");
}

#[test]
fn test_realpath_file_and_links_strip() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("foo");
    at.symlink_file("foo", "bar");

    scene
        .ucmd()
        .arg("foo")
        .arg("-s")
        .succeeds()
        .stdout_contains("foo\n");

    scene
        .ucmd()
        .arg("bar")
        .arg("-s")
        .succeeds()
        .stdout_contains("bar\n");
}

#[test]
fn test_realpath_file_and_links_strip_zero() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("foo");
    at.symlink_file("foo", "bar");

    scene
        .ucmd()
        .arg("foo")
        .arg("-s")
        .arg("-z")
        .succeeds()
        .stdout_contains("foo\u{0}");

    scene
        .ucmd()
        .arg("bar")
        .arg("-s")
        .arg("-z")
        .succeeds()
        .stdout_contains("bar\u{0}");
}

#[test]
fn test_realpath_physical_mode() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.mkdir("dir1");
    at.mkdir_all("dir2/bar");
    at.symlink_dir("dir2/bar", "dir1/foo");

    scene
        .ucmd()
        .arg("dir1/foo/..")
        .succeeds()
        .stdout_contains("dir2\n");
}

#[test]
fn test_realpath_logical_mode() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.mkdir("dir1");
    at.mkdir("dir2");
    at.symlink_dir("dir2", "dir1/foo");

    scene
        .ucmd()
        .arg("-L")
        .arg("dir1/foo/..")
        .succeeds()
        .stdout_contains("dir1\n");
}

#[test]
fn test_realpath_dangling() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.symlink_file("nonexistent-file", "link");
    ucmd.arg("link")
        .fails()
        .stderr_contains("realpath: link: No such file or directory");
}

#[test]
fn test_realpath_loop() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.symlink_file("2", "1");
    at.symlink_file("3", "2");
    at.symlink_file("1", "3");
    ucmd.arg("1")
        .succeeds()
        .stdout_only(at.plus_as_string("2\n"));
}

#[test]
fn test_realpath_default_allows_final_non_existent() {
    let p = Path::new("").join(GIBBERISH);
    let (at, mut ucmd) = at_and_ucmd!();
    let expect = path_concat!(at.root_dir_resolved(), p.to_str().unwrap()) + "\n";
    ucmd.arg(p.as_os_str()).succeeds().stdout_only(expect);
}

#[test]
fn test_realpath_default_forbids_non_final_non_existent() {
    let p = Path::new("").join(GIBBERISH).join(GIBBERISH);
    new_ucmd!().arg(p.to_str().unwrap()).fails();
}

#[test]
fn test_realpath_existing() {
    let (at, mut ucmd) = at_and_ucmd!();
    ucmd.arg("-e")
        .arg(".")
        .succeeds()
        .stdout_only(at.plus_as_string(&format!("{}\n", at.root_dir_resolved())));
}

#[test]
fn test_realpath_existing_error() {
    new_ucmd!().arg("-e").arg(GIBBERISH).fails();
}

#[test]
fn test_realpath_missing() {
    let p = Path::new("").join(GIBBERISH).join(GIBBERISH);
    let (at, mut ucmd) = at_and_ucmd!();
    let expect = path_concat!(at.root_dir_resolved(), p.to_str().unwrap()) + "\n";
    ucmd.arg("-m")
        .arg(p.as_os_str())
        .succeeds()
        .stdout_only(expect);
}

#[test]
fn test_realpath_when_symlink_is_absolute_and_enoent() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("dir2");
    at.touch("dir2/bar");

    at.mkdir("dir1");
    at.symlink_file("dir2/bar", "dir1/foo1");
    at.symlink_file("/dir2/bar", "dir1/foo2");
    at.relative_symlink_file("dir2/baz", at.plus("dir1/foo3").to_str().unwrap());

    #[cfg(unix)]
    ucmd.arg("dir1/foo1")
        .arg("dir1/foo2")
        .arg("dir1/foo3")
        .run()
        .stdout_contains("/dir2/bar\n")
        .stdout_contains("/dir2/baz\n")
        .stderr_is("realpath: dir1/foo2: No such file or directory");

    #[cfg(windows)]
    ucmd.arg("dir1/foo1")
        .arg("dir1/foo2")
        .arg("dir1/foo3")
        .run()
        .stdout_contains("\\dir2\\bar\n")
        .stdout_contains("\\dir2\\baz\n")
        .stderr_is("realpath: dir1/foo2: No such file or directory");
}
