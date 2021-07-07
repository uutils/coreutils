use crate::common::util::*;

const VERSION_MIN_MULTIPLE_USERS: &str = "8.31"; // this feature was introduced in GNU's coreutils 8.31

#[test]
#[cfg(unix)]
fn test_groups() {
    let result = new_ucmd!().run();
    let exp_result = unwrap_or_return!(expected_result(util_name!(), &[]));

    result
        .stdout_is(exp_result.stdout_str())
        .stderr_is(exp_result.stderr_str())
        .code_is(exp_result.code());
}

#[test]
#[cfg(unix)]
fn test_groups_username() {
    let test_users = [&whoami()[..]];

    let result = new_ucmd!().args(&test_users).run();
    let exp_result = unwrap_or_return!(expected_result(util_name!(), &test_users));

    result
        .stdout_is(exp_result.stdout_str())
        .stderr_is(exp_result.stderr_str())
        .code_is(exp_result.code());
}

#[test]
#[cfg(unix)]
fn test_groups_username_multiple() {
    unwrap_or_return!(check_coreutil_version(
        util_name!(),
        VERSION_MIN_MULTIPLE_USERS
    ));
    let test_users = ["root", "man", "postfix", "sshd", &whoami()];

    let result = new_ucmd!().args(&test_users).run();
    let exp_result = unwrap_or_return!(expected_result(util_name!(), &test_users));

    result
        .stdout_is(exp_result.stdout_str())
        .stderr_is(exp_result.stderr_str())
        .code_is(exp_result.code());
}
