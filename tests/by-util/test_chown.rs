use crate::common::util::*;

extern crate chown;
// pub use self::uu_chown::*;

#[cfg(test)]
mod test_passgrp {
    use super::chown::entries::{gid2grp, grp2gid, uid2usr, usr2uid};

    #[test]
    fn test_usr2uid() {
        assert_eq!(0, usr2uid("root").unwrap());
        assert!(usr2uid("88888888").is_err());
        assert!(usr2uid("auserthatdoesntexist").is_err());
    }

    #[test]
    fn test_grp2gid() {
        if cfg!(target_os = "linux") || cfg!(target_os = "android") || cfg!(target_os = "windows") {
            assert_eq!(0, grp2gid("root").unwrap())
        } else {
            assert_eq!(0, grp2gid("wheel").unwrap());
        }
        assert!(grp2gid("88888888").is_err());
        assert!(grp2gid("agroupthatdoesntexist").is_err());
    }

    #[test]
    fn test_uid2usr() {
        assert_eq!("root", uid2usr(0).unwrap());
        assert!(uid2usr(88888888).is_err());
    }

    #[test]
    fn test_gid2grp() {
        if cfg!(target_os = "linux") || cfg!(target_os = "android") || cfg!(target_os = "windows") {
            assert_eq!("root", gid2grp(0).unwrap());
        } else {
            assert_eq!("wheel", gid2grp(0).unwrap());
        }
        assert!(gid2grp(88888888).is_err());
    }
}

#[test]
fn test_invalid_option() {
    new_ucmd!().arg("-w").arg("-q").arg("/").fails();
}

#[test]
fn test_chown_myself() {
    // test chown username file.txt
    let scene = TestScenario::new(util_name!());
    let result = scene.cmd("whoami").run();
    if is_ci() && result.stderr.contains("No such user/group") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    println!("results {}", result.stdout);
    let username = result.stdout.trim_end();

    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "test_install_target_dir_file_a1";

    at.touch(file1);
    let result = ucmd.arg(username).arg(file1).run();
    println!("results stdout {}", result.stdout);
    println!("results stderr {}", result.stderr);
    if is_ci() && result.stderr.contains("invalid user") {
        // In the CI, some server are failing to return id.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    assert!(result.success);
}

#[test]
fn test_chown_myself_second() {
    // test chown username: file.txt
    let scene = TestScenario::new(util_name!());
    let result = scene.cmd("whoami").run();
    if is_ci() && result.stderr.contains("No such user/group") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    println!("results {}", result.stdout);

    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "test_install_target_dir_file_a1";

    at.touch(file1);
    let result = ucmd
        .arg(result.stdout.trim_end().to_owned() + ":")
        .arg(file1)
        .run();

    println!("result.stdout = {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    assert!(result.success);
}

#[test]
fn test_chown_myself_group() {
    // test chown username:group file.txt
    let scene = TestScenario::new(util_name!());
    let result = scene.cmd("whoami").run();
    if is_ci() && result.stderr.contains("No such user/group") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    println!("user name = {}", result.stdout);
    let username = result.stdout.trim_end();

    let result = scene.cmd("id").arg("-gn").run();
    if is_ci() && result.stderr.contains("No such user/group") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    println!("group name = {}", result.stdout);
    let group = result.stdout.trim_end();

    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "test_install_target_dir_file_a1";
    let perm = username.to_owned() + ":" + group;
    at.touch(file1);
    let result = ucmd.arg(perm).arg(file1).run();
    println!("result.stdout = {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    if is_ci() && result.stderr.contains("chown: invalid group:") {
        // With some Ubuntu into the CI, we can get this answer
        return;
    }
    assert!(result.success);
}

#[test]
fn test_chown_only_group() {
    // test chown :group file.txt
    let scene = TestScenario::new(util_name!());
    let result = scene.cmd("whoami").run();
    if is_ci() && result.stderr.contains("No such user/group") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    println!("results {}", result.stdout);

    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "test_install_target_dir_file_a1";
    let perm = ":".to_owned() + result.stdout.trim_end();
    at.touch(file1);
    let result = ucmd.arg(perm).arg(file1).run();

    println!("result.stdout = {}", result.stdout);
    println!("result.stderr = {}", result.stderr);

    if is_ci() && result.stderr.contains("Operation not permitted") {
        // With ubuntu with old Rust in the CI, we can get an error
        return;
    }
    if is_ci() && result.stderr.contains("chown: invalid group:") {
        // With mac into the CI, we can get this answer
        return;
    }
    assert!(result.success);
}

#[test]
fn test_chown_only_id() {
    // test chown 1111 file.txt
    let result = TestScenario::new("id").ucmd_keepenv().arg("-u").run();
    if is_ci() && result.stderr.contains("No such user/group") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    println!("result.stdout = {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    let id = String::from(result.stdout.trim());

    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "test_install_target_dir_file_a1";

    at.touch(file1);
    let result = ucmd.arg(id).arg(file1).run();

    println!("result.stdout = {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    if is_ci() && result.stderr.contains("chown: invalid user:") {
        // With some Ubuntu into the CI, we can get this answer
        return;
    }
    assert!(result.success);
}

#[test]
fn test_chown_only_group_id() {
    // test chown :1111 file.txt
    let result = TestScenario::new("id").ucmd_keepenv().arg("-g").run();
    if is_ci() && result.stderr.contains("No such user/group") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    println!("result.stdout {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    let id = String::from(result.stdout.trim());

    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "test_install_target_dir_file_a1";

    at.touch(file1);
    let perm = ":".to_owned() + &id;

    let result = ucmd.arg(perm).arg(file1).run();

    println!("result.stdout = {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    if is_ci() && result.stderr.contains("chown: invalid group:") {
        // With mac into the CI, we can get this answer
        return;
    }
    assert!(result.success);
}

#[test]
fn test_chown_both_id() {
    // test chown 1111:1111 file.txt
    let result = TestScenario::new("id").ucmd_keepenv().arg("-u").run();
    if is_ci() && result.stderr.contains("No such user/group") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    println!("result.stdout {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    let id_user = String::from(result.stdout.trim());

    let result = TestScenario::new("id").ucmd_keepenv().arg("-g").run();
    if is_ci() && result.stderr.contains("No such user/group") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    println!("result.stdout {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    let id_group = String::from(result.stdout.trim());

    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "test_install_target_dir_file_a1";

    at.touch(file1);
    let perm = id_user + &":".to_owned() + &id_group;

    let result = ucmd.arg(perm).arg(file1).run();
    println!("result.stdout {}", result.stdout);
    println!("result.stderr = {}", result.stderr);

    if is_ci() && result.stderr.contains("invalid user") {
        // In the CI, some server are failing to return id.
        // As seems to be a configuration issue, ignoring it
        return;
    }

    assert!(result.success);
}

#[test]
fn test_chown_both_mix() {
    // test chown 1111:1111 file.txt
    let result = TestScenario::new("id").ucmd_keepenv().arg("-u").run();
    if is_ci() && result.stderr.contains("No such user/group") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    println!("result.stdout {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    let id_user = String::from(result.stdout.trim());

    let result = TestScenario::new("id").ucmd_keepenv().arg("-gn").run();
    if is_ci() && result.stderr.contains("No such user/group") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    println!("result.stdout {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    let group_name = String::from(result.stdout.trim());

    let (at, mut ucmd) = at_and_ucmd!();
    let file1 = "test_install_target_dir_file_a1";

    at.touch(file1);
    let perm = id_user + &":".to_owned() + &group_name;

    let result = ucmd.arg(perm).arg(file1).run();

    if is_ci() && result.stderr.contains("invalid user") {
        // In the CI, some server are failing to return id.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    assert!(result.success);
}

#[test]
fn test_chown_recursive() {
    let scene = TestScenario::new(util_name!());
    let result = scene.cmd("whoami").run();
    if is_ci() && result.stderr.contains("No such user/group") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    println!("result.stdout {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    let username = result.stdout.trim_end();

    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("a");
    at.mkdir("a/b");
    at.mkdir("a/b/c");
    at.mkdir("z");
    at.touch(&at.plus_as_string("a/a"));
    at.touch(&at.plus_as_string("a/b/b"));
    at.touch(&at.plus_as_string("a/b/c/c"));
    at.touch(&at.plus_as_string("z/y"));

    let result = ucmd
        .arg("-R")
        .arg("--verbose")
        .arg(username)
        .arg("a")
        .arg("z")
        .run();
    println!("result.stdout {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    if is_ci() && result.stderr.contains("invalid user") {
        // In the CI, some server are failing to return id.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    assert!(result.stdout.contains("ownership of a/a retained as"));
    assert!(result.success);
}

#[test]
fn test_root_preserve() {
    let scene = TestScenario::new(util_name!());
    let result = scene.cmd("whoami").run();
    if is_ci() && result.stderr.contains("No such user/group") {
        // In the CI, some server are failing to return whoami.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    println!("result.stdout {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    let username = result.stdout.trim_end();

    let result = new_ucmd!()
        .arg("--preserve-root")
        .arg("-R")
        .arg(username)
        .arg("/")
        .fails();
    println!("result.stdout {}", result.stdout);
    println!("result.stderr = {}", result.stderr);
    if is_ci() && result.stderr.contains("invalid user") {
        // In the CI, some server are failing to return id.
        // As seems to be a configuration issue, ignoring it
        return;
    }
    assert!(result
        .stderr
        .contains("chown: it is dangerous to operate recursively"));
}
