use common::util::*;

static UTIL_NAME: &'static str = "who";

fn new_ucmd() -> UCommand {
    TestScenario::new(UTIL_NAME).ucmd()
}

#[cfg(target_os = "linux")]
#[test]
fn test_count() {
    for opt in vec!["-q", "--count"] {
        new_ucmd().arg(opt).run().stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_boot() {
    for opt in vec!["-b", "--boot"] {
        new_ucmd().arg(opt).run().stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_heading() {
    for opt in vec!["-H"] {
        new_ucmd().arg(opt).run().stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_short() {
    for opt in vec!["-s", "--short"] {
        new_ucmd().arg(opt).run().stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_login() {
    for opt in vec!["-l", "--login"] {
        new_ucmd().arg(opt).run().stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_m() {
    for opt in vec!["-m"] {
        new_ucmd().arg(opt).run().stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_dead() {
    for opt in vec!["-d", "--dead"] {
        new_ucmd().arg(opt).run().stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_all() {
    for opt in vec!["-a", "--all"] {
        new_ucmd().arg(opt).run().stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
fn expected_result(arg: &str) -> String {
    TestScenario::new(UTIL_NAME).cmd_keepenv(UTIL_NAME).args(&[arg]).run().stdout
}
