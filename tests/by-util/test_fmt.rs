use crate::common::util::*;

#[test]
fn test_fmt() {
    let result = new_ucmd!().arg("one-word-per-line.txt").run();
    //.stdout_is_fixture("call_graph.expected");
    assert_eq!(
        result.stdout_str().trim(),
        "this is a file with one word per line"
    );
}

#[test]
fn test_fmt_q() {
    let result = new_ucmd!().arg("-q").arg("one-word-per-line.txt").run();
    //.stdout_is_fixture("call_graph.expected");
    assert_eq!(
        result.stdout_str().trim(),
        "this is a file with one word per line"
    );
}

#[test]
fn test_fmt_w_too_big() {
    let result = new_ucmd!()
        .arg("-w")
        .arg("2501")
        .arg("one-word-per-line.txt")
        .run();
    //.stdout_is_fixture("call_graph.expected");
    assert_eq!(
        result.stderr_str().trim(),
        "fmt: invalid width: '2501': Numerical result out of range"
    );
}
#[test]
fn test_fmt_w() {
    let result = new_ucmd!()
        .arg("-w")
        .arg("10")
        .arg("one-word-per-line.txt")
        .run();
    //.stdout_is_fixture("call_graph.expected");
    assert_eq!(
        result.stdout_str().trim(),
        "this is\na file\nwith one\nword per\nline"
    );
}
