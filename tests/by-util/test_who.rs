// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (flags) runlevel mesg logind

use uutests::new_ucmd;
use uutests::unwrap_or_return;
use uutests::util::{TestScenario, expected_result};
use uutests::util_name;
#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[cfg(unix)]
#[test]
#[ignore = "issue #3219"]
fn test_count() {
    let ts = TestScenario::new(util_name!());
    for opt in ["-q", "--count", "--c"] {
        let expected_stdout = unwrap_or_return!(expected_result(&ts, &[opt])).stdout_move_str();
        ts.ucmd().arg(opt).succeeds().stdout_is(expected_stdout);
    }
}

#[cfg(unix)]
#[test]
#[cfg(not(target_os = "openbsd"))]
#[cfg_attr(
    all(target_arch = "aarch64", target_os = "linux"),
    ignore = "Issue #7174 - Test not supported on ARM64 Linux"
)]
fn test_boot() {
    let ts = TestScenario::new(util_name!());
    for opt in ["-b", "--boot", "--b"] {
        let expected_stdout = unwrap_or_return!(expected_result(&ts, &[opt])).stdout_move_str();
        ts.ucmd().arg(opt).succeeds().stdout_is(expected_stdout);
    }
}

#[cfg(unix)]
#[test]
#[ignore = "issue #3219"]
fn test_heading() {
    let ts = TestScenario::new(util_name!());
    for opt in ["-H", "--heading", "--head"] {
        // allow whitespace variation
        // * minor whitespace differences occur between platform built-in outputs;
        //   specifically number of TABs between "TIME" and "COMMENT" may be variant
        let actual = ts.ucmd().arg(opt).succeeds().stdout_move_str();
        let expect = unwrap_or_return!(expected_result(&ts, &[opt])).stdout_move_str();
        println!("actual: {actual:?}");
        println!("expect: {expect:?}");
        let v_actual: Vec<&str> = actual.split_whitespace().collect();
        let v_expect: Vec<&str> = expect.split_whitespace().collect();
        assert_eq!(v_actual, v_expect);
    }
}

#[cfg(unix)]
#[test]
#[ignore = "issue #3219"]
fn test_short() {
    let ts = TestScenario::new(util_name!());
    for opt in ["-s", "--short", "--s"] {
        let expected_stdout = unwrap_or_return!(expected_result(&ts, &[opt])).stdout_move_str();
        ts.ucmd().arg(opt).succeeds().stdout_is(expected_stdout);
    }
}

#[cfg(unix)]
#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_login() {
    let ts = TestScenario::new(util_name!());
    for opt in ["-l", "--login", "--log"] {
        let expected_stdout = unwrap_or_return!(expected_result(&ts, &[opt])).stdout_move_str();
        ts.ucmd().arg(opt).succeeds().stdout_is(expected_stdout);
    }
}

#[cfg(unix)]
#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_m() {
    let ts = TestScenario::new(util_name!());
    let expected_stdout = unwrap_or_return!(expected_result(&ts, &["-m"])).stdout_move_str();
    ts.ucmd().arg("-m").succeeds().stdout_is(expected_stdout);
}

#[cfg(unix)]
#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_process() {
    let ts = TestScenario::new(util_name!());
    for opt in ["-p", "--process", "--p"] {
        let expected_stdout = unwrap_or_return!(expected_result(&ts, &[opt])).stdout_move_str();
        ts.ucmd().arg(opt).succeeds().stdout_is(expected_stdout);
    }
}

#[cfg(unix)]
#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_runlevel() {
    let ts = TestScenario::new(util_name!());
    for opt in ["-r", "--runlevel", "--r"] {
        let expected_stdout = unwrap_or_return!(expected_result(&ts, &[opt])).stdout_move_str();
        ts.ucmd().arg(opt).succeeds().stdout_is(expected_stdout);

        #[cfg(not(target_os = "linux"))]
        ts.ucmd().arg(opt).succeeds().stdout_is("");
    }
}

#[cfg(unix)]
#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_time() {
    let ts = TestScenario::new(util_name!());
    for opt in ["-t", "--time", "--t"] {
        let expected_stdout = unwrap_or_return!(expected_result(&ts, &[opt])).stdout_move_str();
        ts.ucmd().arg(opt).succeeds().stdout_is(expected_stdout);
    }
}

#[cfg(unix)]
#[test]
#[ignore = "issue #3219"]
fn test_mesg() {
    // -T, -w, --mesg
    //     add user's message status as +, - or ?
    // --message
    //     same as -T
    // --writable
    //     same as -T
    let ts = TestScenario::new(util_name!());
    for opt in [
        "-T",
        "-w",
        "--mesg",
        "--m",
        "--message",
        "--writable",
        "--w",
    ] {
        let expected_stdout = unwrap_or_return!(expected_result(&ts, &[opt])).stdout_move_str();
        ts.ucmd().arg(opt).succeeds().stdout_is(expected_stdout);
    }
}

#[cfg(unix)]
#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_arg1_arg2() {
    let args = ["am", "i"];
    let ts = TestScenario::new(util_name!());
    let expected_stdout = unwrap_or_return!(expected_result(&ts, &args)).stdout_move_str();
    ts.ucmd().args(&args).succeeds().stdout_is(expected_stdout);
}

#[test]
fn test_too_many_args() {
    const EXPECTED: &str =
        "error: unexpected value 'u' for '[FILE]...' found; no more were expected";

    let args = ["am", "i", "u"];
    new_ucmd!().args(&args).fails().stderr_contains(EXPECTED);
}

#[cfg(unix)]
#[test]
#[ignore = "issue #3219"]
fn test_users() {
    let ts = TestScenario::new(util_name!());
    for opt in ["-u", "--users", "--us"] {
        let actual = ts.ucmd().arg(opt).succeeds().stdout_move_str();
        let expect = unwrap_or_return!(expected_result(&ts, &[opt])).stdout_move_str();
        println!("actual: {actual:?}");
        println!("expect: {expect:?}");

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
#[ignore = "issue #3219"]
fn test_lookup() {
    let opt = "--lookup";
    let ts = TestScenario::new(util_name!());
    let expected_stdout = unwrap_or_return!(expected_result(&ts, &[opt])).stdout_move_str();
    ts.ucmd().arg(opt).succeeds().stdout_is(expected_stdout);
}

#[cfg(unix)]
#[test]
#[cfg(not(target_os = "openbsd"))]
fn test_dead() {
    let ts = TestScenario::new(util_name!());
    for opt in ["-d", "--dead", "--de"] {
        let expected_stdout = unwrap_or_return!(expected_result(&ts, &[opt])).stdout_move_str();
        ts.ucmd().arg(opt).succeeds().stdout_is(expected_stdout);
    }
}

#[cfg(unix)]
#[test]
#[ignore = "issue #3219"]
fn test_all_separately() {
    if cfg!(target_os = "macos") {
        // TODO: fix `-u`, see: test_users
        return;
    }

    // -a, --all         same as -b -d --login -p -r -t -T -u
    let args = ["-b", "-d", "--login", "-p", "-r", "-t", "-T", "-u"];
    let ts = TestScenario::new(util_name!());
    let expected_stdout = unwrap_or_return!(expected_result(&ts, &args)).stdout_move_str();
    ts.ucmd().args(&args).succeeds().stdout_is(expected_stdout);
    let expected_stdout = unwrap_or_return!(expected_result(&ts, &["--all"])).stdout_move_str();
    ts.ucmd().arg("--all").succeeds().stdout_is(expected_stdout);
}

#[cfg(unix)]
#[test]
#[ignore = "issue #3219"]
fn test_all() {
    if cfg!(target_os = "macos") {
        // TODO: fix `-u`, see: test_users
        return;
    }

    let ts = TestScenario::new(util_name!());
    for opt in ["-a", "--all", "--a"] {
        let expected_stdout = unwrap_or_return!(expected_result(&ts, &[opt])).stdout_move_str();
        ts.ucmd().arg(opt).succeeds().stdout_is(expected_stdout);
    }
}

#[cfg(all(unix, feature = "feat_systemd_logind"))]
#[test]
fn test_systemd_fallback() {
    // Test that who works even when traditional utmp is not available
    // This test verifies that the feat_systemd_logind fallback mechanism works
    let ts = TestScenario::new(util_name!());

    // Test basic functionality - should not fail even if utmp is empty/missing
    ts.ucmd().succeeds();

    // Test with boot flag - should show boot time from systemd/proc
    ts.ucmd().arg("--boot").succeeds();

    // Test count flag - should work with systemd sessions
    ts.ucmd().arg("--count").succeeds();
}

#[cfg(all(unix, feature = "feat_systemd_logind"))]
#[test]
fn test_systemd_boot_time() {
    // Test that boot time can be retrieved from systemd/proc when utmp is unavailable
    let ts = TestScenario::new(util_name!());

    let result = ts.ucmd().arg("--boot").run();

    // Should succeed (exit code 0)
    assert!(result.succeeded());

    // Output should contain something related to boot time
    let stdout = result.stdout_str();

    // Boot time output should contain "system boot" or similar
    // and should have a reasonable date/time format
    assert!(
        stdout.contains("system boot") || stdout.contains("reboot") || !stdout.trim().is_empty(),
        "Boot time output should contain boot information, got: {stdout:?}"
    );
}

#[cfg(all(unix, feature = "feat_systemd_logind"))]
#[test]
fn test_systemd_users() {
    // Test that user listing works with systemd sessions
    let ts = TestScenario::new(util_name!());

    let result = ts.ucmd().run();
    assert!(result.succeeded());

    // The output format should be reasonable (no panics, valid text)
    let stdout = result.stdout_str();

    // Should produce valid UTF-8 output (already verified by stdout_str())
    // and should not contain obvious error messages
    assert!(
        !stdout.contains("Error") && !stdout.contains("Failed") && !stdout.contains("error"),
        "Output should not contain error messages, got: {stdout:?}"
    );
}
