// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (words) araba newroot userspec chdir pwd's isroot

use uutests::at_and_ucmd;
use uutests::new_ucmd;
#[cfg(not(target_os = "android"))]
use uutests::util::is_ci;
use uutests::util::{run_ucmd_as_root, TestScenario};
use uutests::util_name;

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails().code_is(125);
}

#[test]
fn test_missing_operand() {
    let result = new_ucmd!().fails();

    result.code_is(125);

    assert!(result
        .stderr_str()
        .starts_with("error: the following required arguments were not provided"));

    assert!(result.stderr_str().contains("<newroot>"));
}

#[test]
#[cfg(not(target_os = "android"))]
fn test_enter_chroot_fails() {
    // NOTE: since #2689 this test also ensures that we don't regress #2687
    let (at, mut ucmd) = at_and_ucmd!();

    at.mkdir("jail");

    let result = ucmd.arg("jail").fails();
    result.code_is(125);
    assert!(result
        .stderr_str()
        .starts_with("chroot: cannot chroot to 'jail': Operation not permitted (os error 1)"));
}

#[test]
fn test_no_such_directory() {
    let (at, mut ucmd) = at_and_ucmd!();

    at.touch(at.plus_as_string("a"));

    ucmd.arg("a")
        .fails()
        .stderr_is("chroot: cannot change root directory to 'a': no such directory\n")
        .code_is(125);
}

#[test]
fn test_multiple_group_args() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    at.mkdir("id");

    if let Ok(result) = run_ucmd_as_root(
        &ts,
        &["--groups='invalid ignored'", "--groups=''", "/", "id", "-G"],
    ) {
        result.success().stdout_is("0");
    } else {
        print!("Test skipped; requires root user");
    }
}

#[test]
fn test_invalid_user_spec() {
    let ts = TestScenario::new(util_name!());

    if let Ok(result) = run_ucmd_as_root(&ts, &["--userspec=ARABA:", "/"]) {
        result
            .failure()
            .code_is(125)
            .stderr_is("chroot: invalid user");
    } else {
        print!("Test skipped; requires root user");
    }

    if let Ok(result) = run_ucmd_as_root(&ts, &["--userspec=ARABA:ARABA", "/"]) {
        result
            .failure()
            .code_is(125)
            .stderr_is("chroot: invalid user");
    } else {
        print!("Test skipped; requires root user");
    }

    if let Ok(result) = run_ucmd_as_root(&ts, &["--userspec=:ARABA", "/"]) {
        result
            .failure()
            .code_is(125)
            .stderr_is("chroot: invalid group");
    } else {
        print!("Test skipped; requires root user");
    }
}

#[test]
fn test_invalid_user() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    let dir = "CHROOT_DIR";
    at.mkdir(dir);
    if let Ok(result) = run_ucmd_as_root(&ts, &[dir, "whoami"]) {
        result.success().no_stderr().stdout_is("root");
    } else {
        print!("Test skipped; requires root user");
    }

    // `--user` is an abbreviation of `--userspec`.
    if let Ok(result) = run_ucmd_as_root(&ts, &["--user=nobody:+65535", dir, "pwd"]) {
        result.failure().stderr_is("chroot: invalid user");
    } else {
        print!("Test skipped; requires root user");
    }
}

#[test]
#[cfg(not(target_os = "android"))]
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

    // `--user` is an abbreviation of `--userspec`.
    let result = ucmd
        .arg("a")
        .arg("--user")
        .arg("fake")
        .arg("--groups")
        .arg("ABC,DEF")
        .arg(format!("--userspec={username}:{group_name}"))
        .fails();

    result.code_is(125);

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
    let expected = format!("chroot: failed to run command '{shell}': No such file or directory");

    if let Ok(result) = run_ucmd_as_root(&ts, &[dir]) {
        result.stderr_contains(expected);
    } else {
        print!("Test skipped; requires root user");
    }
}

#[test]
fn test_chroot() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    let dir = "CHROOT_DIR";
    at.mkdir(dir);
    if let Ok(result) = run_ucmd_as_root(&ts, &[dir, "whoami"]) {
        result.success().no_stderr().stdout_is("root");
    } else {
        print!("Test skipped; requires root user");
    }

    if let Ok(result) = run_ucmd_as_root(&ts, &[dir, "pwd"]) {
        result.success().no_stderr().stdout_is("/");
    } else {
        print!("Test skipped; requires root user");
    }
}

#[test]
fn test_chroot_skip_chdir_not_root() {
    let (at, mut ucmd) = at_and_ucmd!();

    let dir = "foobar";
    at.mkdir(dir);

    ucmd.arg("--skip-chdir")
        .arg(dir)
        .fails()
        .stderr_contains("chroot: option --skip-chdir only permitted if NEWROOT is old '/'")
        .code_is(125);
}

#[test]
fn test_chroot_skip_chdir() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;
    let dirs = ["/", "/.", "/..", "isroot"];
    at.symlink_file("/", "isroot");
    for dir in dirs {
        let env_cd = std::env::current_dir().unwrap();
        if let Ok(result) = run_ucmd_as_root(&ts, &[dir, "--skip-chdir"]) {
            // Should return the same path
            assert_eq!(
                result.success().no_stderr().stdout_str(),
                env_cd.to_str().unwrap()
            );
        } else {
            print!("Test skipped; requires root user");
        }
    }
}

#[test]
fn test_chroot_extra_arg() {
    let ts = TestScenario::new(util_name!());
    let at = &ts.fixtures;

    let dir = "CHROOT_DIR";
    at.mkdir(dir);
    let env_cd = std::env::current_dir().unwrap();
    // Verify that -P is pwd's and not chroot
    if let Ok(result) = run_ucmd_as_root(&ts, &[dir, "pwd", "-P"]) {
        assert_eq!(
            result.success().no_stderr().stdout_str(),
            env_cd.to_str().unwrap()
        );
    } else {
        print!("Test skipped; requires root user");
    }
}
