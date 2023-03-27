use crate::common::util::TestScenario;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(1);
}

#[test]
fn test_path_with_trailing_slashes() {
    new_ucmd!()
        .arg("/root/alpha/beta/gamma/delta/epsilon/omega//")
        .run()
        .stdout_is("/root/alpha/beta/gamma/delta/epsilon\n");
}

#[test]
fn test_path_without_trailing_slashes() {
    new_ucmd!()
        .arg("/root/alpha/beta/gamma/delta/epsilon/omega")
        .run()
        .stdout_is("/root/alpha/beta/gamma/delta/epsilon\n");
}

#[test]
fn test_path_without_trailing_slashes_and_zero() {
    new_ucmd!()
        .arg("-z")
        .arg("/root/alpha/beta/gamma/delta/epsilon/omega")
        .succeeds()
        .stdout_is("/root/alpha/beta/gamma/delta/epsilon\u{0}");

    new_ucmd!()
        .arg("--zero")
        .arg("/root/alpha/beta/gamma/delta/epsilon/omega")
        .succeeds()
        .stdout_is("/root/alpha/beta/gamma/delta/epsilon\u{0}");
}

#[test]
fn test_root() {
    new_ucmd!().arg("/").run().stdout_is("/\n");
}

#[test]
fn test_pwd() {
    new_ucmd!().arg(".").run().stdout_is(".\n");
}

#[test]
fn test_empty() {
    new_ucmd!().arg("").run().stdout_is(".\n");
}
