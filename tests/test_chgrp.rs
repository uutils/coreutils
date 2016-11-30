use common::util::*;


#[test]
fn test_invalid_option() {
    new_ucmd!()
        .arg("-w")
        .arg("/")
        .fails();
}

static DIR: &'static str = "/tmp";

#[test]
fn test_invalid_group() {
    new_ucmd!()
        .arg("nosuchgroup")
        .arg("/")
        .fails()
        .stderr_is("chgrp: invalid group: nosuchgroup");
}

#[test]
fn test_1() {
    new_ucmd!()
        .arg("bin")
        .arg(DIR)
        .fails()
        .stderr_is("chgrp: changing group of '/tmp': Operation not permitted (os error 1)");
}

#[test]
fn test_fail_silently() {
    for opt in &["-f", "--silent", "--quiet"] {
        new_ucmd!()
            .arg(opt)
            .arg("bin")
            .arg(DIR)
            .run()
            .fails_silently();
    }
}

#[test]
fn test_preserve_root() {
    // It's weird that on OS X, `realpath /etc/..` returns '/private'
    for d in &["/", "/////tmp///../../../../",
               "../../../../../../../../../../../../../../",
               "./../../../../../../../../../../../../../../"] {
        new_ucmd!()
            .arg("--preserve-root")
            .arg("-R")
            .arg("bin").arg(d)
            .fails()
            .stderr_is("chgrp: it is dangerous to operate recursively on '/'\nchgrp: use --no-preserve-root to override this failsafe");
    }
}

#[test]
fn test_preserve_root_symlink() {
    let file = "test_chgrp_symlink2root";
    for d in &["/", "////tmp//../../../../",
               "..//../../..//../..//../../../../../../../../",
               ".//../../../../../../..//../../../../../../../"] {
        let (at, mut ucmd) = at_and_ucmd!();
        at.symlink(d, file);
        ucmd.arg("--preserve-root")
            .arg("-HR")
            .arg("bin").arg(file)
            .fails()
            .stderr_is("chgrp: it is dangerous to operate recursively on '/'\nchgrp: use --no-preserve-root to override this failsafe");
    }

    let (at, mut ucmd) = at_and_ucmd!();
    at.symlink("///usr", file);
    ucmd.arg("--preserve-root")
        .arg("-HR")
        .arg("bin").arg(format!(".//{}/..//..//../../", file))
        .fails()
        .stderr_is("chgrp: it is dangerous to operate recursively on '/'\nchgrp: use --no-preserve-root to override this failsafe");

    let (at, mut ucmd) = at_and_ucmd!();
    at.symlink("/", "/tmp/__root__");
    ucmd.arg("--preserve-root")
        .arg("-R")
        .arg("bin").arg("/tmp/__root__/.")
        .fails()
        .stderr_is("chgrp: it is dangerous to operate recursively on '/'\nchgrp: use --no-preserve-root to override this failsafe");

    use ::std::fs;
    fs::remove_file("/tmp/__root__").unwrap();
}

#[test]
#[cfg(target_os = "linux")]
fn test_reference() {
    new_ucmd!()
        .arg("-v")
        .arg("--reference=/etc/passwd")
        .arg("/etc")
        .fails()
        .stderr_is("chgrp: changing group of '/etc': Operation not permitted (os error 1)\n")
        .stdout_is("failed to change group of /etc from root to root\n");
}

#[test]
#[cfg(target_os = "macos")]
fn test_reference() {
    new_ucmd!()
        .arg("-v")
        .arg("--reference=/etc/passwd")
        .arg("/etc")
        .succeeds();
}

#[test]
#[cfg(target_os = "linux")]
fn test_big_p() {
    new_ucmd!()
        .arg("-RP")
        .arg("bin")
        .arg("/proc/self/cwd")
        .fails()
        .stderr_is("chgrp: changing group of '/proc/self/cwd': Operation not permitted (os error 1)\n");
}

#[test]
#[cfg(target_os = "linux")]
fn test_big_h() {
    assert!(new_ucmd!()
        .arg("-RH")
        .arg("bin")
        .arg("/proc/self/fd")
        .fails()
        .stderr
        .lines()
        .fold(0, |acc, _| acc + 1) > 1);
}
