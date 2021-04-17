use crate::common::util::*;

fn return_whoami_username() -> String {
    let scene = TestScenario::new("whoami");
    let result = scene.cmd("whoami").run();
    if is_ci() && result.stderr.contains("cannot find name for user ID") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return String::from("");
    }

    result.stdout_str().trim().to_string()
}

#[test]
fn test_id() {
    let result = new_ucmd!().arg("-u").run();
    if result.stderr.contains("cannot find name for user ID") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }

    let uid = result.success().stdout_str().trim();
    let result = new_ucmd!().run();
    if is_ci() && result.stderr.contains("cannot find name for user ID") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }

    if !result.stderr_str().contains("Could not find uid") {
        // Verify that the id found by --user/-u exists in the list
        result.success().stdout_contains(&uid);
    }
}

#[test]
fn test_id_from_name() {
    let username = return_whoami_username();
    if username == "" {
        // Sometimes, the CI is failing here
        return;
    }

    let result = new_ucmd!().arg(&username).succeeds();
    let uid = result.stdout_str().trim();

    new_ucmd!()
        .succeeds()
        // Verify that the id found by --user/-u exists in the list
        .stdout_contains(uid)
        // Verify that the username found by whoami exists in the list
        .stdout_contains(username);
}

#[test]
fn test_id_name_from_id() {
    let result = new_ucmd!().arg("-u").succeeds();
    let uid = result.stdout_str().trim();

    let result = new_ucmd!().arg("-nu").arg(uid).run();
    if is_ci() && result.stderr.contains("No such user/group") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }

    let username_id = result.success().stdout_str().trim();

    let scene = TestScenario::new("whoami");
    let result = scene.cmd("whoami").succeeds();

    let username_whoami = result.stdout_str().trim();

    assert_eq!(username_id, username_whoami);
}

#[test]
fn test_id_group() {
    let mut result = new_ucmd!().arg("-g").succeeds();
    let s1 = result.stdout_str().trim();
    assert!(s1.parse::<f64>().is_ok());

    result = new_ucmd!().arg("--group").succeeds();
    let s1 = result.stdout_str().trim();
    assert!(s1.parse::<f64>().is_ok());
}

#[test]
fn test_id_groups() {
    let result = new_ucmd!().arg("-G").succeeds();
    assert!(result.success);
    let groups = result.stdout_str().trim().split_whitespace();
    for s in groups {
        assert!(s.parse::<f64>().is_ok());
    }

    let result = new_ucmd!().arg("--groups").succeeds();
    assert!(result.success);
    let groups = result.stdout_str().trim().split_whitespace();
    for s in groups {
        assert!(s.parse::<f64>().is_ok());
    }
}

#[test]
fn test_id_user() {
    let mut result = new_ucmd!().arg("-u").succeeds();
    let s1 = result.stdout_str().trim();
    assert!(s1.parse::<f64>().is_ok());

    result = new_ucmd!().arg("--user").succeeds();
    let s1 = result.stdout_str().trim();
    assert!(s1.parse::<f64>().is_ok());
}

#[test]
fn test_id_pretty_print() {
    let username = return_whoami_username();
    if username == "" {
        // Sometimes, the CI is failing here
        return;
    }

    let result = new_ucmd!().arg("-p").run();
    if result.stdout_str().trim() == "" {
        // Sometimes, the CI is failing here with
        // old rust versions on Linux
        return;
    }
    result.success().stdout_contains(username);
}

#[test]
fn test_id_password_style() {
    let username = return_whoami_username();
    if username == "" {
        // Sometimes, the CI is failing here
        return;
    }

    let result = new_ucmd!().arg("-P").succeeds();
    assert!(result.stdout_str().starts_with(&username));
}
