use crate::common::util::*;

#[cfg(target_os = "linux")]
#[test]
fn test_count() {
    for opt in vec!["-q", "--count"] {
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_boot() {
    for opt in vec!["-b", "--boot"] {
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_heading() {
    for opt in vec!["-H", "--heading"] {
        // allow whitespace variation
        // * minor whitespace differences occur between platform built-in outputs;
        //   specifically number of TABs between "TIME" and "COMMENT" may be variant
        let actual = new_ucmd!().arg(opt).succeeds().stdout_move_str();
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
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_login() {
    for opt in vec!["-l", "--login"] {
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_m() {
    for opt in vec!["-m"] {
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_process() {
    for opt in vec!["-p", "--process"] {
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_runlevel() {
    for opt in vec!["-r", "--runlevel"] {
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_time() {
    for opt in vec!["-t", "--time"] {
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_mesg() {
    for opt in vec!["-w", "-T", "--users", "--message", "--writable"] {
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_arg1_arg2() {
    let scene = TestScenario::new(util_name!());

    let expected = scene
        .cmd_keepenv(util_name!())
        .env("LANGUAGE", "C")
        .arg("am")
        .arg("i")
        .succeeds();

    scene
        .ucmd()
        .arg("am")
        .arg("i")
        .succeeds()
        .stdout_is(expected.stdout_str());
}

#[test]
fn test_too_many_args() {
    let expected =
        "error: The value 'u' was provided to '<FILE>...', but it wasn't expecting any more values";

    new_ucmd!()
        .arg("am")
        .arg("i")
        .arg("u")
        .fails()
        .stderr_contains(expected);
}

#[cfg(target_os = "linux")]
#[test]
fn test_users() {
    for opt in vec!["-u", "--users"] {
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
#[ignore]
fn test_lookup() {
    for opt in vec!["--lookup"] {
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_dead() {
    for opt in vec!["-d", "--dead"] {
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_all_separately() {
    // -a, --all         same as -b -d --login -p -r -t -T -u
    let scene = TestScenario::new(util_name!());

    let expected = scene
        .cmd_keepenv(util_name!())
        .env("LANGUAGE", "C")
        .arg("-b")
        .arg("-d")
        .arg("--login")
        .arg("-p")
        .arg("-r")
        .arg("-t")
        .arg("-T")
        .arg("-u")
        .succeeds();

    scene
        .ucmd()
        .arg("-b")
        .arg("-d")
        .arg("--login")
        .arg("-p")
        .arg("-r")
        .arg("-t")
        .arg("-T")
        .arg("-u")
        .succeeds()
        .stdout_is(expected.stdout_str());

    scene
        .ucmd()
        .arg("--all")
        .succeeds()
        .stdout_is(expected.stdout_str());
}

#[cfg(target_os = "linux")]
#[test]
fn test_all() {
    for opt in vec!["-a", "--all"] {
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(opt));
    }
}

#[cfg(target_os = "linux")]
fn expected_result(arg: &str) -> String {
    TestScenario::new(util_name!())
        .cmd_keepenv(util_name!())
        .env("LANGUAGE", "C")
        .args(&[arg])
        .succeeds()
        .stdout_move_str()
}
