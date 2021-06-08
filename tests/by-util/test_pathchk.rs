use crate::common::util::*;

#[test]
fn test_default_mode() {
    // test the default mode

    // accept some reasonable default
    new_ucmd!().args(&["dir/file"]).succeeds().no_stdout();

    // accept non-portable chars
    new_ucmd!().args(&["dir#/$file"]).succeeds().no_stdout();

    // accept empty path
    new_ucmd!().args(&[""]).succeeds().no_stdout();

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
    // test the posix mode

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
    // test the posix special mode

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
    // test the posix special mode

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

#[test]
fn test_args_parsing() {
    // fail on no args
    let empty_args: [String; 0] = [];
    new_ucmd!().args(&empty_args).fails().no_stdout();
}
