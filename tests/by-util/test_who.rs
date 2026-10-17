// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (flags) runlevel mesg

use uutests::new_ucmd;
#[cfg(unix)]
use uutests::unwrap_or_return;
#[cfg(unix)]
use uutests::util::{TestScenario, expected_result, gnu_cmd_result};
#[cfg(unix)]
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

/// Builds a glibc `utmp` file holding one record of each type that `who`
/// reports, so the per-record code paths can be tested without relying on the
/// contents of the host's `/var/run/utmp`.
///
/// The layout is the one glibc uses on `x86_64`; other targets use different
/// field sizes.
#[cfg(all(target_os = "linux", target_env = "gnu", target_arch = "x86_64"))]
mod utmp_file {
    use uutests::at_and_ucmd;
    use uutests::util::CmdResult;

    const RUN_LVL: i16 = 1;
    const BOOT_TIME: i16 = 2;
    const NEW_TIME: i16 = 3;
    const INIT_PROCESS: i16 = 5;
    const LOGIN_PROCESS: i16 = 6;
    const USER_PROCESS: i16 = 7;
    const DEAD_PROCESS: i16 = 8;

    /// Tue Nov 14 22:13:20 UTC 2023
    const TIMESTAMP: i32 = 1_700_000_000;

    #[derive(Default)]
    struct Record<'a> {
        kind: i16,
        pid: i32,
        line: &'a str,
        id: &'a str,
        user: &'a str,
        host: &'a str,
        exit: (i16, i16),
    }

    impl Record<'_> {
        fn to_bytes(&self) -> Vec<u8> {
            fn field(buf: &mut Vec<u8>, value: &str, len: usize) {
                assert!(value.len() <= len);
                buf.extend_from_slice(value.as_bytes());
                buf.resize(buf.len() + len - value.len(), 0);
            }

            let mut buf = Vec::with_capacity(384);
            buf.extend_from_slice(&self.kind.to_ne_bytes());
            buf.extend_from_slice(&[0; 2]); // padding
            buf.extend_from_slice(&self.pid.to_ne_bytes());
            field(&mut buf, self.line, 32);
            field(&mut buf, self.id, 4);
            field(&mut buf, self.user, 32);
            field(&mut buf, self.host, 256);
            buf.extend_from_slice(&self.exit.0.to_ne_bytes());
            buf.extend_from_slice(&self.exit.1.to_ne_bytes());
            buf.extend_from_slice(&0_i32.to_ne_bytes()); // ut_session
            buf.extend_from_slice(&TIMESTAMP.to_ne_bytes()); // ut_tv.tv_sec
            buf.extend_from_slice(&0_i32.to_ne_bytes()); // ut_tv.tv_usec
            buf.extend_from_slice(&[0; 16]); // ut_addr_v6
            buf.extend_from_slice(&[0; 20]); // reserved
            assert_eq!(buf.len(), 384);
            buf
        }
    }

    /// The run level record stores the previous level in the upper byte of
    /// the pid and the current one in the lower byte.
    fn run_level(previous: u8, current: u8) -> Record<'static> {
        Record {
            kind: RUN_LVL,
            pid: i32::from(previous) * 256 + i32::from(current),
            line: "~",
            id: "~~",
            user: "runlevel",
            ..Record::default()
        }
    }

    fn records() -> Vec<Record<'static>> {
        vec![
            Record {
                kind: BOOT_TIME,
                line: "~",
                id: "~~",
                user: "reboot",
                ..Record::default()
            },
            run_level(b'N', b'5'),
            Record {
                kind: NEW_TIME,
                line: "{",
                user: "date",
                ..Record::default()
            },
            Record {
                kind: INIT_PROCESS,
                pid: 1234,
                line: "tty9",
                id: "si",
                ..Record::default()
            },
            Record {
                kind: LOGIN_PROCESS,
                pid: 2345,
                line: "tty1",
                id: "1",
                user: "LOGIN",
                ..Record::default()
            },
            // The terminal does not exist, so its write state and idle time
            // are reported as unknown.
            Record {
                kind: USER_PROCESS,
                pid: 3456,
                line: "ttyNotThere",
                id: "ts/9",
                user: "alice",
                host: "example.org",
                ..Record::default()
            },
            Record {
                kind: DEAD_PROCESS,
                pid: 4567,
                line: "pts/98",
                id: "ts/8",
                exit: (1, 2),
                ..Record::default()
            },
        ]
    }

    fn run_who(records: &[Record], args: &[&str]) -> CmdResult {
        let (at, mut ucmd) = at_and_ucmd!();
        let bytes: Vec<u8> = records.iter().flat_map(Record::to_bytes).collect();
        at.write_bytes("utmp", &bytes);
        ucmd.env("LC_ALL", "C")
            .env("TZ", "UTC")
            .args(args)
            .arg("utmp")
            .succeeds()
    }

    #[test]
    fn test_default_lists_user_sessions() {
        run_who(&records(), &[]).stdout_is("alice    ttyNotThere  Nov 14 22:13 (example.org)\n");
    }

    #[test]
    fn test_each_selector() {
        let cases = [
            ("-b", "         system boot  Nov 14 22:13\n"),
            (
                "-r",
                "         run-level 5  Nov 14 22:13                   last=S\n",
            ),
            ("-t", "         clock change Nov 14 22:13\n"),
            (
                "-p",
                "         tty9         Nov 14 22:13       1234 id=si\n",
            ),
            (
                "-l",
                "LOGIN    tty1         Nov 14 22:13              2345 id=1\n",
            ),
            (
                "-d",
                "         pts/98       Nov 14 22:13              4567 id=ts/8  term=1 exit=2\n",
            ),
            (
                "-u",
                "alice    ttyNotThere  Nov 14 22:13   ?          3456 (example.org)\n",
            ),
            ("-T", "alice    ? ttyNotThere  Nov 14 22:13 (example.org)\n"),
        ];
        for (arg, expected) in cases {
            run_who(&records(), &[arg]).stdout_is(expected);
        }
    }

    #[test]
    fn test_all() {
        run_who(&records(), &["-a"]).stdout_is(
            "           system boot  Nov 14 22:13
           run-level 5  Nov 14 22:13                   last=S
           clock change Nov 14 22:13
           tty9         Nov 14 22:13              1234 id=si
LOGIN      tty1         Nov 14 22:13              2345 id=1
alice    ? ttyNotThere  Nov 14 22:13   ?          3456 (example.org)
           pts/98       Nov 14 22:13              4567 id=ts/8  term=1 exit=2
",
        );
    }

    #[test]
    fn test_count_lists_only_user_sessions() {
        run_who(&records(), &["-q"]).stdout_contains("alice\n");
    }

    #[test]
    fn test_run_level_reports_previous_level() {
        run_who(&[run_level(b'3', b'5')], &["-r"])
            .stdout_is("         run-level 5  Nov 14 22:13                   last=3\n");
    }

    #[test]
    fn test_run_level_without_previous_level() {
        run_who(&[run_level(0, b'5')], &["-r"]).stdout_is("         run-level 5  Nov 14 22:13\n");
    }
}
