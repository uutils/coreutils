use common::util::*;



#[cfg(target_os = "linux")]
#[test]
fn test_count() {
    for opt in vec!["-q", "--count"] {
        new_ucmd!().arg(opt).run().stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_boot() {
    for opt in vec!["-b", "--boot"] {
        new_ucmd!().arg(opt).run().stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_heading() {
    for opt in vec!["-H"] {
        new_ucmd!().arg(opt).run().stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_short() {
    for opt in vec!["-s", "--short"] {
        new_ucmd!().arg(opt).run().stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_login() {
    for opt in vec!["-l", "--login"] {
        new_ucmd!().arg(opt).run().stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_m() {
    for opt in vec!["-m"] {
        new_ucmd!().arg(opt).run().stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_dead() {
    for opt in vec!["-d", "--dead"] {
        new_ucmd!().arg(opt).run().stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_all() {
    for opt in vec!["-a", "--all"] {
        new_ucmd!().arg(opt).run().stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
fn expected_result(arg: &str) -> String {
    TestScenario::new(util_name!()).cmd_keepenv(util_name!()).args(&[arg]).run().stdout
}
