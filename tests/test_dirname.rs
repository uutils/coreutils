use common::util::*;


#[test]
fn test_path_with_trailing_slashes() {
    new_ucmd!().arg("/root/alpha/beta/gamma/delta/epsilon/omega//")
        .run().stdout_is("/root/alpha/beta/gamma/delta/epsilon");
}

#[test]
fn test_path_without_trailing_slashes() {
    new_ucmd!().arg("/root/alpha/beta/gamma/delta/epsilon/omega")
        .run().stdout_is("/root/alpha/beta/gamma/delta/epsilon");
}

#[test]
fn test_root() {
    new_ucmd!().arg("/").run().stdout_is("/");
}

#[test]
fn test_pwd() {
    new_ucmd!().arg(".").run().stdout_is(".");
}

#[test]
fn test_empty() {
    new_ucmd!().arg("").run().stdout_is(".");
}
