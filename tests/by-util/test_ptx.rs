use crate::common::util::*;

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
