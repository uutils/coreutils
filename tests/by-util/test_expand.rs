use crate::common::util::*;

#[test]
fn test_with_tab() {
    new_ucmd!()
        .arg("with-tab.txt")
        .succeeds()
        .stdout_contains("        ")
        .stdout_does_not_contain("\t");
}

#[test]
fn test_with_trailing_tab() {
    new_ucmd!()
        .arg("with-trailing-tab.txt")
        .succeeds()
        .stdout_contains("with tabs=>  ")
        .stdout_does_not_contain("\t");
}

#[test]
fn test_with_trailing_tab_i() {
    new_ucmd!()
        .arg("with-trailing-tab.txt")
        .arg("-i")
        .succeeds()
        .stdout_contains("        // with tabs=>\t");
}

#[test]
fn test_with_tab_size() {
    new_ucmd!()
        .arg("with-tab.txt")
        .arg("--tabs=10")
        .succeeds()
        .stdout_contains("          ");
}

#[test]
fn test_with_space() {
    new_ucmd!()
        .arg("with-spaces.txt")
        .succeeds()
        .stdout_contains("    return");
}

#[test]
fn test_with_multiple_files() {
    new_ucmd!()
        .arg("with-spaces.txt")
        .arg("with-tab.txt")
        .succeeds()
        .stdout_contains("    return")
        .stdout_contains("        ");
}
