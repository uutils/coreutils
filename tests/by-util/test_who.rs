//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (flags) runlevel mesg

use crate::common::util::*;

#[cfg(unix)]
#[test]
fn test_count() {
    for opt in &["-q", "--count"] {
        let expected_stdout =
            unwrap_or_return!(expected_result(util_name!(), &[opt])).stdout_move_str();
        new_ucmd!().arg(opt).succeeds().stdout_is(expected_stdout);
    }
}

#[cfg(unix)]
#[test]
fn test_boot() {
    for opt in &["-b", "--boot"] {
        let expected_stdout =
            unwrap_or_return!(expected_result(util_name!(), &[opt])).stdout_move_str();
        new_ucmd!().arg(opt).succeeds().stdout_is(expected_stdout);
    }
}

#[cfg(unix)]
#[test]
fn test_heading() {
    for opt in &["-H", "--heading"] {
        // allow whitespace variation
        // * minor whitespace differences occur between platform built-in outputs;
        //   specifically number of TABs between "TIME" and "COMMENT" may be variant
        let actual = new_ucmd!().arg(opt).succeeds().stdout_move_str();
        let expect = unwrap_or_return!(expected_result(util_name!(), &[opt])).stdout_move_str();
        println!("actual: {:?}", actual);
        println!("expect: {:?}", expect);
        let v_actual: Vec<&str> = actual.split_whitespace().collect();
        let v_expect: Vec<&str> = expect.split_whitespace().collect();
        assert_eq!(v_actual, v_expect);
    }
}

#[cfg(unix)]
#[test]
fn test_short() {
    for opt in &["-s", "--short"] {
        let expected_stdout =
            unwrap_or_return!(expected_result(util_name!(), &[opt])).stdout_move_str();
        new_ucmd!().arg(opt).succeeds().stdout_is(expected_stdout);
    }
}

#[cfg(unix)]
#[test]
fn test_login() {
    for opt in &["-l", "--login"] {
        let expected_stdout =
            unwrap_or_return!(expected_result(util_name!(), &[opt])).stdout_move_str();
        new_ucmd!().arg(opt).succeeds().stdout_is(expected_stdout);
    }
}

#[cfg(unix)]
#[test]
fn test_m() {
    for opt in &["-m"] {
        let expected_stdout =
            unwrap_or_return!(expected_result(util_name!(), &[opt])).stdout_move_str();
        new_ucmd!().arg(opt).succeeds().stdout_is(expected_stdout);
    }
}

#[cfg(unix)]
#[test]
fn test_process() {
    for opt in &["-p", "--process"] {
        let expected_stdout =
            unwrap_or_return!(expected_result(util_name!(), &[opt])).stdout_move_str();
        new_ucmd!().arg(opt).succeeds().stdout_is(expected_stdout);
    }
}

#[cfg(unix)]
#[test]
fn test_runlevel() {
    for opt in &["-r", "--runlevel"] {
        let expected_stdout =
            unwrap_or_return!(expected_result(util_name!(), &[opt])).stdout_move_str();
        new_ucmd!().arg(opt).succeeds().stdout_is(expected_stdout);

        #[cfg(not(target_os = "linux"))]
        new_ucmd!().arg(opt).succeeds().stdout_is("");
    }
}

#[cfg(unix)]
#[test]
fn test_time() {
    for opt in &["-t", "--time"] {
        let expected_stdout =
            unwrap_or_return!(expected_result(util_name!(), &[opt])).stdout_move_str();
        new_ucmd!().arg(opt).succeeds().stdout_is(expected_stdout);
    }
}

#[cfg(unix)]
#[test]
fn test_mesg() {
    // -T, -w, --mesg
    //     add user's message status as +, - or ?
    // --message
    //     same as -T
    // --writable
    //     same as -T
    for opt in &["-T", "-w", "--mesg", "--message", "--writable"] {
        let expected_stdout =
            unwrap_or_return!(expected_result(util_name!(), &[opt])).stdout_move_str();
        new_ucmd!().arg(opt).succeeds().stdout_is(expected_stdout);
    }
}

#[cfg(unix)]
#[test]
fn test_arg1_arg2() {
    let args = ["am", "i"];
    let expected_stdout = unwrap_or_return!(expected_result(util_name!(), &args)).stdout_move_str();

    new_ucmd!()
        .args(&args)
        .succeeds()
        .stdout_is(expected_stdout);
}

#[test]
fn test_too_many_args() {
    const EXPECTED: &str =
        "error: The value 'u' was provided to '<FILE>...', but it wasn't expecting any more values";

    let args = ["am", "i", "u"];
    new_ucmd!().args(&args).fails().stderr_contains(EXPECTED);
}

#[cfg(unix)]
#[test]
fn test_users() {
    for opt in &["-u", "--users"] {
        let actual = new_ucmd!().arg(opt).succeeds().stdout_move_str();
        let expect = unwrap_or_return!(expected_result(util_name!(), &[opt])).stdout_move_str();
        println!("actual: {:?}", actual);
        println!("expect: {:?}", expect);

        let mut v_actual: Vec<&str> = actual.split_whitespace().collect();
        let mut v_expect: Vec<&str> = expect.split_whitespace().collect();

        // TODO: `--users` sometimes differs from GNU's output on macOS (race condition?)
        // actual: "runner   console      Jun 23 06:37 00:34         196\n"
        // expect: "runner   console      Jun 23 06:37  old          196\n"
        if cfg!(target_os = "macos") {
            v_actual.remove(5);
            v_expect.remove(5);
        }

        assert_eq!(v_actual, v_expect);
    }
}

#[cfg(unix)]
#[test]
fn test_lookup() {
    let opt = "--lookup";
    let expected_stdout =
        unwrap_or_return!(expected_result(util_name!(), &[opt])).stdout_move_str();
    new_ucmd!().arg(opt).succeeds().stdout_is(expected_stdout);
}

#[cfg(unix)]
#[test]
fn test_dead() {
    for opt in &["-d", "--dead"] {
        let expected_stdout =
            unwrap_or_return!(expected_result(util_name!(), &[opt])).stdout_move_str();
        new_ucmd!().arg(opt).succeeds().stdout_is(expected_stdout);
    }
}

#[cfg(unix)]
#[test]
fn test_all_separately() {
    if cfg!(target_os = "macos") {
        // TODO: fix `-u`, see: test_users
        return;
    }

    // -a, --all         same as -b -d --login -p -r -t -T -u
    let args = ["-b", "-d", "--login", "-p", "-r", "-t", "-T", "-u"];
    let expected_stdout = unwrap_or_return!(expected_result(util_name!(), &args)).stdout_move_str();
    let scene = TestScenario::new(util_name!());
    scene
        .ucmd()
        .args(&args)
        .succeeds()
        .stdout_is(expected_stdout);
    let expected_stdout =
        unwrap_or_return!(expected_result(util_name!(), &["--all"])).stdout_move_str();
    scene
        .ucmd()
        .arg("--all")
        .succeeds()
        .stdout_is(expected_stdout);
}

#[cfg(unix)]
#[test]
fn test_all() {
    if cfg!(target_os = "macos") {
        // TODO: fix `-u`, see: test_users
        return;
    }

    for opt in &["-a", "--all"] {
        let expected_stdout =
            unwrap_or_return!(expected_result(util_name!(), &[opt])).stdout_move_str();
        new_ucmd!().arg(opt).succeeds().stdout_is(expected_stdout);
    }
}
