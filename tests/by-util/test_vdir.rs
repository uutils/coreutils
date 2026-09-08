// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use regex::Regex;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

/*
 * As vdir use the same functions than ls, we don't have to retest them here.
 * We just test the default and the column output
*/

#[test]
fn test_vdir() {
    new_ucmd!().succeeds();
}

#[test]
fn test_default_output() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("some-dir1");
    at.touch("some-file1");
    filetime::set_file_mtime(
        at.plus("some-file1"),
        filetime::FileTime::from_unix_time(978_307_200, 0),
    )
    .unwrap();

    scene.ucmd().succeeds().stdout_contains("some-file1");

    scene
        .ucmd()
        .succeeds()
        .stdout_contains("Jan  1  2001 some-file1\n");
    scene
        .ucmd()
        .arg("--time-style=long-iso")
        .succeeds()
        .stdout_contains("2001-01-01 00:00 some-file1\n");
    scene
        .ucmd()
        .env("TIME_STYLE", "long-iso")
        .succeeds()
        .stdout_contains("2001-01-01 00:00 some-file1\n");
}

#[test]
fn test_default_format_overrides() {
    let scene = TestScenario::new(util_name!());
    scene.fixtures.touch("file");
    for (args, expected) in [
        (vec!["-1"], "file\n"),
        (vec!["-C"], "file\n"),
        (vec!["-x"], "file\n"),
        (vec!["-m"], "file\n"),
        (vec!["--format=single-column"], "file\n"),
        (vec!["--zero"], "file\0"),
        (vec!["-g", "-C"], "file\n"),
    ] {
        scene
            .ucmd()
            .env("TIME_STYLE", "invalid")
            .args(&args)
            .succeeds()
            .stdout_only(expected);
    }
    for args in [["-l", "-1"], ["-g", "--zero"], ["-C", "-g"]] {
        scene
            .ucmd()
            .args(&args)
            .succeeds()
            .stdout_contains("total 0");
    }
}

#[test]
fn test_column_output() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("some-dir1");
    at.touch("some-file1");

    scene
        .ucmd()
        .arg("-C")
        .succeeds()
        .stdout_contains("some-file1");

    scene
        .ucmd()
        .arg("-C")
        .succeeds()
        .stdout_does_not_match(&Regex::new("[rwx-]{10}.*some-file1$").unwrap());
}

#[test]
fn test_invalid_option_exit_code() {
    new_ucmd!().arg("-/").fails().code_is(2);
}

#[test]
fn test_help_shows_vdir_not_ls() {
    let result = new_ucmd!().arg("--help").succeeds();
    let output = result.stdout_str();

    // Verify help text contains "vdir" in the usage line
    assert!(
        output.contains("vdir [OPTION]"),
        "Help should show 'vdir [OPTION]'"
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
        .stdout_is(format!("vdir {}\n", uucore::crate_version!()));
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
        assert!(stderr.contains("vdir:1:"), "{stderr}");
        assert!(stderr.contains("not a known unit"), "{stderr}");
    }

    #[test]
    fn test_plain_message_when_stderr_is_a_pipe() {
        new_ucmd!()
            .arg("--block-size=1fb")
            .fails_with_code(2)
            .stderr_is("vdir: invalid --block-size argument '1fb'\n");
    }
}
