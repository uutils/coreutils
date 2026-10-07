// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (words) agroupthatdoesntexist auserthatdoesntexist groupname notexisting passgrp NOFILE

#[cfg(all(unix, not(target_os = "openbsd")))]
use std::os::unix::fs::MetadataExt;
use uutests::util::{CmdResult, TestScenario, is_ci, run_ucmd_as_root};
use uutests::{at_and_ucmd, new_ucmd, util_name};

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

#[cfg(any(target_os = "linux", target_os = "android", windows))]
const ROOT_GROUP: &str = "root";
#[cfg(not(any(target_os = "linux", target_os = "android", windows)))]
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
        .stdout_contains("retained as")
        .no_stderr();

    // try to change to another existing user, e.g. 'root'
    scene
        .ucmd()
        .arg("root")
        .arg("--verbose")
        .arg(file1)
        .fails()
        .stdout_contains("failed to change");
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

    let result = scene.cmd("id").arg("-gn").run();
    if skipping_test_is_okay(&result, "id: cannot find name for group ID") {
        return;
    }
    let login_group_name = String::from(result.stdout_str().trim());
    let expected = format!("{user_name}:{login_group_name}");

    let file1 = "test_chown_file1";
    at.touch(file1);

    // "username:" sets the owner and the owner's login group; the
    // ownership is "retained" only if the group already matches.
    scene
        .ucmd()
        .arg(format!("{user_name}:"))
        .arg("--verbose")
        .arg(file1)
        .succeeds()
        .stdout_contains(&expected)
        .no_stderr();

    scene
        .ucmd()
        .arg(format!("{user_name}."))
        .arg("--verbose")
        .arg(file1)
        .succeeds()
        .stdout_contains(&expected)
        .stderr_contains("warning: '.' should be ':'");

    scene
        .ucmd()
        .arg("root:")
        .arg("--verbose")
        .arg(file1)
        .fails()
        .stdout_contains("failed to change");
}

/// Test that `chown USER:` sets the group to USER's login group.
#[test]
#[cfg(all(unix, not(target_os = "openbsd")))]
fn test_chown_login_group() {
    use chown::entries::usr2gid;

    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let file1 = "test_chown_login_group";
    at.touch(file1);

    let Ok(root_gid) = usr2gid("root") else {
        print!("Test skipped; no root user");
        return;
    };
    let other_gid = root_gid.wrapping_add(1).to_string();

    // Move the file's group out of root's login group. Changing the
    // ownership to root requires root privileges.
    let Ok(result) = run_ucmd_as_root(&ts, &[&format!("0:{other_gid}"), file1]) else {
        print!("Test skipped; requires root user");
        return;
    };
    result.success().no_output();

    // "root:" must set the group back to root's login group.
    let Ok(result) = run_ucmd_as_root(&ts, &["root:", file1]) else {
        print!("Test skipped; requires root user");
        return;
    };
    result.success().no_output();

    assert_eq!(at.metadata(file1).gid(), root_gid);
}

/// Test that an owner whose login group cannot be resolved is an invalid spec.
#[test]
fn test_chown_invalid_spec() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("f");

    scene
        .ucmd()
        .arg("auserthatdoesntexist:")
        .arg("f")
        .fails()
        .stderr_contains("chown: invalid spec: 'auserthatdoesntexist:'");

    scene
        .ucmd()
        .arg("12345:")
        .arg("f")
        .fails()
        .stderr_contains("chown: invalid spec: '12345:'");
}

#[test]
fn test_chown_dot_separator_warning() {
    // test that using '.' as separator emits a warning

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let result = scene.cmd("whoami").run();
    if skipping_test_is_okay(&result, "whoami: cannot find name for user ID") {
        return;
    }
    let user_name = String::from(result.stdout_str().trim());
    assert!(!user_name.is_empty());

    let file1 = "test_chown_dot_warn";
    at.touch(file1);

    let result = scene.cmd("id").arg("-gn").run();
    if skipping_test_is_okay(&result, "id: cannot find name for group ID") {
        return;
    }
    let group_name = String::from(result.stdout_str().trim());
    assert!(!group_name.is_empty());

    // chown user. file should warn about '.' separator
    scene
        .ucmd()
        .arg(format!("{user_name}."))
        .arg(file1)
        .succeeds()
        .stderr_contains("warning: '.' should be ':'");

    // chown user.group file should warn AND apply both owner and group
    let result = scene
        .ucmd()
        .arg(format!("{user_name}.{group_name}"))
        .arg("--verbose")
        .arg(file1)
        .run();
    if skipping_test_is_okay(&result, "chown: invalid group:") {
        return;
    }
    result.stderr_contains("warning: '.' should be ':'");
    // "retained as" on Linux, "changed ownership" on BSDs (group inherited from parent dir)
    assert!(
        result.stdout_str().contains("retained as")
            || result.stdout_str().contains("changed ownership"),
        "expected verbose ownership output, got stdout={:?} stderr={:?}",
        result.stdout_str(),
        result.stderr_str()
    );

    // chown user: file should not warn
    scene
        .ucmd()
        .arg(format!("{user_name}:"))
        .arg(file1)
        .succeeds()
        .stderr_does_not_contain("warning");
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
    result.stdout_contains("retained as");
    result.no_stderr(); // GNU prints --verbose retained/changed lines to stdout

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

/// With -v, a change that fails is reported on stdout.
#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_chown_failed_stdout() {
    use chown::entries::{gid2grp, uid2usr};

    // Root can give the files to root, so nothing fails.
    if rustix::process::geteuid().is_root() {
        println!("Test skipped: running as root");
        return;
    }
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("file");
    at.mkdir("dir");
    at.touch("dir/file");
    let user = uid2usr(rustix::process::geteuid().as_raw()).unwrap();
    let root = uid2usr(0).unwrap();
    let group = gid2grp(at.plus("dir").metadata().unwrap().gid()).unwrap();
    let failed = |path| {
        format!("failed to change ownership of '{path}' from {user}:{group} to {root}:{group}\n")
    };
    let denied = |path| format!("chown: changing ownership of '{path}': Operation not permitted\n");

    scene
        .ucmd()
        .args(&["-R", "-v", "root", "file"])
        .fails_with_code(1)
        .stdout_is(failed("file"))
        .stderr_is(denied("file"));
    // Files inside the directory get the line too.
    scene
        .ucmd()
        .args(&["-R", "-v", "root", "dir"])
        .fails_with_code(1)
        .stdout_is(failed("dir") + &failed("dir/file"))
        .stderr_is(denied("dir") + &denied("dir/file"));
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
    result.stdout_contains("retained as");

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
        .stdout_contains("failed to change");
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
    result.stdout_contains("retained as");

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
    result.stdout_contains("retained as");

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
    result.stdout_contains("retained as");
    result.success();

    // FreeBSD user on CI is part of wheel group
    if group_name != ROOT_GROUP {
        scene
            .ucmd()
            .arg(format!(":{ROOT_GROUP}"))
            .arg("--verbose")
            .arg(file1)
            .fails()
            .stdout_contains("failed to change");
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
    result.stdout_contains("retained as");

    scene
        .ucmd()
        .arg("0")
        .arg("--verbose")
        .arg(file1)
        .fails()
        .stdout_contains("failed to change");
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
        result.success().no_output();
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
    result.stdout_contains("retained as");

    // Apparently on CI "macos-latest, x86_64-apple-darwin, feat_os_unix"
    // the process has the rights to change from runner:staff to runner:wheel
    #[cfg(any(windows, all(unix, not(target_vendor = "apple"))))]
    // FreeBSD user on CI is part of wheel group
    if group_id != "0" {
        scene
            .ucmd()
            .arg(":0")
            .arg("--verbose")
            .arg(file1)
            .fails()
            .stdout_contains("failed to change");
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
        result.success().no_output();
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
    result.stdout_contains("retained as");

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
    result.stdout_contains("retained as");

    scene
        .ucmd()
        .arg("0:0")
        .arg("--verbose")
        .arg(file1)
        .fails()
        .stdout_contains("failed to change");
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
    result.stdout_contains("retained as");

    scene
        .ucmd()
        .arg(format!("0:{ROOT_GROUP}"))
        .arg("--verbose")
        .arg(file1)
        .fails()
        .stdout_contains("failed to change");
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
    at.touch(at.plus("a/a"));
    at.touch(at.plus("a/b/b"));
    at.touch(at.plus("a/b/c/c"));
    at.touch(at.plus("z/y"));

    scene
        .ucmd()
        .arg("-R")
        .arg("--verbose")
        .arg(user_name)
        .arg("a")
        .arg("z")
        .succeeds()
        .stdout_contains("ownership of 'a/a' retained as")
        .stdout_contains("ownership of 'z/y' retained as");
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
    if !rustix::process::geteuid().is_root() {
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
        ))
        .stderr_contains("cannot access 'not_existing': No such file or directory");
}

#[test]
#[cfg(all(unix, not(target_os = "openbsd")))]
fn test_chown_cannot_access_vs_cannot_dereference() {
    // GNU says "cannot dereference" only for a symlink whose target is missing,
    // and "cannot access" when the operand itself can't be reached.
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.touch("f");
    at.symlink_file("missing", "dangling");
    let uid = at.metadata("f").uid().to_string();

    scene
        .ucmd()
        .arg(&uid)
        .arg("f/")
        .fails_with_code(1)
        .stderr_is("chown: cannot access 'f/': Not a directory\n");
    scene
        .ucmd()
        .arg(&uid)
        .arg("dir/missing")
        .fails_with_code(1)
        .stderr_is("chown: cannot access 'dir/missing': No such file or directory\n");
    scene
        .ucmd()
        .arg(&uid)
        .arg("dangling")
        .fails_with_code(1)
        .stderr_is("chown: cannot dereference 'dangling': No such file or directory\n");
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
        .stdout_contains("ownership of 'b' retained as")
        .no_stderr();
}

#[test]
#[cfg(target_os = "linux")]
fn test_chown_reference_file_with_non_utf8_path() {
    use std::os::unix::ffi::OsStrExt;

    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("file");

    let reference = std::ffi::OsStr::from_bytes(b"reference_\xFF\xFE");
    std::fs::File::create(at.plus(reference)).unwrap();

    ucmd.arg("--verbose")
        .arg("--reference")
        .arg(reference)
        .arg("file")
        .succeeds()
        .stdout_contains("ownership of 'file' retained as")
        .no_stderr();
}

#[test]
#[cfg(all(unix, not(target_os = "openbsd")))]
fn test_chown_no_dereference_symlink_to_dir() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let result = scene.cmd("whoami").run();
    if skipping_test_is_okay(&result, "whoami: cannot find name for user ID") {
        return;
    }
    let user_name = String::from(result.stdout_str().trim());
    assert!(!user_name.is_empty());

    at.mkdir("dir");
    at.symlink_dir("dir", "link_to_dir");

    let link_meta_before = std::fs::symlink_metadata(at.plus("link_to_dir")).unwrap();
    let link_ctime_before = (link_meta_before.ctime(), link_meta_before.ctime_nsec());

    let dir_meta_before = std::fs::metadata(at.plus("dir")).unwrap();
    let dir_ctime_before = (dir_meta_before.ctime(), dir_meta_before.ctime_nsec());

    // Let the file system clock move past the link's ctime. On some systems,
    // such as the Android emulator, it advances in coarse steps.
    std::thread::sleep(std::time::Duration::from_millis(100));

    scene
        .ucmd()
        .arg("--no-dereference")
        .arg(&user_name)
        .arg("link_to_dir")
        .succeeds();

    let link_meta_after = std::fs::symlink_metadata(at.plus("link_to_dir")).unwrap();
    let link_ctime_after = (link_meta_after.ctime(), link_meta_after.ctime_nsec());

    let dir_meta_after = std::fs::metadata(at.plus("dir")).unwrap();
    let dir_ctime_after = (dir_meta_after.ctime(), dir_meta_after.ctime_nsec());

    assert_ne!(
        link_ctime_before, link_ctime_after,
        "link's ctime should have advanced"
    );
    assert_eq!(
        dir_ctime_before, dir_ctime_after,
        "dir's ctime should not have changed"
    );
}

#[test]
fn test_chown_symlink_cycles() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let result = scene.cmd("whoami").run();
    if skipping_test_is_okay(&result, "whoami: cannot find name for user ID") {
        return;
    }
    let user_name = String::from(result.stdout_str().trim());
    assert!(!user_name.is_empty());

    at.mkdir_all("a/b/c");
    at.symlink_dir("a", "a/b/c/d");

    let result = scene.ucmd().arg("-vRL").arg(&user_name).arg("a").run();

    result
        .success()
        .stdout_contains(format!("ownership of 'a' retained as {user_name}"))
        .stdout_contains(format!("ownership of 'a/b' retained as {user_name}"))
        .stdout_contains(format!("ownership of 'a/b/c' retained as {user_name}"))
        .stdout_contains(format!("ownership of 'a/b/c/d' retained as {user_name}"))
        .stdout_does_not_contain(format!("ownership of 'a/b/c/d/b' retained as {user_name}"))
        .stdout_does_not_contain(format!(
            "ownership of 'a/b/c/d/b/c' retained as {user_name}"
        ))
        .stdout_does_not_contain(format!(
            "ownership of 'a/b/c/d/b/c/d' retained as {user_name}"
        ));
}

#[test]
fn test_chown_symlink_two_links_same_dir() {
    // Two symlinks pointing at the same directory is NOT a cycle: neither link is
    // an ancestor of the other, so the target's contents must be visited through
    // *both* links. This guards the backtracking in the safe-traversal cycle
    // detection against false positives.
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;

    let result = scene.cmd("whoami").run();
    if skipping_test_is_okay(&result, "whoami: cannot find name for user ID") {
        return;
    }
    let user_name = String::from(result.stdout_str().trim());
    assert!(!user_name.is_empty());

    // spell-checker:disable
    at.mkdir_all("base/realdir");
    at.touch("base/realdir/file");
    at.symlink_dir("base/realdir", "base/link1");
    at.symlink_dir("base/realdir", "base/link2");

    let result = scene.ucmd().arg("-vRL").arg(&user_name).arg("base").run();

    result
        .success()
        .stdout_contains(format!(
            "ownership of 'base/realdir/file' retained as {user_name}"
        ))
        .stdout_contains(format!(
            "ownership of 'base/link1/file' retained as {user_name}"
        ))
        .stdout_contains(format!(
            "ownership of 'base/link2/file' retained as {user_name}"
        ));
    // spell-checker:enable
}

#[cfg(target_os = "linux")]
#[test]
fn verbose_missing_file_write_error_is_reported_not_panic() {
    use rustix::process::geteuid;
    use std::fs::OpenOptions;

    let dev_full = OpenOptions::new().write(true).open("/dev/full").unwrap();
    new_ucmd!()
        .arg("--verbose")
        .arg(geteuid().to_string())
        .arg("does-not-exist")
        .set_stdout(dev_full)
        .fails_with_code(1)
        .stderr_contains("chown: write error: No space left on device");
}

#[test]
fn test_repeated_flags_are_accepted() {
    use rustix::process::geteuid;

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("dir");
    at.touch("dir/file");

    // GNU chown accepts a flag given more than once; the last one wins.
    for flags in [
        vec!["-R", "-R"],
        vec!["-R", "--recursive"],
        vec!["-v", "-v"],
        vec!["-c", "-c"],
        vec!["-f", "-f"],
        vec!["-R", "-c", "-c"],
    ] {
        scene
            .ucmd()
            .args(&flags)
            .arg(geteuid().to_string())
            .arg("dir")
            .succeeds();
    }
}

/// Under `-R -H` a symlink met in the tree is changed through, so the file
/// changed is not the link that was looked at: `--from` must not refuse it.
#[cfg(all(unix, not(target_os = "openbsd")))]
#[test]
fn test_chown_from_changes_through_symlink_in_tree_under_big_h() {
    use chown::entries::{gid2grp, uid2usr};

    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("dir");
    at.touch("target");
    at.relative_symlink_file("../target", "dir/link");
    let meta = at.plus("target").metadata().unwrap();
    let owner = format!("{}:{}", meta.uid(), meta.gid());
    let user = uid2usr(meta.uid()).unwrap();
    let group = gid2grp(meta.gid()).unwrap();

    ucmd.args(&["-R", "-H", "-v", &format!("--from={owner}"), &owner, "dir"])
        .succeeds()
        .stdout_is(format!(
            "ownership of 'dir' retained as {user}:{group}\nownership of 'dir/link' retained as {user}:{group}\n"
        ));
}

/// A group of the current user other than the effective one, if any.
#[cfg(target_os = "linux")]
fn other_group() -> Option<u32> {
    let current = rustix::process::getegid().as_raw();
    let other = rustix::process::getgroups()
        .unwrap()
        .into_iter()
        .map(rustix::process::Gid::as_raw)
        .find(|&group| group != current);
    if other.is_none() {
        println!("Test skipped: the user is in only one group");
    }
    other
}

/// `-R --from` changes the entries it judged, through a descriptor held on each.
#[cfg(target_os = "linux")]
#[test]
fn test_chown_recursive_from_changes_entries() {
    let Some(other) = other_group() else {
        return;
    };
    let current = rustix::process::getegid().as_raw();
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("dir");
    at.touch("dir/file");

    ucmd.args(&[
        "-R",
        &format!("--from=:{current}"),
        &format!(":{other}"),
        "dir",
    ])
    .succeeds()
    .no_output();
    for path in ["dir", "dir/file"] {
        assert_eq!(at.plus(path).metadata().unwrap().gid(), other, "{path}");
    }
}

/// With -h, `--from` on a symlink operand changes the link and not its target.
#[cfg(target_os = "linux")]
#[test]
fn test_chown_from_changes_symlink_operand_itself() {
    let Some(other) = other_group() else {
        return;
    };
    let current = rustix::process::getegid().as_raw();
    let (at, mut ucmd) = at_and_ucmd!();
    at.touch("target");
    at.relative_symlink_file("target", "link");

    ucmd.args(&[
        "-h",
        &format!("--from=:{current}"),
        &format!(":{other}"),
        "link",
    ])
    .succeeds()
    .no_output();
    assert_eq!(at.plus("link").symlink_metadata().unwrap().gid(), other);
    assert_eq!(at.plus("target").metadata().unwrap().gid(), current);
}

/// A directory operand that cannot be read is still changed without -R.
#[cfg(all(unix, not(target_os = "openbsd")))]
#[test]
fn test_chown_unreadable_dir_operand() {
    use chown::entries::{gid2grp, uid2usr};
    use std::os::unix::fs::PermissionsExt;

    // Root can read the directory anyway.
    if rustix::process::geteuid().is_root() {
        println!("Test skipped: running as root");
        return;
    }
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("dir");
    let meta = at.plus("dir").metadata().unwrap();
    let owner = format!("{}:{}", meta.uid(), meta.gid());
    let names = format!(
        "{}:{}",
        uid2usr(meta.uid()).unwrap(),
        gid2grp(meta.gid()).unwrap()
    );
    std::fs::set_permissions(at.plus("dir"), std::fs::Permissions::from_mode(0o000)).unwrap();

    for from in [None, Some(format!("--from={owner}"))] {
        scene
            .ucmd()
            .arg("-v")
            .args(from.as_slice())
            .args(&[owner.as_str(), "dir"])
            .succeeds()
            .stdout_is(format!("ownership of 'dir' retained as {names}\n"));
    }
}

/// Under `-R -H` a symlink met in the tree is followed for the change, so
/// `--from` and -v concern the file it points to, not the link.
#[cfg(all(unix, not(target_os = "openbsd")))]
#[test]
fn test_chown_from_judges_symlink_target_under_big_h() {
    use chown::entries::{gid2grp, uid2usr};

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let Some((target_group, link_group)) = two_groups(&scene) else {
        return;
    };
    let user = uid2usr(rustix::process::geteuid().as_raw()).unwrap();
    let target_name = gid2grp(target_group).unwrap();
    let link_name = gid2grp(link_group).unwrap();

    at.mkdir("dir");
    at.touch("target");
    at.relative_symlink_file("../target", "dir/link");
    std::os::unix::fs::chown(at.plus("dir"), None, Some(target_group)).unwrap();
    std::os::unix::fs::chown(at.plus("target"), None, Some(target_group)).unwrap();
    std::os::unix::fs::lchown(at.plus("dir/link"), None, Some(link_group)).unwrap();

    // Only the link is in `link_group`: the target is left alone.
    scene
        .ucmd()
        .args(&["-R", "-H", "-v", &format!("--from=:{link_group}")])
        .arg(format!(":{link_group}"))
        .arg("dir")
        .succeeds()
        .stdout_is(format!(
            "ownership of 'dir' retained as {target_name}\n\
             ownership of 'dir/link' retained as {target_name}\n"
        ));
    assert_eq!(at.plus("target").metadata().unwrap().gid(), target_group);

    // The target is in `target_group`, so it is changed.
    scene
        .ucmd()
        .args(&["-R", "-H", "-v", &format!("--from=:{target_group}")])
        .arg(format!(":{link_group}"))
        .arg("dir")
        .succeeds()
        .stdout_is(format!(
            "changed ownership of 'dir' from {user}:{target_name} to {user}:{link_name}\n\
             changed ownership of 'dir/link' from {user}:{target_name} to {user}:{link_name}\n"
        ));
    assert_eq!(at.plus("target").metadata().unwrap().gid(), link_group);
}

/// Under `-R -H` a symlink whose target cannot be stat'd is left alone and
/// reported, whatever `--from` says about the link itself.
#[cfg(all(unix, not(target_os = "openbsd")))]
#[test]
fn test_chown_dangling_symlink_under_big_h() {
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    at.mkdir("dir");
    at.relative_symlink_file("../nowhere", "dir/dangling");
    let gid = at.plus("dir/dangling").symlink_metadata().unwrap().gid();

    for from in [None, Some(format!("--from=:{gid}"))] {
        scene
            .ucmd()
            .args(&["-R", "-H"])
            .args(from.as_slice())
            .arg(format!(":{gid}"))
            .arg("dir")
            .fails_with_code(1)
            .no_stdout()
            .stderr_is("chown: cannot dereference 'dir/dangling': No such file or directory\n");
    }

    // -v says the change was not made, as for an operand.
    let user = chown::entries::uid2usr(rustix::process::geteuid().as_raw()).unwrap();
    let group = chown::entries::gid2grp(at.plus("dir").metadata().unwrap().gid()).unwrap();
    scene
        .ucmd()
        .args(&["-R", "-H", "-v"])
        .arg(format!(":{gid}"))
        .arg("dir")
        .fails_with_code(1)
        .stdout_is(format!(
            "ownership of 'dir' retained as {user}:{group}\n\
             failed to change ownership of 'dir/dangling' to :{gid}\n"
        ))
        .stderr_is("chown: cannot dereference 'dir/dangling': No such file or directory\n");
}

/// Under -R -L -v, an entry that cannot be stat'd is reported as an operand
/// would be.
#[cfg(all(unix, not(target_os = "openbsd")))]
#[test]
fn test_chown_verbose_reports_dangling_symlink_under_big_l() {
    use chown::entries::{gid2grp, uid2usr};

    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("dir");
    at.relative_symlink_file("../nowhere", "dir/dangling");
    let gid = at.plus("dir").metadata().unwrap().gid();
    let user = uid2usr(rustix::process::geteuid().as_raw()).unwrap();
    let group = gid2grp(gid).unwrap();

    ucmd.args(&["-R", "-L", "-v"])
        .arg(format!(":{gid}"))
        .arg("dir")
        .fails_with_code(1)
        .stdout_is(format!(
            "ownership of 'dir' retained as {user}:{group}\n\
             failed to change ownership of 'dir/dangling' to :{gid}\n"
        ))
        .stderr_is("chown: cannot access 'dir/dangling': No such file or directory\n");
}

/// Under -R -v, a directory operand that cannot be read is left alone, whatever
/// --from says, and -v says so. `from` offsets the group --from asks for.
#[cfg(all(unix, not(target_os = "openbsd")))]
#[rstest::rstest]
#[case::no_from(None)]
#[case::matching_from(Some(0))]
#[case::other_from(Some(1))]
fn test_chown_verbose_reports_unreadable_dir_operand(#[case] from: Option<u32>) {
    // Root can read the directory anyway.
    if rustix::process::geteuid().is_root() {
        println!("Test skipped: running as root");
        return;
    }
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("dir");
    let gid = at.plus("dir").metadata().unwrap().gid();
    at.set_mode("dir", 0o000);
    let from = from.map(|offset| format!("--from=:{}", gid.wrapping_add(offset)));

    ucmd.args(&["-R", "-v"])
        .args(from.as_slice())
        .arg(format!(":{gid}"))
        .arg("dir")
        .fails_with_code(1)
        .stdout_is(format!("failed to change ownership of 'dir' to :{gid}\n"))
        .stderr_is("chown: cannot access 'dir': Permission denied\n");
}

/// -R keeps only a few directories open, so a tree deeper than the descriptor
/// limit is still walked in full, up and down.
#[cfg(all(unix, not(target_os = "openbsd")))]
#[test]
fn test_chown_recursive_deeper_than_descriptor_limit() {
    use rlimit::Resource;

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let Some((_, second)) = two_groups(&scene) else {
        return;
    };
    let mut dir = String::from("deep");
    let mut files = Vec::new();
    for _ in 0..100 {
        at.mkdir(&dir);
        at.touch(format!("{dir}/f"));
        files.push(format!("{dir}/f"));
        dir.push_str("/d");
    }

    scene
        .ucmd()
        .args(&["-R", &format!(":{second}"), "deep"])
        .limit(Resource::NOFILE, 32, 32)
        .succeeds()
        .no_output();
    for file in files {
        assert_eq!(at.plus(&file).metadata().unwrap().gid(), second, "{file}");
    }
}

/// Under -L, a directory reached through a symlink is left through the link,
/// not through its "..", also below the directories kept open.
#[cfg(all(unix, not(target_os = "openbsd")))]
#[test]
fn test_chown_recursive_big_l_deep_through_symlink() {
    use rlimit::Resource;

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let Some((_, second)) = two_groups(&scene) else {
        return;
    };
    let mut files = Vec::new();
    for top in ["deep", "other"] {
        let mut dir = String::from(top);
        for _ in 0..40 {
            at.mkdir(&dir);
            at.touch(format!("{dir}/f"));
            files.push(format!("{dir}/f"));
            dir.push_str("/d");
        }
    }
    at.symlink_dir("other", format!("deep{}/link", "/d".repeat(30)));

    scene
        .ucmd()
        .args(&["-R", "-L", &format!(":{second}"), "deep"])
        .limit(Resource::NOFILE, 32, 32)
        .succeeds()
        .no_output();
    for file in files {
        assert_eq!(at.plus(&file).metadata().unwrap().gid(), second, "{file}");
    }
}

/// Two different groups the caller is in, if there are.
#[cfg(all(unix, not(target_os = "openbsd")))]
fn two_groups(scene: &TestScenario) -> Option<(u32, u32)> {
    let groups: Vec<u32> = scene
        .cmd("id")
        .arg("-G")
        .succeeds()
        .stdout_str()
        .split_whitespace()
        .map(|g| g.parse().unwrap())
        .collect();
    let first = *groups.first()?;
    let second = *groups.iter().find(|&&g| g != first)?;
    Some((first, second))
}

/// -H and -L follow a symlinked directory operand also with -h: as with GNU,
/// `--from` is judged on the directory and it is descended into.
#[cfg(all(unix, not(target_os = "openbsd")))]
#[rstest::rstest]
#[case::big_h("-H")]
#[case::big_l("-L")]
fn test_chown_h_judges_symlink_operand_on_its_target(#[case] opt: &str) {
    use std::os::unix::fs::lchown;

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let Some((first, second)) = two_groups(&scene) else {
        return;
    };
    let gid = |path: &str| at.plus(path).symlink_metadata().unwrap().gid();
    at.mkdir("dir");
    at.touch("dir/file");
    at.symlink_dir("dir", "link");
    lchown(at.plus("dir"), None, Some(second)).unwrap();
    lchown(at.plus("dir/file"), None, Some(first)).unwrap();
    lchown(at.plus("link"), None, Some(first)).unwrap();

    // The link is in `first`, but the directory is not: only the file is.
    scene
        .ucmd()
        .args(&["-R", opt, "-h", &format!("--from=:{first}")])
        .arg(format!(":{second}"))
        .arg("link")
        .succeeds()
        .no_output();
    assert_eq!(
        (gid("link"), gid("dir"), gid("dir/file")),
        (first, second, second)
    );
}

/// -H and -L follow a symlinked directory operand also with -h, but as with
/// GNU, the link itself is changed and not the directory.
#[cfg(all(unix, not(target_os = "openbsd")))]
#[rstest::rstest]
#[case::big_h("-H")]
#[case::big_l("-L")]
fn test_chown_h_changes_symlink_operand_itself(#[case] opt: &str) {
    use std::os::unix::fs::lchown;

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let Some((first, second)) = two_groups(&scene) else {
        return;
    };
    let gid = |path: &str| at.plus(path).symlink_metadata().unwrap().gid();
    at.mkdir("dir");
    at.touch("dir/file");
    at.symlink_dir("dir", "link");
    for path in ["dir", "dir/file", "link"] {
        lchown(at.plus(path), None, Some(second)).unwrap();
    }

    scene
        .ucmd()
        .args(&["-R", opt, "-h"])
        .arg(format!(":{first}"))
        .arg("link")
        .succeeds()
        .no_output();
    assert_eq!(
        (gid("link"), gid("dir"), gid("dir/file")),
        (first, second, first)
    );
}

/// With -h, -R -H and -L leave a link to a directory they cannot read alone,
/// as GNU does, as they would the directory.
#[cfg(all(unix, not(target_os = "openbsd")))]
#[rstest::rstest]
#[case::big_h("-H")]
#[case::big_l("-L")]
fn test_chown_h_leaves_link_to_unreadable_dir(#[case] opt: &str) {
    use std::os::unix::fs::lchown;

    // Root can read the directory anyway.
    if rustix::process::geteuid().is_root() {
        println!("Test skipped: running as root");
        return;
    }
    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let Some((first, second)) = two_groups(&scene) else {
        return;
    };
    at.mkdir("dir");
    at.symlink_dir("dir", "link");
    lchown(at.plus("link"), None, Some(first)).unwrap();
    at.set_mode("dir", 0o000);

    scene
        .ucmd()
        .args(&["-R", opt, "-h", "-v"])
        .arg(format!(":{second}"))
        .arg("link")
        .fails_with_code(1)
        .stdout_is(format!(
            "failed to change ownership of 'link' to :{second}\n"
        ))
        .stderr_is("chown: cannot access 'link': Permission denied\n");
    assert_eq!(at.plus("link").symlink_metadata().unwrap().gid(), first);
}

/// Under -L -h a symlink met in the tree is followed for the descent but, as
/// with GNU, changed itself, also with `--from`.
#[cfg(all(unix, not(target_os = "openbsd")))]
#[rstest::rstest]
#[case::all(false)]
#[case::from(true)]
fn test_chown_big_l_with_h_changes_symlinks_in_tree(#[case] with_from: bool) {
    use std::os::unix::fs::lchown;

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let Some((first, second)) = two_groups(&scene) else {
        return;
    };
    let gid = |path: &str| at.plus(path).symlink_metadata().unwrap().gid();
    at.mkdir("dir");
    at.mkdir("other");
    at.touch("other/file");
    at.symlink_dir("other", "dir/link");
    for path in ["dir", "dir/link", "other", "other/file"] {
        lchown(at.plus(path), None, Some(first)).unwrap();
    }
    let from = with_from.then(|| format!("--from=:{first}"));

    scene
        .ucmd()
        .args(&["-R", "-L", "-h"])
        .args(from.as_slice())
        .arg(format!(":{second}"))
        .arg("dir")
        .succeeds()
        .no_output();
    assert_eq!(
        (gid("dir/link"), gid("other"), gid("other/file")),
        (second, first, second)
    );
}

/// Under -L -h, `--from` judges a symlink met in the tree by its target, as GNU
/// does, and then changes the link.
#[cfg(all(unix, not(target_os = "openbsd")))]
#[test]
fn test_chown_big_l_with_h_judges_from_on_link_target() {
    use std::os::unix::fs::lchown;

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let Some((first, second)) = two_groups(&scene) else {
        return;
    };
    at.mkdir("dir");
    at.mkdir("other");
    at.symlink_dir("other", "dir/link");
    for path in ["dir", "other"] {
        lchown(at.plus(path), None, Some(first)).unwrap();
    }
    lchown(at.plus("dir/link"), None, Some(second)).unwrap();

    scene
        .ucmd()
        .args(&["-R", "-L", "-h", &format!("--from=:{first}")])
        .arg(format!(":{first}"))
        .arg("dir")
        .succeeds()
        .no_output();
    assert_eq!(at.plus("dir/link").symlink_metadata().unwrap().gid(), first);
}

/// With -h, -H and -L judge a symlink operand on its target but change the
/// link: a group not asked for stays as the link has it.
#[cfg(all(unix, not(target_os = "openbsd")))]
#[rstest::rstest]
#[case::big_h("-H")]
#[case::big_l("-L")]
fn test_chown_h_keeps_unchanged_group_of_symlink_operand(#[case] opt: &str) {
    use std::os::unix::fs::lchown;

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let Some((first, second)) = two_groups(&scene) else {
        return;
    };
    at.touch("target");
    at.symlink_file("target", "link");
    lchown(at.plus("target"), None, Some(first)).unwrap();
    lchown(at.plus("link"), None, Some(second)).unwrap();
    let uid = rustix::process::geteuid().as_raw();

    scene
        .ucmd()
        .args(&["-R", opt, "-h", &uid.to_string(), "link"])
        .succeeds()
        .no_output();
    assert_eq!(at.plus("link").symlink_metadata().unwrap().gid(), second);
}

/// With -h, -H and -L judge a dangling symlink operand, as GNU does, on the
/// link itself and change it.
#[cfg(all(unix, not(target_os = "openbsd")))]
#[rstest::rstest]
#[case::big_h("-H")]
#[case::big_l("-L")]
fn test_chown_h_changes_dangling_symlink_operand(#[case] opt: &str) {
    use std::os::unix::fs::lchown;

    let scene = TestScenario::new(util_name!());
    let at = &scene.fixtures;
    let Some((first, second)) = two_groups(&scene) else {
        return;
    };
    at.relative_symlink_file("nowhere", "link");
    lchown(at.plus("link"), None, Some(first)).unwrap();

    scene
        .ucmd()
        .args(&["-R", opt, "-h", &format!("--from=:{first}")])
        .arg(format!(":{second}"))
        .arg("link")
        .succeeds()
        .no_output();
    assert_eq!(at.plus("link").symlink_metadata().unwrap().gid(), second);
}

/// With -h, -H and -L fail on a symlink operand that cannot be followed for
/// another reason than a missing target, as GNU does.
#[cfg(all(unix, not(target_os = "openbsd")))]
#[rstest::rstest]
#[case::big_h("-H")]
#[case::big_l("-L")]
fn test_chown_h_fails_on_symlink_loop_operand(#[case] opt: &str) {
    let (at, mut ucmd) = at_and_ucmd!();
    at.relative_symlink_file("loop", "loop");
    let gid = at.plus("loop").symlink_metadata().unwrap().gid();

    #[cfg(all(not(target_env = "musl"), not(target_os = "android")))]
    let expected = "chown: cannot access 'loop': Too many levels of symbolic links\n";
    #[cfg(target_os = "android")]
    let expected = "chown: cannot access 'loop': Too many symbolic links encountered\n";
    #[cfg(target_env = "musl")]
    let expected = "chown: cannot access 'loop': Symbolic link loop\n";

    ucmd.args(&["-R", opt, "-h"])
        .arg(format!(":{gid}"))
        .arg("loop")
        .fails_with_code(1)
        .no_stdout()
        .stderr_is(expected);
}

/// With --from, the file is held open before it is changed: that must neither
/// wait for a writer on a FIFO nor fail on a file the caller cannot read.
#[cfg(all(unix, not(target_os = "openbsd")))]
#[rstest::rstest]
#[case::operands(&["dir/fifo", "dir/file"], &["dir/fifo", "dir/file"])]
#[case::recursive(&["-R", "dir"], &["dir", "dir/fifo", "dir/file"])]
fn test_chown_from_fifo_and_unreadable_file(#[case] args: &[&str], #[case] paths: &[&str]) {
    use chown::entries::{gid2grp, uid2usr};

    // Root can read the file anyway.
    if rustix::process::geteuid().is_root() {
        println!("Test skipped: running as root");
        return;
    }
    let (at, mut ucmd) = at_and_ucmd!();
    at.mkdir("dir");
    at.mkfifo("dir/fifo");
    at.touch("dir/file");
    at.set_mode("dir/file", 0o000);
    let meta = at.plus("dir").metadata().unwrap();
    let owner = format!("{}:{}", meta.uid(), meta.gid());
    let names = format!(
        "{}:{}",
        uid2usr(meta.uid()).unwrap(),
        gid2grp(meta.gid()).unwrap()
    );

    let result = ucmd
        .args(&["-v", &format!("--from={owner}"), &owner])
        .args(args)
        .succeeds();
    // -R lists the entries in directory order; `paths` is sorted.
    let mut lines: Vec<&str> = result.stdout_str().lines().collect();
    lines.sort_unstable();
    let expected: Vec<String> = paths
        .iter()
        .map(|path| format!("ownership of '{path}' retained as {names}"))
        .collect();
    assert_eq!(lines, expected);
}

/// Under -R, where GNU opens no entry, a file the system refuses to open is
/// changed by name. As an operand with --from it fails, as with GNU.
#[cfg(target_vendor = "apple")]
#[test]
fn test_chown_from_file_refused_by_sandbox() {
    let scene = TestScenario::new(util_name!());
    // A process already inside a sandbox cannot apply another profile.
    let probe = ["-p", "(version 1)(allow default)", "true"];
    if !scene.cmd("sandbox-exec").args(&probe).run().succeeded() {
        println!("Test skipped: sandbox-exec cannot apply a profile here");
        return;
    }
    let at = &scene.fixtures;
    at.mkdir("dir");
    at.touch("dir/file");
    let meta = at.plus("dir/file").metadata().unwrap();
    let owner = format!("{}:{}", meta.uid(), meta.gid());
    let from = format!("--from={owner}");
    // Reading the file fails with EPERM, as under TCC or a sandbox.
    let file = at.plus("dir/file").canonicalize().unwrap();
    let profile = format!("(version 1)(allow default)(deny file-read-data (literal {file:?}))");
    let run_denied = |args: &[&str]| {
        scene
            .cmd("sandbox-exec")
            .args(&["-p", &profile])
            .arg(&scene.bin_path)
            .arg("chown")
            .args(args)
            .run()
    };

    run_denied(&["-R", &from, &owner, "dir"])
        .success()
        .no_output();
    run_denied(&[&from, &owner, "dir/file"])
        .code_is(1)
        .no_stdout()
        .stderr_is("chown: changing ownership of 'dir/file': Operation not permitted\n");
}
