use common::util::*;

#[test]
fn test_ls_ls() {
    new_ucmd!().succeeds();
}

#[test]
fn test_ls_ls_i() {
    new_ucmd!().arg("-i").succeeds();
    new_ucmd!().arg("-il").succeeds();
}

#[test]
fn test_ls_ls_color() {
    new_ucmd!().arg("--color").succeeds();
    new_ucmd!().arg("--color=always").succeeds();
    new_ucmd!().arg("--color=never").succeeds();
}
