// spell-checker:ignore regfile
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

#[test]
fn test_symlink_to_itself_verbose() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.relative_symlink_file("a", "a");
    ucmd.args(&["-ev", "a"])
        .fails()
        .code_is(1)
        .stderr_contains("Too many levels of symbolic links");
}

#[test]
fn test_trailing_slash_regular_file() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("regfile");
    scene
        .ucmd()
        .args(&["-ev", "./regfile/"])
        .fails()
        .code_is(1)
        .stderr_contains("Not a directory")
        .no_stdout();
    scene
        .ucmd()
        .args(&["-e", "./regfile"])
        .succeeds()
        .stdout_contains("regfile");
}

#[test]
fn test_trailing_slash_symlink_to_regular_file() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("regfile");
    at.relative_symlink_file("regfile", "link");
    scene
        .ucmd()
        .args(&["-ev", "./link/"])
        .fails()
        .code_is(1)
        .stderr_contains("Not a directory")
        .no_stdout();
    scene
        .ucmd()
        .args(&["-e", "./link"])
        .succeeds()
        .stdout_contains("regfile");
    scene
        .ucmd()
        .args(&["-ev", "./link/more"])
        .fails()
        .code_is(1)
        .stderr_contains("Not a directory")
        .no_stdout();
}

#[test]
fn test_trailing_slash_directory() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("directory");
    for query in ["./directory", "./directory/"] {
        scene
            .ucmd()
            .args(&["-e", query])
            .succeeds()
            .stdout_contains("directory");
    }
}

#[test]
fn test_trailing_slash_symlink_to_directory() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("directory");
    at.relative_symlink_dir("directory", "link");
    for query in ["./link", "./link/"] {
        scene
            .ucmd()
            .args(&["-e", query])
            .succeeds()
            .stdout_contains("directory");
    }
    scene
        .ucmd()
        .args(&["-ev", "./link/more"])
        .fails()
        .code_is(1)
        .stderr_contains("No such file or directory");
}

#[test]
fn test_trailing_slash_symlink_to_missing() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("subdir");
    at.relative_symlink_file("missing", "link");
    at.relative_symlink_file("subdir/missing", "link2");
    for query in [
        "missing",
        "./missing/",
        "link",
        "./link/",
        "link/more",
        "link2",
        "./link2/",
        "link2/more",
    ] {
        scene
            .ucmd()
            .args(&["-ev", query])
            .fails()
            .code_is(1)
            .stderr_contains("No such file or directory");
    }
}

#[test]
fn test_canonicalize_trailing_slash_regfile() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("regfile");
    at.relative_symlink_file("regfile", "link1");
    for name in ["regfile", "link1"] {
        scene
            .ucmd()
            .args(&["-f", name])
            .succeeds()
            .stdout_contains("regfile");
        scene
            .ucmd()
            .args(&["-fv", &format!("./{}/", name)])
            .fails()
            .code_is(1)
            .stderr_contains("Not a directory");
        scene
            .ucmd()
            .args(&["-fv", &format!("{}/more", name)])
            .fails()
            .code_is(1)
            .stderr_contains("Not a directory");
        scene
            .ucmd()
            .args(&["-fv", &format!("./{}/more/", name)])
            .fails()
            .code_is(1)
            .stderr_contains("Not a directory");
    }
}

#[test]
fn test_canonicalize_trailing_slash_subdir() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("subdir");
    at.relative_symlink_dir("subdir", "link2");
    for name in ["subdir", "link2"] {
        scene
            .ucmd()
            .args(&["-f", name])
            .succeeds()
            .stdout_contains("subdir");
        scene
            .ucmd()
            .args(&["-f", &format!("./{}/", name)])
            .succeeds()
            .stdout_contains("subdir");
        scene
            .ucmd()
            .args(&["-f", &format!("{}/more", name)])
            .succeeds()
            .stdout_contains(path_concat!("subdir", "more"));
        scene
            .ucmd()
            .args(&["-f", &format!("./{}/more/", name)])
            .succeeds()
            .stdout_contains(path_concat!("subdir", "more"));
        scene
            .ucmd()
            .args(&["-f", &format!("{}/more/more2", name)])
            .fails()
            .code_is(1);
        scene
            .ucmd()
            .args(&["-f", &format!("./{}/more/more2/", name)])
            .fails()
            .code_is(1);
    }
}

#[test]
fn test_canonicalize_trailing_slash_missing() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.relative_symlink_file("missing", "link3");
    for name in ["missing", "link3"] {
        scene
            .ucmd()
            .args(&["-f", name])
            .succeeds()
            .stdout_contains("missing");
        scene
            .ucmd()
            .args(&["-f", &format!("./{}/", name)])
            .succeeds()
            .stdout_contains("missing");
        scene
            .ucmd()
            .args(&["-f", &format!("{}/more", name)])
            .fails()
            .code_is(1);
        scene
            .ucmd()
            .args(&["-f", &format!("./{}/more/", name)])
            .fails()
            .code_is(1);
    }
}

#[test]
fn test_canonicalize_trailing_slash_subdir_missing() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("subdir");
    at.relative_symlink_file("subdir/missing", "link4");
    let name = "link4";
    scene
        .ucmd()
        .args(&["-f", name])
        .succeeds()
        .stdout_contains(path_concat!("subdir", "missing"));
    scene
        .ucmd()
        .args(&["-f", &format!("./{}/", name)])
        .succeeds()
        .stdout_contains(path_concat!("subdir", "missing"));
    scene
        .ucmd()
        .args(&["-f", &format!("{}/more", name)])
        .fails()
        .code_is(1);
    scene
        .ucmd()
        .args(&["-f", &format!("./{}/more/", name)])
        .fails()
        .code_is(1);
}

#[test]
fn test_canonicalize_trailing_slash_symlink_loop() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.relative_symlink_file("link5", "link5");
    for name in ["link5"] {
        scene.ucmd().args(&["-f", name]).fails().code_is(1);
        scene
            .ucmd()
            .args(&["-f", &format!("./{}/", name)])
            .fails()
            .code_is(1);
        scene
            .ucmd()
            .args(&["-f", &format!("{}/more", name)])
            .fails()
            .code_is(1);
        scene
            .ucmd()
            .args(&["-f", &format!("./{}/more/", name)])
            .fails()
            .code_is(1);
    }
}
