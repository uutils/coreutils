use common::util::*;

static UTIL_NAME: &'static str = "pinky";

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
    let (_, mut ucmd) = testing(UTIL_NAME);
    ucmd.arg("-l").arg("root");
    let expected = "Login name: root                        In real life:  root\nDirectory: /root                        Shell:  /bin/bash\n\n";
    assert_eq!(expected, ucmd.run().stdout);

    let (_, mut ucmd) = testing(UTIL_NAME);
    ucmd.arg("-lb").arg("root");
    let expected = "Login name: root                        In real life:  root\n\n";
    assert_eq!(expected, ucmd.run().stdout);
}

#[test]
#[cfg(target_os = "macos")]
fn test_long_format() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    ucmd.arg("-l").arg("root");
    let expected = "Login name: root                        In real life:  System Administrator\nDirectory: /var/root                    Shell:  /bin/sh\n\n";
    assert_eq!(expected, ucmd.run().stdout);

    let (_, mut ucmd) = testing(UTIL_NAME);
    ucmd.arg("-lb").arg("root");
    let expected = "Login name: root                        In real life:  System Administrator\n\n";
    assert_eq!(expected, ucmd.run().stdout);
}

#[cfg(target_os = "linux")]
#[test]
#[ignore]
fn test_short_format() {
    let (_, mut ucmd) = testing(UTIL_NAME);
    let args = ["-s"];
    ucmd.args(&args);
    assert_eq!(expected_result(&args), ucmd.run().stdout);

    let (_, mut ucmd) = testing(UTIL_NAME);
    let args = ["-f"];
    ucmd.args(&args);
    assert_eq!(expected_result(&args), ucmd.run().stdout);

    let (_, mut ucmd) = testing(UTIL_NAME);
    let args = ["-w"];
    ucmd.args(&args);
    assert_eq!(expected_result(&args), ucmd.run().stdout);

    let (_, mut ucmd) = testing(UTIL_NAME);
    let args = ["-i"];
    ucmd.args(&args);
    assert_eq!(expected_result(&args), ucmd.run().stdout);

    let (_, mut ucmd) = testing(UTIL_NAME);
    let args = ["-q"];
    ucmd.args(&args);
    assert_eq!(expected_result(&args), ucmd.run().stdout);
}

#[cfg(target_os = "linux")]
fn expected_result(args: &[&str]) -> String {
    use std::process::Command;

    let output = Command::new(UTIL_NAME).args(args).output().unwrap();
    String::from_utf8_lossy(&output.stdout).into_owned()
}
