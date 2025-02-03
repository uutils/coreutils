// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore contenta
use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn unexpand_init_0() {
    new_ucmd!()
        .args(&["-t4"])
        .pipe_in(" 1\n  2\n   3\n    4\n")
        .run()
        .stdout_is(" 1\n  2\n   3\n\t4\n");
}

#[test]
fn unexpand_init_1() {
    new_ucmd!()
        .args(&["-t4"])
        .pipe_in("     5\n      6\n       7\n        8\n")
        .run()
        .stdout_is("\t 5\n\t  6\n\t   7\n\t\t8\n");
}

#[test]
fn unexpand_init_list_0() {
    new_ucmd!()
        .args(&["-t2,4"])
        .pipe_in(" 1\n  2\n   3\n    4\n")
        .run()
        .stdout_is(" 1\n\t2\n\t 3\n\t\t4\n");
}

#[test]
fn unexpand_init_list_1() {
    // Once the list is exhausted, spaces are not converted anymore
    new_ucmd!()
        .args(&["-t2,4"])
        .pipe_in("     5\n      6\n       7\n        8\n")
        .run()
        .stdout_is("\t\t 5\n\t\t  6\n\t\t   7\n\t\t    8\n");
}

#[test]
fn unexpand_flag_a_0() {
    new_ucmd!()
        .args(&["--"])
        .pipe_in("e     E\nf      F\ng       G\nh        H\n")
        .run()
        .stdout_is("e     E\nf      F\ng       G\nh        H\n");
}

#[test]
fn unexpand_flag_a_1() {
    new_ucmd!()
        .args(&["-a"])
        .pipe_in("e     E\nf      F\ng       G\nh        H\n")
        .run()
        .stdout_is("e     E\nf      F\ng\tG\nh\t H\n");
}

#[test]
fn unexpand_flag_a_2() {
    new_ucmd!()
        .args(&["-t8"])
        .pipe_in("e     E\nf      F\ng       G\nh        H\n")
        .run()
        .stdout_is("e     E\nf      F\ng\tG\nh\t H\n");
}

#[test]
fn unexpand_first_only_0() {
    new_ucmd!()
        .args(&["-t3"])
        .pipe_in("        A     B")
        .run()
        .stdout_is("\t\t  A\t  B");
}

#[test]
fn unexpand_first_only_1() {
    new_ucmd!()
        .args(&["-t3", "--first-only"])
        .pipe_in("        A     B")
        .run()
        .stdout_is("\t\t  A     B");
}

#[test]
fn unexpand_trailing_space_0() {
    // evil
    // Individual spaces before fields starting with non blanks should not be
    // converted, unless they are at the beginning of the line.
    new_ucmd!()
        .args(&["-t4"])
        .pipe_in("123 \t1\n123 1\n123 \n123 ")
        .run()
        .stdout_is("123\t\t1\n123 1\n123 \n123 ");
}

#[test]
fn unexpand_trailing_space_1() {
    // super evil
    new_ucmd!()
        .args(&["-t1"])
        .pipe_in(" abc d e  f  g ")
        .run()
        .stdout_is("\tabc d e\t\tf\t\tg ");
}

#[test]
fn unexpand_spaces_follow_tabs_0() {
    // The two first spaces can be included into the first tab.
    new_ucmd!()
        .pipe_in("  \t\t   A")
        .run()
        .stdout_is("\t\t   A");
}

#[test]
fn unexpand_spaces_follow_tabs_1() {
    // evil
    // Explanation of what is going on here:
    //      'a' -> 'a'          // first tabstop (1)
    //    ' \t' -> '\t'         // second tabstop (4)
    //      ' ' -> '\t'         // third tabstop (5)
    // '  B \t' -> '  B \t'     // after the list is exhausted, nothing must change
    new_ucmd!()
        .args(&["-t1,4,5"])
        .pipe_in("a \t   B \t")
        .run()
        .stdout_is("a\t\t  B \t");
}

#[test]
fn unexpand_spaces_after_fields() {
    new_ucmd!()
        .args(&["-a"])
        .pipe_in("   \t        A B C D             A\t\n")
        .run()
        .stdout_is("\t\tA B C D\t\t    A\t\n");
}

#[test]
fn unexpand_read_from_file() {
    new_ucmd!()
        .arg("with_spaces.txt")
        .arg("-t4")
        .run()
        .success();
}

#[test]
fn unexpand_read_from_two_file() {
    new_ucmd!()
        .arg("with_spaces.txt")
        .arg("with_spaces.txt")
        .arg("-t4")
        .run()
        .success();
}

#[test]
fn test_tabs_shortcut() {
    new_ucmd!()
        .arg("-3")
        .pipe_in("   a   b")
        .run()
        .stdout_is("\ta   b");
}

#[test]
fn test_tabs_shortcut_combined_with_all_arg() {
    fn run_cmd(all_arg: &str) {
        new_ucmd!()
            .args(&[all_arg, "-3"])
            .pipe_in("a  b  c")
            .run()
            .stdout_is("a\tb\tc");
    }

    let all_args = vec!["-a", "--all"];

    for arg in all_args {
        run_cmd(arg);
    }
}

#[test]
fn test_comma_separated_tabs_shortcut() {
    new_ucmd!()
        .args(&["-a", "-3,9"])
        .pipe_in("a  b     c")
        .run()
        .stdout_is("a\tb\tc");
}

#[test]
fn test_tabs_cannot_be_zero() {
    new_ucmd!()
        .arg("--tabs=0")
        .fails()
        .stderr_contains("tab size cannot be 0");
}

#[test]
fn test_tabs_must_be_ascending() {
    new_ucmd!()
        .arg("--tabs=1,1")
        .fails()
        .stderr_contains("tab sizes must be ascending");
}

#[test]
fn test_tabs_with_invalid_chars() {
    new_ucmd!()
        .arg("--tabs=x")
        .fails()
        .stderr_contains("tab size contains invalid character(s): 'x'");
    new_ucmd!()
        .arg("--tabs=1x2")
        .fails()
        .stderr_contains("tab size contains invalid character(s): 'x2'");
}

#[test]
fn test_tabs_shortcut_with_too_large_size() {
    let arg = format!("-{}", u128::MAX);
    let expected_error = "tab stop value is too large";

    new_ucmd!().arg(arg).fails().stderr_contains(expected_error);
}

#[test]
fn test_is_directory() {
    let (at, mut ucmd) = at_and_ucmd!();
    let dir_name = "dir";
    at.mkdir(dir_name);

    ucmd.arg(dir_name)
        .fails()
        .stderr_contains(format!("unexpand: {dir_name}: Is a directory"));
}

#[test]
fn test_multiple_files() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.write("file", "content");
    at.write("file1", "a        b");

    ucmd.args(&["file", "file1"])
        .succeeds()
        .stdout_is("contenta        b");
}

#[test]
fn test_one_nonexisting_file() {
    new_ucmd!()
        .arg("asdf.txt")
        .fails()
        .stderr_contains("asdf.txt: No such file or directory");
}
