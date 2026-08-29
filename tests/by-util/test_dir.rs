// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use regex::Regex;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

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

#[test]
fn test_invalid_option_exit_code() {
    new_ucmd!().arg("-/").fails().code_is(2);
}

#[test]
fn test_help_shows_dir_not_ls() {
    let result = new_ucmd!().arg("--help").succeeds();
    let output = result.stdout_str();

    // Verify help text contains "dir" in the usage line
    assert!(
        output.contains("dir [OPTION]"),
        "Help should show 'dir [OPTION]'"
    );

    // Verify help text does not incorrectly show "ls"
    assert!(
        !output.contains("ls [OPTION]"),
        "Help should not show 'ls [OPTION]'"
    );
}

#[test]
fn test_version() {
    new_ucmd!()
        .arg("--version")
        .succeeds()
        .no_stderr()
        .stdout_is(format!("dir {}\n", uucore::crate_version!()));
}

#[test]
#[cfg(target_os = "linux")]
fn test_write_error() {
    let scene = TestScenario::new(util_name!());
    scene.fixtures.touch("file");

    scene
        .ucmd()
        .arg("file")
        .set_stdout(std::fs::File::create("/dev/full").unwrap())
        .fails()
        .stderr_is("dir: write error: No space left on device\n");
}

#[cfg(all(feature = "feat_diagnostics", not(wasi_runner)))]
mod diagnostics {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn test_snippet_points_at_the_unknown_unit_of_block_size() {
        let result = new_ucmd!()
            .terminal_sim_stderr()
            .arg("--block-size=1fb")
            .fails_with_code(2);
        let stderr = result.stderr_as_displayed();

        // The report names the utility as it was called, not `ls`.
        assert!(stderr.contains("dir:1:"), "{stderr}");
        assert!(stderr.contains("not a known unit"), "{stderr}");
    }

    #[test]
    fn test_plain_message_when_stderr_is_a_pipe() {
        new_ucmd!()
            .arg("--block-size=1fb")
            .fails_with_code(2)
            .stderr_is("dir: invalid --block-size argument '1fb'\n");
    }
}
