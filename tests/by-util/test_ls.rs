use crate::common::util::*;

#[test]
fn test_ls_ls() {
    new_ucmd!().succeeds();
}

#[test]
fn test_ls_ls_i() {
    new_ucmd!().arg("-i").succeeds();
    new_ucmd!().arg("-il").succeeds();
}

#[test]
fn test_ls_non_existing() {
    new_ucmd!().arg("doesntexist").fails();
}

#[test]
fn test_ls_files_dirs() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("a");
    at.mkdir("a/b");
    at.mkdir("a/b/c");
    at.mkdir("z");
    at.touch(&at.plus_as_string("a/a"));
    at.touch(&at.plus_as_string("a/b/b"));

    scene.ucmd().arg("a").succeeds();
    scene.ucmd().arg("a/a").succeeds();
    scene.ucmd().arg("a").arg("z").succeeds();

    let result = scene.ucmd().arg("doesntexist").fails();
    // Doesn't exist
    assert!(result
        .stderr
        .contains("error: 'doesntexist': No such file or directory"));

    let result = scene.ucmd().arg("a").arg("doesntexist").fails();
    // One exists, the other doesn't
    assert!(result
        .stderr
        .contains("error: 'doesntexist': No such file or directory"));
    assert!(result.stdout.contains("a:"));
}

#[test]
fn test_ls_ls_color() {
    new_ucmd!().arg("--color").succeeds();
    new_ucmd!().arg("--color=always").succeeds();
    new_ucmd!().arg("--color=never").succeeds();
}
