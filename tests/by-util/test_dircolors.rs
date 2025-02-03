// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore overridable colorterm
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

use dircolors::{guess_syntax, OutputFmt, StrUtils};

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_shell_syntax() {
    use std::env;
    let last = env::var("SHELL");
    env::set_var("SHELL", "/path/csh");
    assert_eq!(OutputFmt::CShell, guess_syntax());
    env::set_var("SHELL", "csh");
    assert_eq!(OutputFmt::CShell, guess_syntax());
    env::set_var("SHELL", "/path/bash");
    assert_eq!(OutputFmt::Shell, guess_syntax());
    env::set_var("SHELL", "bash");
    assert_eq!(OutputFmt::Shell, guess_syntax());
    env::set_var("SHELL", "/asd/bar");
    assert_eq!(OutputFmt::Shell, guess_syntax());
    env::set_var("SHELL", "foo");
    assert_eq!(OutputFmt::Shell, guess_syntax());
    env::set_var("SHELL", "");
    assert_eq!(OutputFmt::Unknown, guess_syntax());
    env::remove_var("SHELL");
    assert_eq!(OutputFmt::Unknown, guess_syntax());

    if let Ok(s) = last {
        env::set_var("SHELL", s);
    }
}

#[test]
fn test_str_utils() {
    let s = "  asd#zcv #hk\t\n  ";
    assert_eq!("asd#zcv", s.purify());

    let s = "con256asd";
    assert!(s.fnmatch("*[2][3-6][5-9]?sd")); // spell-checker:disable-line

    let s = "zxc \t\nqwe jlk    hjl"; // spell-checker:disable-line
    let (k, v) = s.split_two();
    assert_eq!("zxc", k);
    assert_eq!("qwe jlk    hjl", v);
}

#[test]
fn test1() {
    test_helper("test1", "gnome");
}

#[test]
fn test_keywords() {
    test_helper("keywords", "");
}

#[test]
fn test_internal_db() {
    new_ucmd!()
        .arg("-p")
        .run()
        .stdout_is_fixture("internal.expected");
}

#[test]
fn test_ls_colors() {
    new_ucmd!()
        .arg("--print-ls-colors")
        .run()
        .stdout_is_fixture("ls_colors.expected");
}

#[test]
fn test_bash_default() {
    new_ucmd!()
        .env("TERM", "screen")
        .arg("-b")
        .run()
        .stdout_is_fixture("bash_def.expected");
}

#[test]
fn test_csh_default() {
    new_ucmd!()
        .env("TERM", "screen")
        .arg("-c")
        .run()
        .stdout_is_fixture("csh_def.expected");
}
#[test]
fn test_overridable_args() {
    new_ucmd!()
        .env("TERM", "screen")
        .arg("-bc")
        .run()
        .stdout_is_fixture("csh_def.expected");
    new_ucmd!()
        .env("TERM", "screen")
        .arg("-cb")
        .run()
        .stdout_is_fixture("bash_def.expected");
}

#[test]
fn test_no_env() {
    // no SHELL and TERM
    new_ucmd!().fails();
}

#[test]
fn test_exclusive_option() {
    new_ucmd!()
        .arg("-bp")
        .fails()
        .stderr_contains("mutually exclusive");
    new_ucmd!()
        .arg("-cp")
        .fails()
        .stderr_contains("mutually exclusive");
    new_ucmd!()
        .args(&["-b", "--print-ls-colors"])
        .fails()
        .stderr_contains("mutually exclusive");
    new_ucmd!()
        .args(&["-c", "--print-ls-colors"])
        .fails()
        .stderr_contains("mutually exclusive");
    new_ucmd!()
        .args(&["-p", "--print-ls-colors"])
        .fails()
        .stderr_contains("mutually exclusive");
}

#[test]
fn test_stdin() {
    new_ucmd!()
        .pipe_in("owt 40;33\n")
        .args(&["-b", "-"])
        .succeeds()
        .stdout_is("LS_COLORS='tw=40;33:';\nexport LS_COLORS\n")
        .no_stderr();
}

#[test]
fn test_quoting() {
    new_ucmd!()
        .pipe_in("exec 'echo Hello;:'\n")
        .args(&["-b", "-"])
        .succeeds()
        .stdout_is("LS_COLORS='ex='\\''echo Hello;\\:'\\'':';\nexport LS_COLORS\n")
        .no_stderr();
}

/*
#[test]
fn test_print_ls_colors() {
    new_ucmd!()
        .pipe_in("OWT 40;33\n")
        .args(&["--print-ls-colors"])
        .succeeds()
        .stdout_is("\x1B[40;33mtw\t40;33\x1B[0m\n")
        .no_stderr();
}
*/

#[test]
fn test_extra_operand() {
    new_ucmd!()
        .args(&["-c", "file1", "file2"])
        .fails()
        .stderr_contains("dircolors: extra operand 'file2'\n")
        .no_stdout();
}

#[test]
fn test_term_matching() {
    fn check(term_pattern: &str, term: &str, expectation: &str) {
        let theme = format!(
            "
TERM {term_pattern}

.term_matching    00;38;5;61
"
        );

        new_ucmd!()
            .env("TERM", term)
            .pipe_in(theme)
            .args(&["-b", "-"])
            .succeeds()
            .stdout_is(expectation)
            .no_stderr();
    }

    let expectation_if_match = r"
LS_COLORS='*.term_matching=00;38;5;61:';
export LS_COLORS
"
    .trim_start();
    let expectation_if_no_match = r"
LS_COLORS='';
export LS_COLORS
"
    .trim_start();

    // sanity checks
    check("matches", "matches", expectation_if_match);
    check("matches", "no_match", expectation_if_no_match);
    // character set negation should treat ^ like !
    check("[!a]_negation", "a_negation", expectation_if_no_match);
    check("[!a]_negation", "b_negation", expectation_if_match);
    check("[^a]_negation", "a_negation", expectation_if_no_match);
    check("[^a]_negation", "b_negation", expectation_if_match);
}

fn test_helper(file_name: &str, term: &str) {
    new_ucmd!()
        .env("TERM", term)
        .arg("-c")
        .arg(format!("{file_name}.txt"))
        .run()
        .stdout_is_fixture(format!("{file_name}.csh.expected"));

    new_ucmd!()
        .env("TERM", term)
        .arg("-b")
        .arg(format!("{file_name}.txt"))
        .run()
        .stdout_is_fixture(format!("{file_name}.sh.expected"));
}

#[test]
fn test_dircolors_for_dir_as_file() {
    let result = new_ucmd!().args(&["-c", "/"]).fails();
    result.no_stdout();
    assert_eq!(
        result.stderr_str().trim(),
        "dircolors: expected file, got directory '/'",
    );
}

#[test]
fn test_repeated() {
    for arg in ["-b", "-c", "--print-database", "--print-ls-colors"] {
        new_ucmd!().arg(arg).arg(arg).succeeds().no_stderr();
    }
}

#[test]
fn test_colorterm_empty_with_wildcard() {
    new_ucmd!()
        .env("COLORTERM", "")
        .pipe_in("COLORTERM ?*\nowt 40;33\n")
        .args(&["-b", "-"])
        .succeeds()
        .stdout_is("LS_COLORS='';\nexport LS_COLORS\n")
        .no_stderr();
}
