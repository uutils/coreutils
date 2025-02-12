// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use uutests::new_ucmd;
use uutests::util::TestScenario;
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
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
