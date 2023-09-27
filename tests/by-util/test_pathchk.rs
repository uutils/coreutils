// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use crate::common::util::TestScenario;

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
        .no_stdout()
        .stderr_contains(format!("pathchk: limit {} exceeded", libc::PATH_MAX));

    // fail on long filename
    new_ucmd!()
        .args(&[format!(
            "dir/{}",
            "file".repeat(libc::FILENAME_MAX as usize + 1)
        )])
        .fails()
        .no_stdout()
        .stderr_contains(format!("pathchk: limit {} exceeded", libc::FILENAME_MAX));
}

#[test]
fn test_posix_mode() {
    // accept some reasonable default
    new_ucmd!().args(&["-p", "dir/file"]).succeeds().no_stdout();

    // fail on long path
    new_ucmd!()
        .args(&["-p", "dir".repeat(libc::PATH_MAX as usize + 1).as_str()])
        .fails()
        .no_stdout()
        .stderr_contains("pathchk: limit 255 exceeded");

    // fail on long filename
    new_ucmd!()
        .args(&["-p", "dir/123456789012345"])
        .fails()
        .no_stdout()
        .stderr_contains("pathchk: limit 14 exceeded by length 15");

    // fail on non-portable chars
    new_ucmd!()
        .args(&["-p", "dir#/$file"])
        .fails()
        .no_stdout()
        .stderr_contains("pathchk: nonportable character '#'");
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
        .no_stdout()
        .stderr_contains(format!("pathchk: limit {} exceeded", libc::PATH_MAX));

    // fail on long filename
    new_ucmd!()
        .args(&[
            "-P",
            format!("dir/{}", "file".repeat(libc::FILENAME_MAX as usize + 1)).as_str(),
        ])
        .fails()
        .no_stdout()
        .stderr_contains(format!("pathchk: limit {} exceeded", libc::FILENAME_MAX));

    // fail on leading hyphen char
    new_ucmd!()
        .args(&["-P", "dir/-file"])
        .fails()
        .no_stdout()
        .stderr_is("pathchk: leading '-' in a component of file name '-file'\n");

    // fail on empty path
    new_ucmd!()
        .args(&["-P", ""])
        .fails()
        .no_stdout()
        .stderr_is("pathchk: empty file name\n");
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
        .no_stdout()
        .stderr_contains("pathchk: limit 255 exceeded");

    // fail on long filename
    new_ucmd!()
        .args(&["-p", "-P", "dir/123456789012345"])
        .fails()
        .no_stdout()
        .stderr_contains("pathchk: limit 14 exceeded by length 15");

    // fail on non-portable chars
    new_ucmd!()
        .args(&["-p", "-P", "dir#/$file"])
        .fails()
        .no_stdout()
        .stderr_is("pathchk: nonportable character '#' in file name 'dir#'\n");

    // fail on leading hyphen char
    new_ucmd!()
        .args(&["-p", "-P", "dir/-file"])
        .fails()
        .no_stdout()
        .stderr_is("pathchk: leading '-' in a component of file name '-file'\n");

    // fail on empty path
    new_ucmd!()
        .args(&["-p", "-P", ""])
        .fails()
        .no_stdout()
        .stderr_is("pathchk: empty file name\n");
}

#[test]
fn test_args_parsing() {
    // fail on no args
    let empty_args: [String; 0] = [];
    new_ucmd!()
        .args(&empty_args)
        .fails()
        .no_stdout()
        .stderr_contains("pathchk: missing operand");
}
