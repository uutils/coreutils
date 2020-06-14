use crate::common::util::*;

#[test]
fn test_id() {
    let scene = TestScenario::new(util_name!());

    let mut result = scene.ucmd().arg("-u").run();
    if result.stderr.contains("cannot find name for user ID") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    assert!(result.success);

    let uid = String::from(result.stdout.trim());
    result = scene.ucmd().run();
    if is_ci() && result.stderr.contains("cannot find name for user ID") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    println!("result.stdout = {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    if !result.stderr.contains("Could not find uid") {
        // Verify that the id found by --user/-u exists in the list
        assert!(result.stdout.contains(&uid));
    }
}

#[test]
fn test_id_from_name() {
    let mut scene = TestScenario::new("whoami");
    let result = scene.cmd("whoami").run();
    if is_ci() && result.stderr.contains("cannot find name for user ID") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }

    let username = result.stdout.trim();

    scene = TestScenario::new(util_name!());
    let result = scene.ucmd().arg(username).succeeds();
    println!("result.stdout = {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    assert!(result.success);
    let uid = String::from(result.stdout.trim());
    let result = scene.ucmd().succeeds();
    println!("result.stdout = {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    // Verify that the id found by --user/-u exists in the list
    assert!(result.stdout.contains(&uid));
    // Verify that the username found by whoami exists in the list
    assert!(result.stdout.contains(&username));
}

#[test]
fn test_id_name_from_id() {
    let mut scene = TestScenario::new(util_name!());
    let result = scene.ucmd().arg("-u").run();
    println!("result.stdout = {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    assert!(result.success);
    let uid = String::from(result.stdout.trim());

    scene = TestScenario::new(util_name!());
    let result = scene.ucmd().arg("-nu").arg(uid).run();
    if is_ci() && result.stderr.contains("No such user/group") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    println!("result.stdout = {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    assert!(result.success);

    let username_id = String::from(result.stdout.trim());

    scene = TestScenario::new("whoami");
    let result = scene.cmd("whoami").run();

    let username_whoami = result.stdout.trim();

    assert_eq!(username_id, username_whoami);
}

#[test]
fn test_id_group() {
    let scene = TestScenario::new(util_name!());

    let mut result = scene.ucmd().arg("-g").succeeds();
    println!("result.stdout = {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    assert!(result.success);
    let s1 = String::from(result.stdout.trim());
    assert!(s1.parse::<f64>().is_ok());

    result = scene.ucmd().arg("--group").succeeds();
    println!("result.stdout = {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    assert!(result.success);
    let s1 = String::from(result.stdout.trim());
    assert!(s1.parse::<f64>().is_ok());
}

#[test]
fn test_id_groups() {
    let scene = TestScenario::new(util_name!());

    let result = scene.ucmd().arg("-G").succeeds();
    println!("result.stdout = {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    assert!(result.success);
    let groups = result.stdout.trim().split_whitespace();
    for s in groups {
        assert!(s.parse::<f64>().is_ok());
    }

    let result = scene.ucmd().arg("--groups").succeeds();
    println!("result.stdout = {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    assert!(result.success);
    let groups = result.stdout.trim().split_whitespace();
    for s in groups {
        assert!(s.parse::<f64>().is_ok());
    }
}

#[test]
fn test_id_user() {
    let scene = TestScenario::new(util_name!());

    let mut result = scene.ucmd().arg("-u").succeeds();
    assert!(result.success);
    let s1 = String::from(result.stdout.trim());
    assert!(s1.parse::<f64>().is_ok());
    result = scene.ucmd().arg("--user").succeeds();
    assert!(result.success);
    let s1 = String::from(result.stdout.trim());
    assert!(s1.parse::<f64>().is_ok());
}
