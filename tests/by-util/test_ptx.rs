// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore roff

use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn gnu_ext_disabled_rightward_no_ref() {
    new_ucmd!()
        .args(&["-G", "-R", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_rightward_no_ref.expected");
}

#[test]
fn gnu_ext_disabled_rightward_no_ref_empty_word_regexp() {
    new_ucmd!()
        .args(&["-G", "-R", "-W", "", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_rightward_no_ref.expected");
}

#[test]
fn gnu_ext_disabled_rightward_no_ref_word_regexp_exc_space() {
    new_ucmd!()
        .args(&["-G", "-R", "-W", "[^\t\n]+", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_rightward_no_ref_word_regexp_exc_space.expected");
}

#[test]
fn gnu_ext_disabled_rightward_input_ref() {
    new_ucmd!()
        .args(&["-G", "-r", "-R", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_rightward_input_ref.expected");
}

#[test]
fn gnu_ext_disabled_rightward_auto_ref() {
    new_ucmd!()
        .args(&["-G", "-A", "-R", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_rightward_auto_ref.expected");
}

#[test]
fn gnu_ext_disabled_tex_no_ref() {
    new_ucmd!()
        .args(&["-G", "-T", "-R", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_tex_no_ref.expected");
}

#[test]
fn gnu_ext_disabled_tex_input_ref() {
    new_ucmd!()
        .args(&["-G", "-T", "-r", "-R", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_tex_input_ref.expected");
}

#[test]
fn gnu_ext_disabled_tex_auto_ref() {
    new_ucmd!()
        .args(&["-G", "-T", "-A", "-R", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_tex_auto_ref.expected");
}

#[test]
fn gnu_ext_disabled_ignore_and_only_file() {
    new_ucmd!()
        .args(&["-G", "-o", "only", "-i", "ignore", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_ignore_and_only_file.expected");
}

#[test]
fn gnu_ext_disabled_output_width_50() {
    new_ucmd!()
        .args(&["-G", "-w", "50", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_output_width_50.expected");
}

#[test]
fn gnu_ext_disabled_output_width_70() {
    new_ucmd!()
        .args(&["-G", "-w", "70", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_output_width_70.expected");
}

#[test]
fn gnu_ext_disabled_break_file() {
    new_ucmd!()
        .args(&["-G", "-b", "break_file", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_break_file.expected");
}

#[test]
fn gnu_ext_disabled_empty_word_regexp_ignores_break_file() {
    new_ucmd!()
        .args(&["-G", "-b", "break_file", "-R", "-W", "", "input"])
        .succeeds()
        .stdout_only_fixture("gnu_ext_disabled_rightward_no_ref.expected");
}

#[test]
fn test_reject_too_many_operands() {
    new_ucmd!().args(&["-G", "-", "-", "-"]).fails_with_code(1);
}

#[test]
fn test_break_file_regex_escaping() {
    new_ucmd!()
        .pipe_in("\\.+*?()|[]{}^$#&-~")
        .args(&["-G", "-b", "-", "input"])
        .succeeds()
        .stdout_only_fixture("break_file_regex_escaping.expected");
}

#[test]
fn test_ignore_case() {
    new_ucmd!()
        .args(&["-G", "-f"])
        .pipe_in("a _")
        .succeeds()
        .stdout_only(".xx \"\" \"\" \"a _\" \"\"\n.xx \"\" \"a\" \"_\" \"\"\n");
}

#[test]
fn test_format() {
    new_ucmd!()
        .args(&["-G", "-O"])
        .pipe_in("a")
        .succeeds()
        .stdout_only(".xx \"\" \"\" \"a\" \"\"\n");
    new_ucmd!()
        .args(&["-G", "-T"])
        .pipe_in("a")
        .succeeds()
        .stdout_only("\\xx {}{}{a}{}{}\n");
    new_ucmd!()
        .args(&["-G", "--format=roff"])
        .pipe_in("a")
        .succeeds()
        .stdout_only(".xx \"\" \"\" \"a\" \"\"\n");
    new_ucmd!()
        .args(&["-G", "--format=tex"])
        .pipe_in("a")
        .succeeds()
        .stdout_only("\\xx {}{}{a}{}{}\n");
}

#[cfg(target_os = "linux")]
#[test]
fn test_failed_write_is_reported() {
    new_ucmd!()
        .arg("-G")
        .pipe_in("hello")
        .set_stdout(std::fs::File::create("/dev/full").unwrap())
        .fails()
        .stderr_is("ptx: write failed: No space left on device\n");
}

#[test]
fn test_utf8() {
    new_ucmd!()
        .args(&["-G"])
        .pipe_in("it’s disabled\n")
        .succeeds()
        .stdout_only(".xx \"\" \"it’s\" \"disabled\" \"\"\n.xx \"\" \"\" \"it’s disabled\" \"\"\n");
    new_ucmd!()
        .args(&["-G", "-T"])
        .pipe_in("it’s disabled\n")
        .succeeds()
        .stdout_only("\\xx {}{it’s}{disabled}{}{}\n\\xx {}{}{it’s}{ disabled}{}\n");
}
