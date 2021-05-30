use crate::common::util::*;

#[test]
fn test_users_noarg() {
    new_ucmd!().succeeds();
}

#[test]
#[cfg(any(target_vendor = "apple", target_os = "linux"))]
fn test_users_check_name() {
    #[cfg(target_os = "linux")]
    let util_name = util_name!();
    #[cfg(target_vendor = "apple")]
    let util_name = format!("g{}", util_name!());

    let expected = TestScenario::new(&util_name)
        .cmd_keepenv(util_name)
        .env("LANGUAGE", "C")
        .succeeds()
        .stdout_move_str();

    new_ucmd!().succeeds().stdout_is(&expected);
}
