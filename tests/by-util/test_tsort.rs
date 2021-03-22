use crate::common::util::*;

#[test]
fn test_sort_call_graph() {
    new_ucmd!()
        .arg("call_graph.txt")
        .run()
        .stdout_is_fixture("call_graph.expected");
}

#[test]
fn test_sort_self_loop() {
    new_ucmd!()
        .pipe_in("first first\nfirst second second second")
        .succeeds()
        .stdout_only("first\nsecond\n");
}

#[test]
fn test_no_such_file() {
    let result = new_ucmd!().pipe_in("invalid_file_txt").run();

    assert_eq!(true, result.stdout.contains("No such file or directory"));
}

#[test]
fn test_no_such_file() {
    let version_short = new_ucmd!().arg("-V").run();
    let version_long = new_ucmd!().arg("--versioon").run();

    assert_eq!(version_short.stdout, version_long.stdout);
}

#[test]
fn test_no_such_file() {
    let help_short = new_ucmd!().arg("-h").run();
    let help_long = new_ucmd!().arg("--help").run();

    assert_eq!(help_short.stdout, help_long.stdout);
}

#[test]
fn test_multiple_arguments() {
    let result = new_ucmd!()
        .arg("call_graph.txt")
        .arg("invalid_file.txt")
        .run();

    assert_eq!(true, result.stdout.contains("error: Found argument 'invalid_file.txt' which wasn't expected, or isn't valid in this context"))
}
