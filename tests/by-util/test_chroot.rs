// spell-checker:ignore (words) araba newroot userspec

use crate::common::util::*;

#[test]
fn test_missing_operand() {
    let result = new_ucmd!().run();

    assert!(result
        .stderr_str()
        .starts_with("error: The following required arguments were not provided"));

    assert!(result.stderr_str().contains("<newroot>"));
}

#[test]
#[cfg(not(target_os = "android"))]
fn test_enter_chroot_fails() {
    // NOTE: since #2689 this test also ensures that we don't regress #2687
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("jail");

    let result = ucmd.arg("jail").fails();

    assert!(result
        .stderr_str()
        .starts_with("chroot: cannot chroot to 'jail': Operation not permitted (os error 1)"));
}

#[test]
fn test_no_such_directory() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch(&at.plus_as_string("a"));

    ucmd.arg("a")
        .fails()
        .stderr_is("chroot: cannot change root directory to 'a': no such directory");
}

#[test]
fn test_invalid_user_spec() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("a");

    let result = ucmd.arg("a").arg("--userspec=ARABA:").fails();

    assert!(result.stderr_str().starts_with("chroot: invalid userspec"));
}

#[test]
fn test_preference_of_userspec() {
    let scene = TestScenario::new(util_name!());
    let result = scene.cmd("whoami").run();
    if is_ci() && result.stderr_str().contains("No such user/group") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    println!("result.stdout = {}", result.stdout_str());
    println!("result.stderr = {}", result.stderr_str());
    let username = result.stdout_str().trim_end();

    let ts = TestScenario::new("id");
    let result = ts.cmd("id").arg("-g").arg("-n").run();
    println!("result.stdout = {}", result.stdout_str());
    println!("result.stderr = {}", result.stderr_str());

    if is_ci() && result.stderr_str().contains("cannot find name for user ID") {
        // In the CI, some server are failing to return id.
        // As seems to be a configuration issue, ignoring it
        return;
    }

    let group_name = result.stdout_str().trim_end();
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("a");

    let result = ucmd
        .arg("a")
        .arg("--user")
        .arg("fake")
        .arg("-G")
        .arg("ABC,DEF")
        .arg(format!("--userspec={}:{}", username, group_name))
        .run();

    println!("result.stdout = {}", result.stdout_str());
    println!("result.stderr = {}", result.stderr_str());
}

#[test]
fn test_default_shell() {
    // NOTE: This test intends to trigger code which can only be reached with root permissions.
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    let dir = "CHROOT_DIR";
    at.mkdir(dir);

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let _expected = format!(
        "chroot: failed to run command '{}': No such file or directory",
        shell
    );

    // TODO: [2021-09; jhscheer] uncomment if/when #2692 gets merged
    // if let Ok(result) = run_ucmd_as_root(&ts, &[dir]) {
    //     result.stderr_contains(expected);
    // } else {
    //     print!("TEST SKIPPED");
    // }
}
