use crate::common::util::*;

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
#[test]
fn test_count() {
    for opt in &["-q", "--count"] {
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(&[opt]));
    }
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
#[test]
fn test_boot() {
    for opt in &["-b", "--boot"] {
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(&[opt]));
    }
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
#[test]
fn test_heading() {
    for opt in &["-H", "--heading"] {
        // allow whitespace variation
        // * minor whitespace differences occur between platform built-in outputs;
        //   specifically number of TABs between "TIME" and "COMMENT" may be variant
        let actual = new_ucmd!().arg(opt).succeeds().stdout_move_str();
        let expect = expected_result(&[opt]);
        println!("actual: {:?}", actual);
        println!("expect: {:?}", expect);
        let v_actual: Vec<&str> = actual.split_whitespace().collect();
        let v_expect: Vec<&str> = expect.split_whitespace().collect();
        assert_eq!(v_actual, v_expect);
    }
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
#[test]
fn test_short() {
    for opt in &["-s", "--short"] {
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(&[opt]));
    }
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
#[test]
fn test_login() {
    for opt in &["-l", "--login"] {
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(&[opt]));
    }
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
#[test]
fn test_m() {
    for opt in &["-m"] {
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(&[opt]));
    }
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
#[test]
fn test_process() {
    for opt in &["-p", "--process"] {
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(&[opt]));
    }
}

#[test]
fn test_runlevel() {
    for opt in &["-r", "--runlevel"] {
        #[cfg(any(target_vendor = "apple", target_os = "linux"))]
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(&[opt]));

        #[cfg(not(target_os = "linux"))]
        new_ucmd!().arg(opt).succeeds().stdout_is("");
    }
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
#[test]
fn test_time() {
    for opt in &["-t", "--time"] {
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(&[opt]));
    }
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
#[test]
fn test_mesg() {
    // -T, -w, --mesg
    //     add user's message status as +, - or ?
    // --message
    //     same as -T
    // --writable
    //     same as -T
    for opt in &["-T", "-w", "--mesg", "--message", "--writable"] {
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(&[opt]));
    }
}

#[test]
fn test_arg1_arg2() {
    let args = ["am", "i"];

    new_ucmd!()
        .args(&args)
        .succeeds()
        .stdout_is(expected_result(&args));
}

#[test]
fn test_too_many_args() {
    const EXPECTED: &str =
        "error: The value 'u' was provided to '<FILE>...', but it wasn't expecting any more values";

    let args = ["am", "i", "u"];
    new_ucmd!().args(&args).fails().stderr_contains(EXPECTED);
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
#[test]
fn test_users() {
    for opt in &["-u", "--users"] {
        let actual = new_ucmd!().arg(opt).succeeds().stdout_move_str();
        let expect = expected_result(&[opt]);
        println!("actual: {:?}", actual);
        println!("expect: {:?}", expect);

        let mut v_actual: Vec<&str> = actual.split_whitespace().collect();
        let mut v_expect: Vec<&str> = expect.split_whitespace().collect();

        // TODO: `--users` differs from GNU's output on macOS
        // Diff < left / right > :
        // <"runner   console      2021-05-20 22:03 00:08         196\n"
        // >"runner   console      2021-05-20 22:03  old          196\n"
        if cfg!(target_os = "macos") {
            v_actual.remove(4);
            v_expect.remove(4);
        }

        assert_eq!(v_actual, v_expect);
    }
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
#[test]
fn test_lookup() {
    let opt = "--lookup";
    new_ucmd!()
        .arg(opt)
        .succeeds()
        .stdout_is(expected_result(&[opt]));
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
#[test]
fn test_dead() {
    for opt in &["-d", "--dead"] {
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(&[opt]));
    }
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
#[test]
fn test_all_separately() {
    if cfg!(target_os = "macos") {
        // TODO: fix `-u`, see: test_users
        return;
    }

    // -a, --all         same as -b -d --login -p -r -t -T -u
    let args = ["-b", "-d", "--login", "-p", "-r", "-t", "-T", "-u"];
    let scene = TestScenario::new(util_name!());
    scene
        .ucmd()
        .args(&args)
        .succeeds()
        .stdout_is(expected_result(&args));
    scene
        .ucmd()
        .arg("--all")
        .succeeds()
        .stdout_is(expected_result(&args));
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
#[test]
fn test_all() {
    if cfg!(target_os = "macos") {
        // TODO: fix `-u`, see: test_users
        return;
    }

    for opt in &["-a", "--all"] {
        new_ucmd!()
            .arg(opt)
            .succeeds()
            .stdout_is(expected_result(&[opt]));
    }
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
fn expected_result(args: &[&str]) -> String {
    #[cfg(target_os = "linux")]
    let util_name = util_name!();
    #[cfg(target_vendor = "apple")]
    let util_name = format!("g{}", util_name!());

    TestScenario::new(&util_name)
        .cmd_keepenv(util_name)
        .env("LANGUAGE", "C")
        .args(args)
        .succeeds()
        .stdout_move_str()
}
