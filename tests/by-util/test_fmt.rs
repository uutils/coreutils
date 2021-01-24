use crate::common::util::*;

#[test]
fn test_fmt() {
    let result = new_ucmd!().arg("one-word-per-line.txt").run();
    //.stdout_is_fixture("call_graph.expected");
    assert_eq!(
        result.stdout.trim(),
        "this is a file with one word per line"
    );
}

#[test]
fn test_fmt_q() {
    let result = new_ucmd!().arg("-q").arg("one-word-per-line.txt").run();
    //.stdout_is_fixture("call_graph.expected");
    assert_eq!(
        result.stdout.trim(),
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
        result.stderr.trim(),
        "fmt: error: invalid width: '2501': Numerical result out of range"
    );
}
/* #[test]
 Fails for now, see https://github.com/uutils/coreutils/issues/1501
fn test_fmt_w() {
    let result = new_ucmd!()
        .arg("-w")
        .arg("10")
        .arg("one-word-per-line.txt")
        .run();
        //.stdout_is_fixture("call_graph.expected");
    assert_eq!(result.stdout.trim(), "this is a file with one word per line");
}


fmt is pretty broken in general, needs more works to have more tests
 */
