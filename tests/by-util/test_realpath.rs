// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore nusr
use uutests::new_ucmd;
use uutests::path_concat;
use uutests::util::{get_root_path, TestScenario};
use uutests::{at_and_ucmd, util_name};

#[cfg(windows)]
use regex::Regex;
use std::path::{Path, MAIN_SEPARATOR};

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
    let strip_args = ["-s", "--strip", "--no-symlinks"];
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("foo");
    at.symlink_file("foo", "bar");

    for strip_arg in strip_args {
        scene
            .ucmd()
            .arg("foo")
            .arg(strip_arg)
            .succeeds()
            .stdout_contains("foo\n");

        scene
            .ucmd()
            .arg("bar")
            .arg(strip_arg)
            .succeeds()
            .stdout_contains("bar\n");
    }
}

#[test]
fn test_realpath_file_and_links_strip_zero() {
    let strip_args = ["-s", "--strip", "--no-symlinks"];
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    at.touch("foo");
    at.symlink_file("foo", "bar");

    for strip_arg in strip_args {
        scene
            .ucmd()
            .arg("foo")
            .arg(strip_arg)
            .arg("-z")
            .succeeds()
            .stdout_contains("foo\u{0}");

        scene
            .ucmd()
            .arg("bar")
            .arg(strip_arg)
            .arg("-z")
            .succeeds()
            .stdout_contains("bar\u{0}");
    }
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
        .succeeds()
        .stdout_contains(at.plus_as_string("nonexistent-file\n"));
}

#[test]
fn test_realpath_loop() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.symlink_file("2", "1");
    at.symlink_file("3", "2");
    at.symlink_file("1", "3");
    ucmd.arg("1")
        .fails()
        .stderr_contains("Too many levels of symbolic links");
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
        .stdout_only(at.plus_as_string(format!("{}\n", at.root_dir_resolved())));
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
    at.relative_symlink_file("../dir2/baz", "dir1/foo3");

    #[cfg(unix)]
    ucmd.arg("dir1/foo1")
        .arg("dir1/foo2")
        .arg("dir1/foo3")
        .run()
        .stdout_contains("/dir2/bar\n")
        .stdout_contains("/dir2/baz\n")
        .stderr_is("realpath: dir1/foo2: No such file or directory\n");

    #[cfg(windows)]
    ucmd.arg("dir1/foo1")
        .arg("dir1/foo2")
        .arg("dir1/foo3")
        .run()
        .stdout_contains("\\dir2\\bar\n")
        .stdout_contains("\\dir2\\baz\n")
        .stderr_is("realpath: dir1/foo2: No such file or directory\n");
}

#[test]
fn test_realpath_when_symlink_part_is_missing() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("dir2");
    at.touch("dir2/bar");

    at.mkdir("dir1");
    at.relative_symlink_file("../dir2/bar", "dir1/foo1");
    at.relative_symlink_file("dir2/bar", "dir1/foo2");
    at.relative_symlink_file("../dir2/baz", "dir1/foo3");
    at.symlink_file("dir3/bar", "dir1/foo4");

    let expect1 = format!("dir2{MAIN_SEPARATOR}bar");
    let expect2 = format!("dir2{MAIN_SEPARATOR}baz");

    ucmd.args(&["dir1/foo1", "dir1/foo2", "dir1/foo3", "dir1/foo4"])
        .run()
        .stdout_contains(expect1 + "\n")
        .stdout_contains(expect2 + "\n")
        .stderr_contains("realpath: dir1/foo2: No such file or directory\n")
        .stderr_contains("realpath: dir1/foo4: No such file or directory\n");
}

#[test]
fn test_relative_existing_require_directories() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("dir1");
    at.touch("dir1/f");
    ucmd.args(&["-e", "--relative-base=.", "--relative-to=dir1/f", "."])
        .fails()
        .code_is(1)
        .stderr_contains("directory");
}

#[test]
fn test_relative_existing_require_directories_2() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("dir1");
    at.touch("dir1/f");
    ucmd.args(&["-e", "--relative-base=.", "--relative-to=dir1", "."])
        .succeeds()
        .stdout_is("..\n");
}

#[test]
fn test_relative_base_not_prefix_of_relative_to() {
    let result = new_ucmd!()
        .args(&[
            "-sm",
            "--relative-base=/usr/local",
            "--relative-to=/usr",
            "/usr",
            "/usr/local",
        ])
        .succeeds();

    #[cfg(windows)]
    result.stdout_matches(&Regex::new(r"^.*:\\usr\n.*:\\usr\\local\n$").unwrap());

    #[cfg(not(windows))]
    result.stdout_is("/usr\n/usr/local\n");
}

#[test]
fn test_relative_string_handling() {
    let result = new_ucmd!()
        .args(&["-m", "--relative-to=prefix", "prefixed/1"])
        .succeeds();
    #[cfg(not(windows))]
    result.stdout_is("../prefixed/1\n");
    #[cfg(windows)]
    result.stdout_is("..\\prefixed\\1\n");

    let result = new_ucmd!()
        .args(&["-m", "--relative-to=prefixed", "prefix/1"])
        .succeeds();
    #[cfg(not(windows))]
    result.stdout_is("../prefix/1\n");
    #[cfg(windows)]
    result.stdout_is("..\\prefix\\1\n");

    new_ucmd!()
        .args(&["-m", "--relative-to=prefixed", "prefixed/1"])
        .succeeds()
        .stdout_is("1\n");
}

#[test]
fn test_relative() {
    let result = new_ucmd!()
        .args(&[
            "-sm",
            "--relative-base=/usr",
            "--relative-to=/usr",
            "/tmp",
            "/usr",
        ])
        .succeeds();
    #[cfg(not(windows))]
    result.stdout_is("/tmp\n.\n");
    #[cfg(windows)]
    result.stdout_matches(&Regex::new(r"^.*:\\tmp\n\.\n$").unwrap());

    new_ucmd!()
        .args(&["-sm", "--relative-base=/", "--relative-to=/", "/", "/usr"])
        .succeeds()
        .stdout_is(".\nusr\n");

    let result = new_ucmd!()
        .args(&["-sm", "--relative-base=/usr", "/tmp", "/usr"])
        .succeeds();
    #[cfg(not(windows))]
    result.stdout_is("/tmp\n.\n");
    #[cfg(windows)]
    result.stdout_matches(&Regex::new(r"^.*:\\tmp\n\.\n$").unwrap());

    new_ucmd!()
        .args(&["-sm", "--relative-base=/", "/", "/usr"])
        .succeeds()
        .stdout_is(".\nusr\n");
}

#[test]
fn test_realpath_trailing_slash() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("file");
    at.mkdir("dir");
    at.relative_symlink_file("file", "link_file");
    at.relative_symlink_dir("dir", "link_dir");
    at.relative_symlink_dir("no_dir", "link_no_dir");
    scene
        .ucmd()
        .arg("link_file")
        .succeeds()
        .stdout_contains(format!("{}file\n", std::path::MAIN_SEPARATOR));
    scene.ucmd().arg("link_file/").fails().code_is(1);
    scene
        .ucmd()
        .arg("link_dir")
        .succeeds()
        .stdout_contains(format!("{}dir\n", std::path::MAIN_SEPARATOR));
    scene
        .ucmd()
        .arg("link_dir/")
        .succeeds()
        .stdout_contains(format!("{}dir\n", std::path::MAIN_SEPARATOR));
    scene
        .ucmd()
        .arg("link_no_dir")
        .succeeds()
        .stdout_contains(format!("{}no_dir\n", std::path::MAIN_SEPARATOR));
    scene
        .ucmd()
        .arg("link_no_dir/")
        .succeeds()
        .stdout_contains(format!("{}no_dir\n", std::path::MAIN_SEPARATOR));
    scene
        .ucmd()
        .args(&["-e", "link_file"])
        .succeeds()
        .stdout_contains(format!("{}file\n", std::path::MAIN_SEPARATOR));
    scene.ucmd().args(&["-e", "link_file/"]).fails().code_is(1);
    scene
        .ucmd()
        .args(&["-e", "link_dir"])
        .succeeds()
        .stdout_contains(format!("{}dir\n", std::path::MAIN_SEPARATOR));
    scene
        .ucmd()
        .args(&["-e", "link_dir/"])
        .succeeds()
        .stdout_contains(format!("{}dir\n", std::path::MAIN_SEPARATOR));
    scene.ucmd().args(&["-e", "link_no_dir"]).fails().code_is(1);
    scene
        .ucmd()
        .args(&["-e", "link_no_dir/"])
        .fails()
        .code_is(1);
    scene
        .ucmd()
        .args(&["-m", "link_file"])
        .succeeds()
        .stdout_contains(format!("{}file\n", std::path::MAIN_SEPARATOR));
    scene
        .ucmd()
        .args(&["-m", "link_file/"])
        .succeeds()
        .stdout_contains(format!("{}file\n", std::path::MAIN_SEPARATOR));
    scene
        .ucmd()
        .args(&["-m", "link_dir"])
        .succeeds()
        .stdout_contains(format!("{}dir\n", std::path::MAIN_SEPARATOR));
    scene
        .ucmd()
        .args(&["-m", "link_dir/"])
        .succeeds()
        .stdout_contains(format!("{}dir\n", std::path::MAIN_SEPARATOR));
    scene
        .ucmd()
        .args(&["-m", "link_no_dir"])
        .succeeds()
        .stdout_contains(format!("{}no_dir\n", std::path::MAIN_SEPARATOR));
    scene
        .ucmd()
        .args(&["-m", "link_no_dir/"])
        .succeeds()
        .stdout_contains(format!("{}no_dir\n", std::path::MAIN_SEPARATOR));
}

#[test]
fn test_realpath_empty() {
    new_ucmd!().fails().code_is(1);
}
