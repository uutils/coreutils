use crate::common::util::*;

#[test]
#[cfg(not(windows))]
fn test_dev_null() {
    new_ucmd!()
        .pipe_in("</dev/null")
        .fails()
        .stdout_is("not a tty\n");
}

#[test]
#[cfg(not(windows))]
fn test_dev_null_silent() {
    new_ucmd!()
        .args(&["-s"])
        .pipe_in("</dev/null")
        .fails()
        .stdout_is("");
}

#[test]
fn test_close_stdin() {
    new_ucmd!().pipe_in("<&-").fails().stdout_is("not a tty\n");
}

#[test]
fn test_close_stdin_silent() {
    new_ucmd!()
        .args(&["-s"])
        .pipe_in("<&-")
        .fails()
        .stdout_is("");
}

#[test]
fn test_close_stdin_silent_long() {
    new_ucmd!()
        .args(&["--silent"])
        .pipe_in("<&-")
        .fails()
        .stdout_is("");
}

#[test]
fn test_close_stdin_silent_alias() {
    new_ucmd!()
        .args(&["--quiet"])
        .pipe_in("<&-")
        .fails()
        .stdout_is("");
}

#[test]
fn test_wrong_argument() {
    new_ucmd!().args(&["a"]).fails();
}
