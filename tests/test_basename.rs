use common::util::*;


#[test]
fn test_directory() {
    new_ucmd!().args(&["/root/alpha/beta/gamma/delta/epsilon/omega/"])
        .succeeds().stdout_only("omega");
}

#[test]
fn test_file() {
    new_ucmd!().args(&["/etc/passwd"]).succeeds().stdout_only("passwd");
}

#[test]
fn test_remove_suffix() {
    new_ucmd!().args(&["/usr/local/bin/reallylongexecutable.exe", ".exe"])
        .succeeds().stdout_only("reallylongexecutable");
}

#[test]
fn test_dont_remove_suffix() {
    new_ucmd!().args(&["/foo/bar/baz", "baz"]).succeeds().stdout_only( "baz");
}

#[cfg_attr(not(feature="test_unimplemented"),ignore)]
#[test]
fn test_multiple_param() {
    for multiple_param in vec!["-a", "--multiple"] {
        let path = "/foo/bar/baz";
        new_ucmd!().args(&[multiple_param, path, path])
            .succeeds().stdout_only("baz\nbaz");
    }
}

#[cfg_attr(not(feature="test_unimplemented"),ignore)]
#[test]
fn test_suffix_param() {
    for suffix_param in vec!["-s", "--suffix"] {
        let path = "/foo/bar/baz.exe";
        new_ucmd!()
            .args(&[suffix_param, ".exe", path, path])
            .succeeds().stdout_only("baz\nbaz");
    }
}

#[cfg_attr(not(feature="test_unimplemented"),ignore)]
#[test]
fn test_zero_param() {
    for zero_param in vec!["-z", "--zero"] {
        let path = "/foo/bar/baz";
        new_ucmd!().args(&[zero_param, "-a", path, path])
            .succeeds().stdout_only("baz\0baz\0");
    }
}


fn expect_error(input: Vec<&str>) {
    assert!(new_ucmd!().args(&input)
                .fails().no_stdout().stderr.len() > 0);
}

#[test]
fn test_invalid_option() {
    let path = "/foo/bar/baz";
    expect_error(vec!["-q", path]);
}

#[test]
fn test_no_args() {
    expect_error(vec![]);
}

#[test]
fn test_too_many_args() {
    expect_error(vec!["a", "b", "c"]);
}
