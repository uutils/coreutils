use crate::common::util::*;

#[test]
fn test_users_no_arg() {
    new_ucmd!().succeeds();
}

#[test]
#[cfg(any(target_vendor = "apple", target_os = "linux"))]
#[ignore = "issue #3219"]
fn test_users_check_name() {
    #[cfg(target_os = "linux")]
    let util_name = util_name!();
    #[cfg(target_vendor = "apple")]
    let util_name = format!("g{}", util_name!());

    // note: clippy::needless_borrow *false positive*
    #[allow(clippy::needless_borrow)]
    let expected = TestScenario::new(&util_name)
        .cmd_keepenv(util_name)
        .env("LC_ALL", "C")
        .succeeds()
        .stdout_move_str();

    new_ucmd!().succeeds().stdout_is(&expected);
}
