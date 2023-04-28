use crate::common::util::TestScenario;
use regex::Regex;

/*
 * As dir use the same functions than ls, we don't have to retest them here.
 * We just test the default and the long output
*/

#[test]
fn test_dir() {
    new_ucmd!().succeeds();
}

#[test]
fn test_default_output() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("some-dir1");
    at.touch("some-file1");

    scene.ucmd().succeeds().stdout_contains("some-file1");

    scene
        .ucmd()
        .succeeds()
        .stdout_does_not_match(&Regex::new("[rwx-]{10}.*some-file1$").unwrap());
}

#[test]
fn test_long_output() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("some-dir1");
    at.touch("some-file1");

    scene
        .ucmd()
        .arg("-l")
        .succeeds()
        .stdout_contains("some-file1");

    scene
        .ucmd()
        .arg("-l")
        .succeeds()
        .stdout_matches(&Regex::new("[rwx-]{10}.*some-file1\n$").unwrap());
}
