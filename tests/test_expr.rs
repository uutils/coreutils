use common::util::*;


#[test]
fn test_simple_arithmetic() {
    new_ucmd!().args(&["1", "+", "1"]).run().stdout_is("2\n");

    new_ucmd!().args(&["1", "-", "1"]).run().stdout_is("0\n");

    new_ucmd!().args(&["3", "*", "2"]).run().stdout_is("6\n");

    new_ucmd!().args(&["4", "/", "2"]).run().stdout_is("2\n");
}

#[test]
fn test_complex_arithmetic() {
    let run = new_ucmd!().args(&["9223372036854775807", "+", "9223372036854775807"]).run();
    run.stdout_is("");
    run.stderr_is("expr: error: +: Numerical result out of range");

    let run = new_ucmd!().args(&["9", "/", "0"]).run();
    run.stdout_is("");
    run.stderr_is("expr: error: division by zero");
}

#[test]
fn test_parenthesis() {
    new_ucmd!().args(&["(", "1", "+", "1", ")", "*", "2"]).run().stdout_is("4\n");
}

#[test]
fn test_or() {
    new_ucmd!().args(&["0", "|", "foo"]).run().stdout_is("foo\n");

    new_ucmd!().args(&["foo", "|", "bar"]).run().stdout_is("foo\n");
}

#[test]
fn test_and() {
    new_ucmd!().args(&["foo", "&", "1"]).run().stdout_is("foo\n");

    new_ucmd!().args(&["", "&", "1"]).run().stdout_is("0\n");
}
