// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (words) agroupthatdoesntexist auserthatdoesntexist cuuser groupname notexisting passgrp

#[cfg(any(target_os = "linux", target_os = "android"))]
use uucore::process::geteuid;
use uutests::util::{CmdResult, TestScenario, is_ci, run_ucmd_as_root};
use uutests::util_name;
use uutests::{at_and_ucmd, new_ucmd};
// Apparently some CI environments have configuration issues, e.g. with 'whoami' and 'id'.
// If we are running inside the CI and "needle" is in "stderr" skipping this test is
// considered okay. If we are not inside the CI this calls assert!(result.success).
//
// From the Logs: "Build (ubuntu-18.04, x86_64-unknown-linux-gnu, feat_os_unix, use-cross)"
//
// stderr: "whoami: cannot find name for user ID 1001"
// TODO: Maybe `adduser --uid 1001 username` can put things right?
//
// stderr: "id: cannot find name for group ID 116"
// stderr: "thread 'main' panicked at 'called `Result::unwrap()` on an `Err`
//     value: Custom { kind: NotFound, error: "No such id: 1001" }',
//     /project/src/uucore/src/lib/features/perms.rs:176:44"
//
fn skipping_test_is_okay(result: &CmdResult, needle: &str) -> bool {
    if !result.succeeded() {
        println!("result.stdout = {}", result.stdout_str());
        println!("result.stderr = {}", result.stderr_str());
        if is_ci() && result.stderr_str().contains(needle) {
            println!("test skipped:");
            return true;
        }
        result.success();
    }
    false
}

#[cfg(any(target_os = "linux", target_os = "android", target_os = "windows"))]
const ROOT_GROUP: &str = "root";
#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "windows")))]
const ROOT_GROUP: &str = "wheel";

#[cfg(test)]
mod test_passgrp {
    use chown::entries::{gid2grp, grp2gid, uid2usr, usr2uid};

    #[test]
    fn test_usr2uid() {
        assert_eq!(0, usr2uid("root").unwrap());
        assert!(usr2uid("88_888_888").is_err());
        assert!(usr2uid("auserthatdoesntexist").is_err());
    }

    #[test]
    fn test_grp2gid() {
        assert_eq!(0, grp2gid(super::ROOT_GROUP).unwrap());
        assert!(grp2gid("88_888_888").is_err());
        assert!(grp2gid("agroupthatdoesntexist").is_err());
    }

    #[test]
    fn test_uid2usr() {
        assert_eq!("root", uid2usr(0).unwrap());
        assert!(uid2usr(88_888_888).is_err());
    }

    #[test]
    fn test_gid2grp() {
        assert_eq!(super::ROOT_GROUP, gid2grp(0).unwrap());
        assert!(gid2grp(88_888_888).is_err());
    }
}

#[test]
fn test_invalid_option() {
    new_ucmd!().arg("-w").arg("-q").arg("/").fails();
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_chown_only_owner() {
    // test chown username file.txt

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let result = scene.cmd("whoami").run();
    if skipping_test_is_okay(&result, "whoami: cannot find name for user ID") {
        return;
    }
    let user_name = String::from(result.stdout_str().trim());
    assert!(!user_name.is_empty());

    let file1 = "test_chown_file1";
    at.touch(file1);

    // since only superuser can change owner, we have to change from ourself to ourself
    scene
        .ucmd()
        .arg(user_name)
        .arg("--verbose")
        .arg(file1)
        .succeeds()
        .stderr_contains("retained as");

    // try to change to another existing user, e.g. 'root'
    scene
        .ucmd()
        .arg("root")
        .arg("--verbose")
        .arg(file1)
        .fails()
        .stderr_contains("failed to change");
}

#[test]
fn test_chown_only_owner_colon() {
    // test chown username: file.txt

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let result = scene.cmd("whoami").run();
    if skipping_test_is_okay(&result, "whoami: cannot find name for user ID") {
        return;
    }
    let user_name = String::from(result.stdout_str().trim());
    assert!(!user_name.is_empty());

    let file1 = "test_chown_file1";
    at.touch(file1);

    scene
        .ucmd()
        .arg(format!("{user_name}:"))
        .arg("--verbose")
        .arg(file1)
        .succeeds()
        .stderr_contains("retained as");

    scene
        .ucmd()
        .arg(format!("{user_name}."))
        .arg("--verbose")
        .arg(file1)
        .succeeds()
        .stderr_contains("retained as");

    scene
        .ucmd()
        .arg("root:")
        .arg("--verbose")
        .arg(file1)
        .fails()
        .stderr_contains("failed to change");
}

#[test]
fn test_chown_only_colon() {
    // test chown : file.txt

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let file1 = "test_chown_file1";
    at.touch(file1);

    // expected:
    // $ chown -v : file.txt 2>out_err ; echo $? ; cat out_err
    // ownership of 'file.txt' retained
    // 0
    let result = scene.ucmd().arg(":").arg("--verbose").arg(file1).run();
    if skipping_test_is_okay(&result, "No such id") {
        return;
    }
    result.stderr_contains("retained as"); // TODO: verbose is not printed to stderr in GNU chown

    // test chown : file.txt
    // expected:
    // $ chown -v :: file.txt 2>out_err ; echo $? ; cat out_err
    // 1
    // chown: invalid group: '::'
    scene
        .ucmd()
        .arg("::")
        .arg("--verbose")
        .arg(file1)
        .fails()
        .stderr_contains("invalid group: '::'");

    scene
        .ucmd()
        .arg("..")
        .arg("--verbose")
        .arg(file1)
        .fails()
        .stderr_contains("invalid group: '..'");
}

#[test]
fn test_chown_failed_stdout() {
    // test chown root file.txt

    // TODO: implement once output "failed to change" to stdout is fixed
    // expected:
    // $ chown -v root file.txt 2>out_err ; echo $? ; cat out_err
    // failed to change ownership of 'file.txt' from jhs to root
    // 1
    // chown: changing ownership of 'file.txt': Operation not permitted
}

#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_chown_owner_group() {
    // test chown username:group file.txt

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let result = scene.cmd("whoami").run();
    if skipping_test_is_okay(&result, "whoami: cannot find name for user ID") {
        return;
    }

    let user_name = String::from(result.stdout_str().trim());
    assert!(!user_name.is_empty());

    let file1 = "test_chown_file1";
    at.touch(file1);

    let result = scene.cmd("id").arg("-gn").run();
    if skipping_test_is_okay(&result, "id: cannot find name for group ID") {
        return;
    }
    let group_name = String::from(result.stdout_str().trim());
    assert!(!group_name.is_empty());

    let result = scene
        .ucmd()
        .arg(format!("{user_name}:{group_name}"))
        .arg("--verbose")
        .arg(file1)
        .run();
    if skipping_test_is_okay(&result, "chown: invalid group:") {
        return;
    }
    result.stderr_contains("retained as");

    scene
        .ucmd()
        .arg("root:root:root")
        .arg("--verbose")
        .arg(file1)
        .fails()
        .stderr_contains("invalid group");

    scene
        .ucmd()
        .arg("root.root.root")
        .arg("--verbose")
        .arg(file1)
        .fails()
        .stderr_contains("invalid group");

    scene
        .ucmd()
        .arg(format!("root:{ROOT_GROUP}"))
        .arg("--verbose")
        .arg(file1)
        .fails()
        .stderr_contains("failed to change");
}

#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_chown_various_input() {
    // test chown username:group file.txt

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let result = scene.cmd("whoami").run();
    if skipping_test_is_okay(&result, "whoami: cannot find name for user ID") {
        return;
    }

    let user_name = String::from(result.stdout_str().trim());
    assert!(!user_name.is_empty());

    let file1 = "test_chown_file1";
    at.touch(file1);

    let result = scene.cmd("id").arg("-gn").run();
    if skipping_test_is_okay(&result, "id: cannot find name for group ID") {
        return;
    }
    let group_name = String::from(result.stdout_str().trim());
    assert!(!group_name.is_empty());

    let result = scene
        .ucmd()
        .arg(format!("{user_name}:{group_name}"))
        .arg("--verbose")
        .arg(file1)
        .run();
    if skipping_test_is_okay(&result, "chown: invalid group:") {
        return;
    }
    result.stderr_contains("retained as");

    // check that username.groupname is understood
    let result = scene
        .ucmd()
        .arg(format!("{user_name}.{group_name}"))
        .arg("--verbose")
        .arg(file1)
        .run();
    if skipping_test_is_okay(&result, "chown: invalid group:") {
        return;
    }
    result.stderr_contains("retained as");

    // Fails as user.name doesn't exist in the CI
    // but it is valid
    scene
        .ucmd()
        .arg(format!("{}:{}", "user.name", "groupname"))
        .arg("--verbose")
        .arg(file1)
        .fails()
        .stderr_contains("chown: invalid user: 'user.name:groupname'");
}

#[test]
#[cfg(any(windows, all(unix, not(target_os = "openbsd"))))]
fn test_chown_only_group() {
    // test chown :group file.txt

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let result = scene.cmd("id").arg("-gn").run();
    if skipping_test_is_okay(&result, "id: cannot find name for group ID") {
        return;
    }
    let group_name = String::from(result.stdout_str().trim());
    assert!(!group_name.is_empty());

    let file1 = "test_chown_file1";
    at.touch(file1);

    let result = scene
        .ucmd()
        .arg(format!(":{group_name}"))
        .arg("--verbose")
        .arg(file1)
        .run();
    result.stderr_contains("retained as");
    result.success();

    // FreeBSD user on CI is part of wheel group
    if group_name != ROOT_GROUP {
        scene
            .ucmd()
            .arg(format!(":{ROOT_GROUP}"))
            .arg("--verbose")
            .arg(file1)
            .fails()
            .stderr_contains("failed to change");
    }
}

#[test]
fn test_chown_only_user_id() {
    // test chown 1111 file.txt

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let result = scene.cmd("id").arg("-u").run();
    if skipping_test_is_okay(&result, "id: cannot find name for group ID") {
        return;
    }
    let user_id = String::from(result.stdout_str().trim());
    assert!(!user_id.is_empty());

    let file1 = "test_chown_file1";
    at.touch(file1);

    let result = scene.ucmd().arg(user_id).arg("--verbose").arg(file1).run();
    if skipping_test_is_okay(&result, "invalid user") {
        // From the Logs: "Build (ubuntu-18.04, x86_64-unknown-linux-gnu, feat_os_unix, use-cross)"
        // stderr: "chown: invalid user: '1001'
        return;
    }
    result.stderr_contains("retained as");

    scene
        .ucmd()
        .arg("0")
        .arg("--verbose")
        .arg(file1)
        .fails()
        .stderr_contains("failed to change");
}

#[test]
fn test_chown_fail_id() {
    // test chown 1111. file.txt

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let result = scene.cmd("id").arg("-u").run();
    if skipping_test_is_okay(&result, "id: cannot find name for group ID") {
        return;
    }
    let user_id = String::from(result.stdout_str().trim());
    assert!(!user_id.is_empty());

    let file1 = "test_chown_file1";
    at.touch(file1);

    scene
        .ucmd()
        .arg(format!("{user_id}:"))
        .arg(file1)
        .fails()
        .stderr_contains("invalid spec");

    scene
        .ucmd()
        .arg(format!("{user_id}."))
        .arg(file1)
        .fails()
        .stderr_contains("invalid spec");
}

/// Test for setting the owner to a user ID for a user that does not exist.
///
/// For example:
///
///     $ touch f && chown 12345 f
///
/// succeeds with exit status 0 and outputs nothing. The owner of the
/// file is set to 12345, even though no user with that ID exists.
///
/// This test must be run as root, because only the root user can
/// transfer ownership of a file.
#[test]
fn test_chown_only_user_id_nonexistent_user() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("f");
    if let Ok(result) = run_ucmd_as_root(&ts, &["12345", "f"]) {
        result.success().no_stdout().no_stderr();
    } else {
        print!("Test skipped; requires root user");
    }
}

#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_chown_only_group_id() {
    // test chown :1111 file.txt

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let result = scene.cmd("id").arg("-g").run();
    if skipping_test_is_okay(&result, "id: cannot find name for group ID") {
        return;
    }
    let group_id = String::from(result.stdout_str().trim());
    assert!(!group_id.is_empty());

    let file1 = "test_chown_file1";
    at.touch(file1);

    let result = scene
        .ucmd()
        .arg(format!(":{group_id}"))
        .arg("--verbose")
        .arg(file1)
        .run();
    if skipping_test_is_okay(&result, "chown: invalid group:") {
        // With mac into the CI, we can get this answer
        return;
    }
    result.stderr_contains("retained as");

    // Apparently on CI "macos-latest, x86_64-apple-darwin, feat_os_macos"
    // the process has the rights to change from runner:staff to runner:wheel
    #[cfg(any(windows, all(unix, not(target_os = "macos"))))]
    // FreeBSD user on CI is part of wheel group
    if group_id != "0" {
        scene
            .ucmd()
            .arg(":0")
            .arg("--verbose")
            .arg(file1)
            .fails()
            .stderr_contains("failed to change");
    }
}

/// Test for setting the group to a group ID for a group that does not exist.
///
/// For example:
///
///     $ touch f && chown :12345 f
///
/// succeeds with exit status 0 and outputs nothing. The group of the
/// file is set to 12345, even though no group with that ID exists.
///
/// This test must be run as root, because only the root user can
/// transfer ownership of a file.
#[test]
fn test_chown_only_group_id_nonexistent_group() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.touch("f");
    if let Ok(result) = run_ucmd_as_root(&ts, &[":12345", "f"]) {
        result.success().no_stdout().no_stderr();
    } else {
        print!("Test skipped; requires root user");
    }
}

#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_chown_owner_group_id() {
    // test chown 1111:1111 file.txt

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let result = scene.cmd("id").arg("-u").run();
    if skipping_test_is_okay(&result, "id: cannot find name for group ID") {
        return;
    }
    let user_id = String::from(result.stdout_str().trim());
    assert!(!user_id.is_empty());

    let result = scene.cmd("id").arg("-g").run();
    if skipping_test_is_okay(&result, "id: cannot find name for group ID") {
        return;
    }
    let group_id = String::from(result.stdout_str().trim());
    assert!(!group_id.is_empty());

    let file1 = "test_chown_file1";
    at.touch(file1);

    let result = scene
        .ucmd()
        .arg(format!("{user_id}:{group_id}"))
        .arg("--verbose")
        .arg(file1)
        .run();
    if skipping_test_is_okay(&result, "invalid user") {
        // From the Logs: "Build (ubuntu-18.04, x86_64-unknown-linux-gnu, feat_os_unix, use-cross)"
        // stderr: "chown: invalid user: '1001:116'
        return;
    }
    result.stderr_contains("retained as");

    let result = scene
        .ucmd()
        .arg(format!("{user_id}.{group_id}"))
        .arg("--verbose")
        .arg(file1)
        .run();
    if skipping_test_is_okay(&result, "invalid user") {
        // From the Logs: "Build (ubuntu-18.04, x86_64-unknown-linux-gnu, feat_os_unix, use-cross)"
        // stderr: "chown: invalid user: '1001.116'
        return;
    }
    result.stderr_contains("retained as");

    scene
        .ucmd()
        .arg("0:0")
        .arg("--verbose")
        .arg(file1)
        .fails()
        .stderr_contains("failed to change");
}

#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_chown_owner_group_mix() {
    // test chown 1111:group file.txt

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let result = scene.cmd("id").arg("-u").run();
    if skipping_test_is_okay(&result, "id: cannot find name for group ID") {
        return;
    }
    let user_id = String::from(result.stdout_str().trim());
    assert!(!user_id.is_empty());

    let result = scene.cmd("id").arg("-gn").run();
    if skipping_test_is_okay(&result, "id: cannot find name for group ID") {
        return;
    }
    let group_name = String::from(result.stdout_str().trim());
    assert!(!group_name.is_empty());

    let file1 = "test_chown_file1";
    at.touch(file1);

    let result = scene
        .ucmd()
        .arg(format!("{user_id}:{group_name}"))
        .arg("--verbose")
        .arg(file1)
        .run();
    result.stderr_contains("retained as");

    scene
        .ucmd()
        .arg(format!("0:{ROOT_GROUP}"))
        .arg("--verbose")
        .arg(file1)
        .fails()
        .stderr_contains("failed to change");
}

#[test]
fn test_chown_recursive() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let result = scene.cmd("whoami").run();
    if skipping_test_is_okay(&result, "whoami: cannot find name for user ID") {
        return;
    }
    let user_name = String::from(result.stdout_str().trim());
    assert!(!user_name.is_empty());

    at.mkdir_all("a/b/c");
    at.mkdir("z");
    at.touch(at.plus_as_string("a/a"));
    at.touch(at.plus_as_string("a/b/b"));
    at.touch(at.plus_as_string("a/b/c/c"));
    at.touch(at.plus_as_string("z/y"));

    scene
        .ucmd()
        .arg("-R")
        .arg("--verbose")
        .arg(user_name)
        .arg("a")
        .arg("z")
        .succeeds()
        .stderr_contains("ownership of 'a/a' retained as")
        .stderr_contains("ownership of 'z/y' retained as");
}

#[test]
fn test_root_preserve() {
    let scene = TestScenario::new(util_name!());

    let result = scene.cmd("whoami").run();
    if skipping_test_is_okay(&result, "whoami: cannot find name for user ID") {
        return;
    }
    let user_name = String::from(result.stdout_str().trim());
    assert!(!user_name.is_empty());

    let result = scene
        .ucmd()
        .arg("--preserve-root")
        .arg("-R")
        .arg(user_name)
        .arg("/")
        .fails();
    result.stderr_contains("chown: it is dangerous to operate recursively");
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_big_p() {
    if geteuid() != 0 {
        new_ucmd!()
            .arg("-RP")
            .arg("bin")
            .arg("/proc/self/cwd")
            .fails()
            .stderr_contains(
                // linux fails with "Operation not permitted (os error 1)"
                // because of insufficient permissions,
                // android fails with "Permission denied (os error 13)"
                // because it can't resolve /proc (even though it can resolve /proc/self/)
                "chown: changing ownership of '/proc/self/cwd': ",
            );
    }
}

#[test]
fn test_chown_file_notexisting() {
    // test chown username not_existing

    let scene = TestScenario::new(util_name!());

    let result = scene.cmd("whoami").run();
    if skipping_test_is_okay(&result, "whoami: cannot find name for user ID") {
        return;
    }
    let user_name = String::from(result.stdout_str().trim());
    assert!(!user_name.is_empty());

    scene
        .ucmd()
        .arg(&user_name)
        .arg("--verbose")
        .arg("not_existing")
        .fails()
        .stdout_contains(format!(
            "failed to change ownership of 'not_existing' to {user_name}"
        ));
    // TODO: uncomment once message changed from "cannot dereference" to "cannot access"
    // result.stderr_contains("cannot access 'not_existing': No such file or directory");
}

#[test]
fn test_chown_no_change_to_user() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let result = scene.cmd("whoami").run();
    if skipping_test_is_okay(&result, "whoami: cannot find name for user ID") {
        return;
    }
    let user_name = String::from(result.stdout_str().trim());
    assert!(!user_name.is_empty());

    for (i, from) in ["42", ":42", "42:42"].iter().enumerate() {
        let file = i.to_string();
        at.touch(&file);
        scene
            .ucmd()
            .arg("-v")
            .arg(format!("--from={from}"))
            .arg("43")
            .arg(&file)
            .succeeds()
            .stdout_only(format!("ownership of '{file}' retained as {user_name}\n"));
    }
}

#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_chown_no_change_to_group() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let result = scene.cmd("whoami").run();
    if skipping_test_is_okay(&result, "whoami: cannot find name for user ID") {
        return;
    }
    let user_name = String::from(result.stdout_str().trim());
    assert!(!user_name.is_empty());
    let result = scene.cmd("id").arg("-ng").run();
    if skipping_test_is_okay(&result, "id: cannot find name for group ID") {
        return;
    }
    let group_name = String::from(result.stdout_str().trim());
    assert!(!group_name.is_empty());

    for (i, from) in ["42", ":42", "42:42"].iter().enumerate() {
        let file = i.to_string();
        at.touch(&file);
        scene
            .ucmd()
            .arg("-v")
            .arg(format!("--from={from}"))
            .arg(":43")
            .arg(&file)
            .succeeds()
            .stdout_only(format!("ownership of '{file}' retained as {group_name}\n"));
    }
}

#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_chown_no_change_to_user_group() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let result = scene.cmd("whoami").run();
    if skipping_test_is_okay(&result, "whoami: cannot find name for user ID") {
        return;
    }
    let user_name = String::from(result.stdout_str().trim());
    assert!(!user_name.is_empty());
    let result = scene.cmd("id").arg("-ng").run();
    if skipping_test_is_okay(&result, "id: cannot find name for group ID") {
        return;
    }
    let group_name = String::from(result.stdout_str().trim());
    assert!(!group_name.is_empty());

    for (i, from) in ["42", ":42", "42:42"].iter().enumerate() {
        let file = i.to_string();
        at.touch(&file);
        scene
            .ucmd()
            .arg("-v")
            .arg(format!("--from={from}"))
            .arg("43:43")
            .arg(&file)
            .succeeds()
            .stdout_only(format!(
                "ownership of '{file}' retained as {user_name}:{group_name}\n"
            ));
    }
}

#[test]
fn test_chown_reference_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("a");
    at.touch("b");
    ucmd.arg("--verbose")
        .arg("--reference")
        .arg("a")
        .arg("b")
        .succeeds()
        .stderr_contains("ownership of 'b' retained as")
        .no_stdout();
}
