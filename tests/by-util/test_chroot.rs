use crate::common::util::*;

#[test]
fn test_missing_operand() {
    let result = new_ucmd!().run();

    assert_eq!(
        true,
        result
            .stderr
            .starts_with("error: The following required arguments were not provided")
    );

    assert_eq!(true, result.stderr.contains("<newroot>"));
}

#[test]
fn test_enter_chroot_fails() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("jail");

    let result = ucmd.arg("jail").run();

    assert_eq!(
        true,
        result.stderr.starts_with(
            "chroot: error: cannot chroot to jail: Operation not permitted (os error 1)"
        )
    )
}

#[test]
fn test_no_such_directory() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch(&at.plus_as_string("a"));

    ucmd.arg("a")
        .fails()
        .stderr_is("chroot: error: cannot change root directory to `a`: no such directory");
}

#[test]
fn test_invalid_user_spec() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("a");

    let result = ucmd.arg("a").arg("--userspec=ARABA:").run();

    assert_eq!(
        true,
        result.stderr.starts_with("chroot: error: invalid userspec")
    );
}

#[test]
fn test_preference_of_userspec() {
    let scene = TestScenario::new(util_name!());
    let result = scene.cmd("whoami").run();
    if is_ci() && result.stderr.contains("No such user/group") {
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

    if is_ci() && result.stderr.contains("cannot find name for user ID") {
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
