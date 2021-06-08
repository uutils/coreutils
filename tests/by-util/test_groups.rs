use crate::common::util::*;

#[test]
#[cfg(any(target_vendor = "apple", target_os = "linux"))]
fn test_groups() {
    if !is_ci() {
        new_ucmd!().succeeds().stdout_is(expected_result(&[]));
    } else {
        // TODO: investigate how this could be tested in CI
        // stderr = groups: cannot find name for group ID 116
        println!("test skipped:");
    }
}

#[test]
#[cfg(any(target_vendor = "apple", target_os = "linux"))]
#[ignore = "fixme: 'groups USERNAME' needs more debugging"]
fn test_groups_username() {
    let scene = TestScenario::new(util_name!());
    let whoami_result = scene.cmd("whoami").run();

    let username = if whoami_result.succeeded() {
        whoami_result.stdout_move_str()
    } else if is_ci() {
        String::from("docker")
    } else {
        println!("test skipped:");
        return;
    };

    // TODO: stdout should be in the form: "username : group1 group2 group3"

    scene
        .ucmd()
        .arg(&username)
        .succeeds()
        .stdout_is(expected_result(&[&username]));
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
fn expected_result(args: &[&str]) -> String {
    let util_name = "id";

    TestScenario::new(&util_name)
        .cmd_keepenv(util_name)
        .env("LANGUAGE", "C")
        .args(args)
        .args(&["-Gn"])
        .succeeds()
        .stdout_move_str()
}
