use common::util::*;

#[test]
fn test_with_tab() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("with-tab.txt").run();
    assert!(result.success);
    assert!(result.stdout.contains("        "));
    assert!(!result.stdout.contains("\t"));
}

#[test]
fn test_with_trailing_tab() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("with-trailing-tab.txt").run();
    assert!(result.success);
    assert!(result.stdout.contains("with tabs=>  "));
    assert!(!result.stdout.contains("\t"));
}

#[test]
fn test_with_trailing_tab_i() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("with-trailing-tab.txt").arg("-i").run();
    assert!(result.success);
    assert!(result.stdout.contains("        // with tabs=>\t"));
}

#[test]
fn test_with_tab_size() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("with-tab.txt").arg("--tabs=10").run();
    assert!(result.success);
    assert!(result.stdout.contains("          "));
}

#[test]
fn test_with_space() {
    let (_, mut ucmd) = at_and_ucmd!();

    let result = ucmd.arg("with-spaces.txt").run();
    assert!(result.success);
    assert!(result.stdout.contains("    return"));
}
