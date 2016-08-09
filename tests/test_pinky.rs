use common::util::*;

static UTIL_NAME: &'static str = "pinky";

fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

extern crate uu_pinky;

pub use self::uu_pinky::*;

#[test]
fn test_capitalize() {
    assert_eq!("Zbnmasd", "zbnmasd".capitalize());
    assert_eq!("Abnmasd", "Abnmasd".capitalize());
    assert_eq!("1masd", "1masd".capitalize());
    assert_eq!("", "".capitalize());
}

#[test]
#[cfg(target_os = "linux")]
fn test_long_format() {
    new_ucmd()
        .arg("-l").arg("root")
        .run()
        .stdout_is("Login name: root                        In real life:  root\nDirectory: /root                        Shell:  /bin/bash\n\n");

    new_ucmd()
        .arg("-lb").arg("root")
        .run()
        .stdout_is("Login name: root                        In real life:  root\n\n");
}

#[test]
#[cfg(target_os = "macos")]
fn test_long_format() {
    new_ucmd()
        .arg("-l").arg("root")
        .run()
        .stdout_is("Login name: root                        In real life:  System Administrator\nDirectory: /var/root                    Shell:  /bin/sh\n\n");

    new_ucmd()
        .arg("-lb").arg("root")
        .run()
        .stdout_is("Login name: root                        In real life:  System Administrator\n\n");
}

#[cfg(target_os = "linux")]
#[test]
fn test_short_format() {
    let scene = TestScenario::new(UTIL_NAME);

    let args = ["-i"];
    scene.ucmd().args(&args).run().stdout_is(expected_result(&args));

    let args = ["-q"];
    scene.ucmd().args(&args).run().stdout_is(expected_result(&args));
}

#[cfg(target_os = "linux")]
fn expected_result(args: &[&str]) -> String {
    TestScenario::new(UTIL_NAME).cmd_keepenv(UTIL_NAME).args(args).run().stdout
}
