use common::util::*;


#[test]
fn test_bsd_single_file() {
    new_ucmd!()
        .arg("lorem_ipsum.txt")
        .succeeds().stdout_only_fixture("bsd_single_file.expected");
}

#[test]
fn test_bsd_multiple_files() {
    new_ucmd!()
        .arg("lorem_ipsum.txt")
        .arg("alice_in_wonderland.txt")
        .succeeds().stdout_only_fixture("bsd_multiple_files.expected");
}

#[test]
fn test_bsd_stdin() {
    new_ucmd!()
        .pipe_in_fixture("lorem_ipsum.txt")
        .succeeds().stdout_only_fixture("bsd_stdin.expected");
}

#[test]
fn test_sysv_single_file() {
    new_ucmd!()
        .arg("-s").arg("lorem_ipsum.txt")
        .succeeds().stdout_only_fixture("sysv_single_file.expected");
}

#[test]
fn test_sysv_multiple_files() {
    new_ucmd!()
        .arg("-s")
        .arg("lorem_ipsum.txt")
        .arg("alice_in_wonderland.txt")
        .succeeds().stdout_only_fixture("sysv_multiple_files.expected");
}

#[test]
fn test_sysv_stdin() {
    new_ucmd!()
        .arg("-s")
        .pipe_in_fixture("lorem_ipsum.txt")
        .succeeds().stdout_only_fixture("sysv_stdin.expected");
}
