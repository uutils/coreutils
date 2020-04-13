#[cfg(target_os = "linux")]
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
        // allow whitespace variation
        // * minor whitespace differences occur between platform built-in outputs; specfically number of TABs between "TIME" and "COMMENT" may be variant
        let actual = new_ucmd!().arg(opt).run().stdout;
        let expect = expected_result(opt);
        println!("actual: {:?}", actual);
        println!("expect: {:?}", expect);
        let v_actual: Vec<&str> = actual.split_whitespace().collect();
        let v_expect: Vec<&str> = expect.split_whitespace().collect();
        assert_eq!(v_actual, v_expect);
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
    TestScenario::new(util_name!()).cmd_keepenv(util_name!()).env("LANGUAGE", "C").args(&[arg]).run().stdout
}
