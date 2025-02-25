// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_no_args() {
    new_ucmd!()
        .fails()
        .no_stdout()
        .stderr_contains("pathchk: missing operand");
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_default_mode() {
    // accept some reasonable default
    new_ucmd!().args(&["dir/file"]).succeeds().no_stdout();

    // accept non-portable chars
    new_ucmd!().args(&["dir#/$file"]).succeeds().no_stdout();

    // fail on empty path
    new_ucmd!()
        .args(&[""])
        .fails()
        .stderr_only("pathchk: '': No such file or directory\n");

    new_ucmd!().args(&["", ""]).fails().stderr_only(
        "pathchk: '': No such file or directory\n\
        pathchk: '': No such file or directory\n",
    );

    // fail on long path
    new_ucmd!()
        .args(&["dir".repeat(libc::PATH_MAX as usize + 1)])
        .fails()
        .no_stdout();

    // fail on long filename
    new_ucmd!()
        .args(&[format!(
            "dir/{}",
            "file".repeat(libc::FILENAME_MAX as usize + 1)
        )])
        .fails()
        .no_stdout();
}

#[test]
fn test_posix_mode() {
    // accept some reasonable default
    new_ucmd!().args(&["-p", "dir/file"]).succeeds().no_stdout();

    // fail on long path
    new_ucmd!()
        .args(&["-p", "dir".repeat(libc::PATH_MAX as usize + 1).as_str()])
        .fails()
        .no_stdout();

    // fail on long filename
    new_ucmd!()
        .args(&[
            "-p",
            format!("dir/{}", "file".repeat(libc::FILENAME_MAX as usize + 1)).as_str(),
        ])
        .fails()
        .no_stdout();

    // fail on non-portable chars
    new_ucmd!().args(&["-p", "dir#/$file"]).fails().no_stdout();
}

#[test]
fn test_posix_special() {
    // accept some reasonable default
    new_ucmd!().args(&["-P", "dir/file"]).succeeds().no_stdout();

    // accept non-portable chars
    new_ucmd!()
        .args(&["-P", "dir#/$file"])
        .succeeds()
        .no_stdout();

    // accept non-leading hyphen
    new_ucmd!()
        .args(&["-P", "dir/file-name"])
        .succeeds()
        .no_stdout();

    // fail on long path
    new_ucmd!()
        .args(&["-P", "dir".repeat(libc::PATH_MAX as usize + 1).as_str()])
        .fails()
        .no_stdout();

    // fail on long filename
    new_ucmd!()
        .args(&[
            "-P",
            format!("dir/{}", "file".repeat(libc::FILENAME_MAX as usize + 1)).as_str(),
        ])
        .fails()
        .no_stdout();

    // fail on leading hyphen char
    new_ucmd!().args(&["-P", "dir/-file"]).fails().no_stdout();

    // fail on empty path
    new_ucmd!().args(&["-P", ""]).fails().no_stdout();
}

#[test]
fn test_posix_all() {
    // accept some reasonable default
    new_ucmd!()
        .args(&["-p", "-P", "dir/file"])
        .succeeds()
        .no_stdout();

    // accept non-leading hyphen
    new_ucmd!()
        .args(&["-p", "-P", "dir/file-name"])
        .succeeds()
        .no_stdout();

    // fail on long path
    new_ucmd!()
        .args(&[
            "-p",
            "-P",
            "dir".repeat(libc::PATH_MAX as usize + 1).as_str(),
        ])
        .fails()
        .no_stdout();

    // fail on long filename
    new_ucmd!()
        .args(&[
            "-p",
            "-P",
            format!("dir/{}", "file".repeat(libc::FILENAME_MAX as usize + 1)).as_str(),
        ])
        .fails()
        .no_stdout();

    // fail on non-portable chars
    new_ucmd!()
        .args(&["-p", "-P", "dir#/$file"])
        .fails()
        .no_stdout();

    // fail on leading hyphen char
    new_ucmd!()
        .args(&["-p", "-P", "dir/-file"])
        .fails()
        .no_stdout();

    // fail on empty path
    new_ucmd!().args(&["-p", "-P", ""]).fails().no_stdout();
}
