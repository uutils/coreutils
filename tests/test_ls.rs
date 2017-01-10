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
