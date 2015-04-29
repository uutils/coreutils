use std::io::Write;
use std::process::{Command, Stdio};

static PROGNAME: &'static str = "./unexpand";

fn run(input: &str, args: &[&'static str]) -> Vec<u8> {
    let mut process = Command::new(PROGNAME)
                                   .args(args)
                                   .stdin(Stdio::piped())
                                   .stdout(Stdio::piped())
                                   .spawn()
                                   .unwrap_or_else(|e| panic!("{}", e));

    process.stdin.take().unwrap_or_else(|| panic!("Could not take child process stdin"))
        .write_all(input.as_bytes()).unwrap_or_else(|e| panic!("{}", e));

    let po = process.wait_with_output().unwrap_or_else(|e| panic!("{}", e));
    po.stdout
}

#[test]
fn unexpand_init_0() {
    let out = run(" 1\n  2\n   3\n    4\n", &["-t4"]);
    assert_eq!(&out[..], b" 1\n  2\n   3\n\t4\n" as &[u8]);
}

#[test]
fn unexpand_init_1() {
    let out = run("     5\n      6\n       7\n        8\n", &["-t4"]);
    assert_eq!(&out[..], b"\t 5\n\t  6\n\t   7\n\t\t8\n" as &[u8]);
}

#[test]
fn unexpand_init_list_0() {
    let out = run(" 1\n  2\n   3\n    4\n", &["-t2,4"]);
    assert_eq!(&out[..], b" 1\n\t2\n\t 3\n\t\t4\n" as &[u8]);
}

#[test]
fn unexpand_init_list_1() {
    // Once the list is exhausted, spaces are not converted anymore
    let out = run("     5\n      6\n       7\n        8\n", &["-t2,4"]);
    assert_eq!(&out[..], b"\t\t 5\n\t\t  6\n\t\t   7\n\t\t    8\n" as &[u8]);
}

#[test]
fn unexpand_aflag_0() {
    let out = run("e     E\nf      F\ng       G\nh        H\n", &["--"]);
    assert_eq!(&out[..], b"e     E\nf      F\ng       G\nh        H\n" as &[u8]);
}

#[test]
fn unexpand_aflag_1() {
    let out = run("e     E\nf      F\ng       G\nh        H\n", &["-a"]);
    assert_eq!(&out[..], b"e     E\nf      F\ng\tG\nh\t H\n" as &[u8]);
}

#[test]
fn unexpand_aflag_2() {
    let out = run("e     E\nf      F\ng       G\nh        H\n", &["-t8"]);
    assert_eq!(&out[..], b"e     E\nf      F\ng\tG\nh\t H\n" as &[u8]);
}

#[test]
fn unexpand_first_only_0() {
    let out = run("        A     B", &["-t3"]);
    assert_eq!(&out[..], b"\t\t  A\t  B" as &[u8]);
}

#[test]
fn unexpand_first_only_1() {
    let out = run("        A     B", &["-t3", "--first-only"]);
    assert_eq!(&out[..], b"\t\t  A     B" as &[u8]);
}

#[test]
fn unexpand_trailing_space_0() { // evil
    // Individual spaces before fields starting with non blanks should not be
    // converted, unless they are at the beginning of the line.
    let out = run("123 \t1\n123 1\n123 \n123 ", &["-t4"]);
    assert_eq!(&out[..], b"123\t\t1\n123 1\n123 \n123 " as &[u8]);
}

#[test]
fn unexpand_trailing_space_1() { // super evil
    let out = run(" abc d e  f  g ", &["-t1"]);
    assert_eq!(&out[..], b"\tabc d e\t\tf\t\tg " as &[u8]);
}

#[test]
fn unexpand_spaces_follow_tabs_0() {
    // The two first spaces can be included into the first tab.
    let out = run("  \t\t   A", &[]);
    assert_eq!(&out[..], b"\t\t   A" as &[u8]);
}

#[test]
fn unexpand_spaces_follow_tabs_1() { // evil
    // Explanation of what is going on here:
    //      'a' -> 'a'          // first tabstop (1)
    //    ' \t' -> '\t'         // second tabstop (4)
    //      ' ' -> '\t'         // third tabstop (5)
    // '  B \t' -> '  B \t'     // after the list is exhausted, nothing must change
    let out = run("a \t   B \t", &["-t1,4,5"]);
    assert_eq!(&out[..], b"a\t\t  B \t" as &[u8]);
}


