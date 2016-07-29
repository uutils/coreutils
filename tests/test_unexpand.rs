use common::util::*;

static UTIL_NAME: &'static str = "unexpand";
fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[test]
fn unexpand_init_0() {
    let result = new_ucmd()
        .args(&["-t4"]).run_piped_stdin(" 1\n  2\n   3\n    4\n");
    assert_eq!(result.stdout, " 1\n  2\n   3\n\t4\n");
}

#[test]
fn unexpand_init_1() {
    let result = new_ucmd()
        .args(&["-t4"]).run_piped_stdin("     5\n      6\n       7\n        8\n");
    assert_eq!(result.stdout, "\t 5\n\t  6\n\t   7\n\t\t8\n");
}

#[test]
fn unexpand_init_list_0() {
    let result = new_ucmd()
        .args(&["-t2,4"]).run_piped_stdin(" 1\n  2\n   3\n    4\n");
    assert_eq!(result.stdout, " 1\n\t2\n\t 3\n\t\t4\n");
}

#[test]
fn unexpand_init_list_1() {
    // Once the list is exhausted, spaces are not converted anymore
    let result = new_ucmd()
        .args(&["-t2,4"]).run_piped_stdin("     5\n      6\n       7\n        8\n");
    assert_eq!(result.stdout, "\t\t 5\n\t\t  6\n\t\t   7\n\t\t    8\n");
}

#[test]
fn unexpand_aflag_0() {
    let result = new_ucmd()
        .args(&["--"]).run_piped_stdin("e     E\nf      F\ng       G\nh        H\n");
    assert_eq!(result.stdout, "e     E\nf      F\ng       G\nh        H\n");
}

#[test]
fn unexpand_aflag_1() {
    let result = new_ucmd()
        .args(&["-a"]).run_piped_stdin("e     E\nf      F\ng       G\nh        H\n");
    assert_eq!(result.stdout, "e     E\nf      F\ng\tG\nh\t H\n");
}

#[test]
fn unexpand_aflag_2() {
    let result = new_ucmd()
        .args(&["-t8"]).run_piped_stdin("e     E\nf      F\ng       G\nh        H\n");
    assert_eq!(result.stdout, "e     E\nf      F\ng\tG\nh\t H\n");
}

#[test]
fn unexpand_first_only_0() {
    let result = new_ucmd()
        .args(&["-t3"]).run_piped_stdin("        A     B");
    assert_eq!(result.stdout, "\t\t  A\t  B");
}

#[test]
fn unexpand_first_only_1() {
    let result = new_ucmd()
        .args(&["-t3", "--first-only"]).run_piped_stdin("        A     B");
    assert_eq!(result.stdout, "\t\t  A     B");
}

#[test]
fn unexpand_trailing_space_0() {
    // evil
    // Individual spaces before fields starting with non blanks should not be
    // converted, unless they are at the beginning of the line.
    let result = new_ucmd()
        .args(&["-t4"]).run_piped_stdin("123 \t1\n123 1\n123 \n123 ");
    assert_eq!(result.stdout, "123\t\t1\n123 1\n123 \n123 ");
}

#[test]
fn unexpand_trailing_space_1() {
    // super evil
    let result = new_ucmd()
        .args(&["-t1"]).run_piped_stdin(" abc d e  f  g ");
    assert_eq!(result.stdout, "\tabc d e\t\tf\t\tg ");
}

#[test]
fn unexpand_spaces_follow_tabs_0() {
    // The two first spaces can be included into the first tab.
    let result = new_ucmd()
        .run_piped_stdin("  \t\t   A");
    assert_eq!(result.stdout, "\t\t   A");
}

#[test]
fn unexpand_spaces_follow_tabs_1() {
    // evil
    // Explanation of what is going on here:
    //      'a' -> 'a'          // first tabstop (1)
    //    ' \t' -> '\t'         // second tabstop (4)
    //      ' ' -> '\t'         // third tabstop (5)
    // '  B \t' -> '  B \t'     // after the list is exhausted, nothing must change
    let result = new_ucmd()
        .args(&["-t1,4,5"]).run_piped_stdin("a \t   B \t");
    assert_eq!(result.stdout, "a\t\t  B \t");
}

#[test]
fn unexpand_spaces_after_fields() {
    let result = new_ucmd()
        .args(&["-a"]).run_piped_stdin("   \t        A B C D             A\t\n");
    assert_eq!(result.stdout, "\t\tA B C D\t\t    A\t\n");
}
