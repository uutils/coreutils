use common::util::*;


use ::std::fs::File;
use ::std::io::BufReader;
use ::std::io::BufRead;

thread_local! {
    static PASSWD: Vec<String> = BufReader::new(File::open("/etc/passwd").unwrap())
        .lines()
        .filter_map(|l| l.ok())
        .filter(|l| l.starts_with("root:"))
        .map(|l| {
            l.split(':').map(|s| s.to_owned()).collect::<Vec<_>>()
        })
        .next().unwrap();
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
fn test_long_format() {
    PASSWD.with(|v| {
        let gecos = v[4].replace("&", &v[4].capitalize());
        new_ucmd!()
            .arg("-l").arg("root")
            .run()
            .stdout_is(format!("Login name: {:<28}In real life:  {}\nDirectory: {:<29}Shell:  {}\n", v[0], gecos, v[5], v[6]));

        new_ucmd!()
            .arg("-lb")
            .arg("root")
            .run()
            .stdout_is(format!("Login name: {:<28}In real life:  {1}\n\n", v[0], gecos));
    })
}

#[cfg(target_os = "linux")]
#[test]
fn test_short_format() {
    let scene = TestScenario::new(util_name!());

    let args = ["-i"];
    scene.ucmd().args(&args).run().stdout_is(expected_result(&args));

    let args = ["-q"];
    scene.ucmd().args(&args).run().stdout_is(expected_result(&args));
}

#[cfg(target_os = "linux")]
fn expected_result(args: &[&str]) -> String {
    TestScenario::new(util_name!()).cmd_keepenv(util_name!()).args(args).run().stdout
}
