use crate::common::util::*;

#[test]
fn test_with_tab() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("with-tab.txt").succeeds();
    assert!(result.stdout_str().contains("        "));
    assert!(!result.stdout_str().contains("\t"));
}

#[test]
fn test_with_trailing_tab() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("with-trailing-tab.txt").succeeds();
    assert!(result.stdout_str().contains("with tabs=>  "));
    assert!(!result.stdout_str().contains("\t"));
}

#[test]
fn test_with_trailing_tab_i() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("with-trailing-tab.txt").arg("-i").succeeds();
    assert!(result.stdout_str().contains("        // with tabs=>\t"));
}

#[test]
fn test_with_tab_size() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("with-tab.txt").arg("--tabs=10").succeeds();
    assert!(result.stdout_str().contains("          "));
}

#[test]
fn test_with_space() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("with-spaces.txt").succeeds();
    assert!(result.stdout_str().contains("    return"));
}

#[test]
fn test_with_multiple_files() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("with-spaces.txt").arg("with-tab.txt").succeeds();
    assert!(result.stdout_str().contains("    return"));
    assert!(result.stdout_str().contains("        "));
}
