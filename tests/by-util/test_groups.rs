use crate::common::util::*;

#[test]
#[cfg(unix)]
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
#[cfg(unix)]
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

#[cfg(unix)]
fn expected_result(args: &[&str]) -> String {
    // We want to use GNU id. On most linux systems, this is "id", but on
    // bsd-like systems (e.g. FreeBSD, MacOS), it is commonly "gid".
    #[cfg(any(target_os = "linux"))]
    let util_name = "id";
    #[cfg(not(target_os = "linux"))]
    let util_name = "gid";

    TestScenario::new(util_name)
        .cmd_keepenv(util_name)
        .env("LANGUAGE", "C")
        .args(args)
        .args(&["-Gn"])
        .succeeds()
        .stdout_move_str()
}
