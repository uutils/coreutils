use common::util::*;


#[test]
fn gnu_ext_disabled_roff_no_ref() {
    new_ucmd!().args(&["-G", "-R", "input"])
        .succeeds().stdout_only_fixture("gnu_ext_disabled_roff_no_ref.expected");
}

#[test]
fn gnu_ext_disabled_roff_input_ref() {
    new_ucmd!().args(&["-G", "-r", "-R", "input"])
        .succeeds().stdout_only_fixture("gnu_ext_disabled_roff_input_ref.expected");
}

#[test]
fn gnu_ext_disabled_roff_auto_ref() {
    new_ucmd!().args(&["-G", "-A", "-R", "input"])
        .succeeds().stdout_only_fixture("gnu_ext_disabled_roff_auto_ref.expected");
}

#[test]
fn gnu_ext_disabled_tex_no_ref() {
    new_ucmd!().args(&["-G", "-T", "-R", "input"])
        .succeeds().stdout_only_fixture("gnu_ext_disabled_tex_no_ref.expected");
}

#[test]
fn gnu_ext_disabled_tex_input_ref() {
    new_ucmd!().args(&["-G", "-T", "-r", "-R", "input"])
        .succeeds().stdout_only_fixture("gnu_ext_disabled_tex_input_ref.expected");
}

#[test]
fn gnu_ext_disabled_tex_auto_ref() {
    new_ucmd!().args(&["-G", "-T", "-A", "-R", "input"])
        .succeeds().stdout_only_fixture("gnu_ext_disabled_tex_auto_ref.expected");
}

#[test]
fn gnu_ext_disabled_ignore_and_only_file() {
    new_ucmd!().args(&["-G", "-o", "only", "-i", "ignore", "input"])
        .succeeds().stdout_only_fixture("gnu_ext_disabled_ignore_and_only_file.expected");
}
