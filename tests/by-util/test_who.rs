// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (flags) runlevel mesg

#[cfg(all(target_os = "linux", target_env = "gnu"))]
use uucore::utmpx;
#[cfg(all(target_os = "linux", target_env = "gnu"))]
use uutests::at_and_ucmd;
use uutests::new_ucmd;
use uutests::unwrap_or_return;
use uutests::util::{TestScenario, expected_result, gnu_cmd_result};
use uutests::util_name;

#[cfg(all(target_os = "linux", target_env = "gnu"))]
struct LinuxUtmpRecord {
    record_type: i16,
    pid: i32,
    line: [u8; 32],
    id: [u8; 4],
    user: [u8; 32],
    host: [u8; 256],
    termination: i16,
    exit: i16,
    timestamp: i32,
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
impl LinuxUtmpRecord {
    fn new(record_type: i16, pid: i32, line: &str, id: &str, user: &str, host: &str) -> Self {
        Self {
            record_type,
            pid,
            line: fixed_field(line),
            id: fixed_field(id),
            user: fixed_field(user),
            host: fixed_field(host),
            termination: 0,
            exit: 0,
            timestamp: 1_716_371_201,
        }
    }

    fn with_exit_status(mut self, termination: i16, exit: i16) -> Self {
        self.termination = termination;
        self.exit = exit;
        self
    }

    fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(384);
        bytes.extend_from_slice(&self.record_type.to_ne_bytes());
        // glibc pads ut_type to align the following pid field.
        bytes.extend_from_slice(&[0; 2]);
        bytes.extend_from_slice(&self.pid.to_ne_bytes());
        bytes.extend_from_slice(&self.line);
        bytes.extend_from_slice(&self.id);
        bytes.extend_from_slice(&self.user);
        bytes.extend_from_slice(&self.host);
        bytes.extend_from_slice(&self.termination.to_ne_bytes());
        bytes.extend_from_slice(&self.exit.to_ne_bytes());
        bytes.extend_from_slice(&0_i32.to_ne_bytes());
        bytes.extend_from_slice(&self.timestamp.to_ne_bytes());
        bytes.extend_from_slice(&0_i32.to_ne_bytes());
        for _ in 0..4 {
            bytes.extend_from_slice(&0_i32.to_ne_bytes());
        }
        bytes.extend_from_slice(&[0; 20]);
        assert_eq!(bytes.len(), 384);
        bytes
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn fixed_field<const N: usize>(value: &str) -> [u8; N] {
    assert!(value.len() <= N);
    let mut field = [0; N];
    field[..value.len()].copy_from_slice(value.as_bytes());
    field
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn write_linux_utmp(path: &std::path::Path, records: &[LinuxUtmpRecord]) {
    let bytes: Vec<_> = records.iter().flat_map(LinuxUtmpRecord::encode).collect();
    std::fs::write(path, bytes).unwrap();
}

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
        ts.ucmd().arg(opt).succeeds().no_output();
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
        if cfg!(target_vendor = "apple") {
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
    if cfg!(target_vendor = "apple") {
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
    if cfg!(target_vendor = "apple") {
        // TODO: fix `-u`, see: test_users
        return;
    }

    let ts = TestScenario::new(util_name!());
    for opt in ["-a", "--all", "--a"] {
        let expected_stdout = unwrap_or_return!(expected_result(&ts, &[opt])).stdout_move_str();
        ts.ucmd().arg(opt).succeeds().stdout_is(expected_stdout);
    }
}

#[cfg(unix)]
#[test]
#[ignore = "issue #3219"]
fn test_locale() {
    let ts = TestScenario::new(util_name!());

    let expected_stdout =
        unwrap_or_return!(gnu_cmd_result(&ts, &[], &[("LC_ALL", "C")])).stdout_move_str();
    ts.ucmd()
        .env("LC_ALL", "C")
        .succeeds()
        .stdout_is(&expected_stdout);

    let expected_stdout =
        unwrap_or_return!(gnu_cmd_result(&ts, &[], &[("LC_ALL", "en_US.UTF-8")])).stdout_move_str();
    ts.ucmd()
        .env("LC_ALL", "C")
        .succeeds()
        .stdout_str_check(|s| s != expected_stdout);
    ts.ucmd()
        .env("LC_ALL", "en_US.UTF-8")
        .succeeds()
        .stdout_is(&expected_stdout);
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[test]
#[cfg_attr(
    target_arch = "aarch64",
    ignore = "Issue #7174 - Test not supported on ARM64 Linux"
)]
fn test_records_from_file() {
    let (at, mut ucmd) = at_and_ucmd!();
    let path = at.plus("who.utmp");
    let runlevel_pid = i32::from(b'N') * 256 + i32::from(b'3');
    let records = [
        LinuxUtmpRecord::new(utmpx::RUN_LVL, runlevel_pid, "~", "~~", "runlevel", ""),
        LinuxUtmpRecord::new(utmpx::BOOT_TIME, 0, "~", "~~", "reboot", "kernel"),
        LinuxUtmpRecord::new(utmpx::NEW_TIME, 0, "}", "", "", ""),
        LinuxUtmpRecord::new(utmpx::OLD_TIME, 0, "|", "", "", ""),
        LinuxUtmpRecord::new(utmpx::INIT_PROCESS, 105, "ttyI", "i1", "", ""),
        LinuxUtmpRecord::new(utmpx::LOGIN_PROCESS, 106, "ttyL", "l1", "", ""),
        LinuxUtmpRecord::new(
            utmpx::USER_PROCESS,
            107,
            "missing-tty",
            "u1",
            "alice",
            "localhost",
        ),
        LinuxUtmpRecord::new(utmpx::USER_PROCESS, 108, "null", "u2", "bob", ""),
        LinuxUtmpRecord::new(utmpx::DEAD_PROCESS, 109, "ttyD", "d1", "", "").with_exit_status(9, 4),
        LinuxUtmpRecord::new(utmpx::ACCOUNTING, 110, "ignored", "x1", "ignored", ""),
    ];
    write_linux_utmp(&path, &records);

    ucmd.args(&["--all", "--heading"])
        .arg(&path)
        .env("LC_ALL", "C")
        .succeeds()
        .stdout_contains("NAME")
        .stdout_contains("run-level 3")
        .stdout_contains("last=S")
        .stdout_contains("system boot")
        .stdout_contains("clock change")
        .stdout_contains("ttyI")
        .stdout_contains("LOGIN")
        .stdout_contains("alice")
        .stdout_contains("(localhost)")
        .stdout_contains("bob")
        .stdout_contains("term=9 exit=4")
        .stdout_does_not_contain("ignored");

    new_ucmd!()
        .args(&["--count"])
        .arg(&path)
        .env("LC_ALL", "C")
        .succeeds()
        .stdout_is("alice bob\n# users=2\n");

    new_ucmd!()
        .arg("--lookup")
        .arg(&path)
        .env("LC_ALL", "C")
        .succeeds()
        .stdout_contains("alice")
        .stdout_contains("localhost");

    new_ucmd!()
        .arg(&path)
        .env("LC_ALL", "C.UTF-8")
        .succeeds()
        .stdout_contains("alice")
        .stdout_does_not_contain("system boot");
}

#[cfg(target_os = "linux")]
#[test]
fn test_piped_to_dev_full() {
    let ts = TestScenario::new(util_name!());

    let dev_full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();

    ts.ucmd()
        .arg("--heading")
        .set_stdout(dev_full)
        .fails()
        .stderr_is("who: No space left on device\n");
}

// `-q` took a separate branch that printed with `println!`, which aborts the
// process on a write error instead of reporting it (#13388).
#[cfg(target_os = "linux")]
#[test]
fn test_short_list_piped_to_dev_full() {
    let ts = TestScenario::new(util_name!());

    let dev_full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();

    ts.ucmd()
        .arg("-q")
        .set_stdout(dev_full)
        .fails()
        .stderr_is("who: No space left on device\n");
}
