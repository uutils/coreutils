// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore IAMNOTASIGNAL RTMAX RTMIN SIGIO SIGRTMAX GHSA CHLD SIGSTOP taskkill unreaped
use regex::Regex;
#[cfg(windows)]
use std::io::Write;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
#[cfg(windows)]
use std::process::Stdio;
use std::process::{Child, Command, ExitStatus};
#[cfg(windows)]
use uucore::process::Job;
#[cfg(any(target_os = "linux", target_os = "android"))]
use uucore::signals::realtime_signal_bounds;
use uutests::new_ucmd;
use uutests::util::get_tests_binary;

// A child process the tests will try to kill: `<coreutils> sleep 30` via the
// multicall test binary, which exists on every test platform (unlike `sleep`
// from PATH on Windows). The natural death after 30s avoids hanging failing
// tests.
struct Target {
    child: Child,
    killed: bool,
}

impl Target {
    fn new() -> Self {
        Self {
            child: Command::new(get_tests_binary())
                .args(["sleep", "30"])
                .spawn()
                .expect("cannot spawn target"),
            killed: false,
        }
    }

    /// Wait for the target to exit, so `Drop` no longer has to kill it.
    fn reap(&mut self) -> ExitStatus {
        let status = self.child.wait().expect("cannot wait on target");
        self.killed = true;
        status
    }

    /// Reap the target and assert it was killed by `signal` (`128 + n` exit
    /// code on windows).
    fn assert_signaled(&mut self, signal: i32) {
        let status = self.reap();
        #[cfg(unix)]
        assert_eq!(status.signal(), Some(signal));
        #[cfg(windows)]
        assert_eq!(status.code(), Some(128 + signal));
    }

    #[cfg(windows)]
    fn assert_alive(&mut self) {
        assert!(self.child.try_wait().expect("cannot poll target").is_none());
    }

    fn pid(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for Target {
    // Terminates this target to avoid littering test boxes with zombi processes
    // when a test fails after creating a target but before killing it.
    fn drop(&mut self) {
        if !self.killed {
            self.child.kill().expect("cannot kill target");
        }
    }
}

/// A `cmd.exe` in a freshly created Job object, waiting for a command line on
/// its stdin pipe.
///
/// The only safe way to run a *terminating* `kill 0` from a test: `kill 0`
/// signals the killer's immediate job, which here is `job` and nothing else, so
/// it cannot reach the test runner, another test's children, nextest's per-test
/// job or a CI agent — the ambient job is `job`'s *parent*, and a parent's
/// members are not in a child job's list.
///
/// The gate is what makes that true rather than merely likely: `cmd` gets no
/// command until `assign` has succeeded, so the window between `CreateProcess`
/// and `AssignProcessToJobObject` contains nothing that could signal. Fail-safe
/// too — if anything panics before [`JobShell::run`], the pipe closes, `cmd`
/// sees EOF and exits having executed nothing.
#[cfg(windows)]
struct JobShell {
    job: Job,
    shell: Child,
}

#[cfg(windows)]
impl JobShell {
    fn new() -> Self {
        let job = Job::new().expect("cannot create job object");
        let shell = Command::new("cmd")
            // `/d` skips the AutoRun registry command, so a machine that has
            // one cannot run it inside this job or write to the stderr the
            // assertions quote.
            .args(["/d", "/q"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("cannot spawn cmd");
        job.assign(&shell)
            .expect("cannot assign cmd to the fresh job");
        Self { job, shell }
    }

    /// So a group signal from inside reaches `child`.
    fn adopt(&self, child: &Child) {
        self.job
            .assign(child)
            .expect("cannot assign process to the fresh job");
    }

    /// Closes stdin afterwards so cmd exits at EOF rather than blocking forever
    /// if the command fails to kill it.
    fn run(mut self, command: &str) -> (Option<i32>, String) {
        let mut stdin = self.shell.stdin.take().expect("cmd has no stdin pipe");
        write!(stdin, "{command}\r\n").expect("cannot write command");
        drop(stdin);
        let output = self.shell.wait_with_output().expect("cannot wait on cmd");
        (
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    }
}

#[test]
fn test_invalid_arg() {
    new_ucmd!().arg("--definitely-invalid").fails_with_code(1);
}

#[test]
fn test_kill_list_all_signals() {
    // Check for a few signals.  Do not try to be comprehensive.
    new_ucmd!()
        .arg("-l")
        .succeeds()
        .stdout_contains("KILL")
        .stdout_contains("TERM")
        .stdout_contains("HUP")
        .stdout_contains("EXIT");
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_kill_list_contains_realtime_signals() {
    new_ucmd!()
        .arg("-l")
        .succeeds()
        .stdout_contains("RTMIN")
        .stdout_contains("RTMAX");
}

#[test]
fn test_kill_list_final_new_line() {
    let re = Regex::new("\\n$").unwrap();
    assert!(re.is_match(new_ucmd!().arg("-l").succeeds().stdout_str()));
}

#[test]
fn test_kill_list_all_signals_as_table() {
    // Check for a few signals.  Do not try to be comprehensive.
    new_ucmd!()
        .arg("-t")
        .succeeds()
        .stdout_contains("KILL")
        .stdout_contains("TERM")
        .stdout_contains("HUP")
        .stdout_contains("EXIT");
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_kill_table_contains_realtime_signals() {
    new_ucmd!()
        .arg("-t")
        .succeeds()
        .stdout_contains("RTMIN")
        .stdout_contains("RTMAX");
}

#[test]
fn test_kill_table_starts_at_0() {
    new_ucmd!()
        .arg("-t")
        .succeeds()
        .stdout_matches(&Regex::new("^\\s?0\\sEXIT").unwrap());
}

#[test]
fn test_kill_table_lists_all_vertically() {
    // Check for a few signals.  Do not try to be comprehensive.
    let command = new_ucmd!().arg("-t").succeeds();
    let signals = command
        .stdout_str()
        .split('\n')
        .filter_map(|line| line.trim().split(' ').nth(1))
        .collect::<Vec<&str>>();

    assert!(signals.contains(&"KILL"));
    assert!(signals.contains(&"TERM"));
    assert!(signals.contains(&"HUP"));
    assert!(signals.contains(&"EXIT"));
}

#[test]
fn test_kill_list_one_signal_from_number() {
    new_ucmd!()
        .arg("-l")
        .arg("9")
        .succeeds()
        .stdout_only("KILL\n");
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_kill_list_rtmax_from_name() {
    new_ucmd!()
        .arg("-l")
        .arg("RTMAX")
        .succeeds()
        .stdout_only(format!("{}\n", libc::SIGRTMAX()));
}

#[test]
fn test_kill_list_one_signal_from_invalid_number() {
    new_ucmd!()
        .arg("-l")
        .arg("99")
        .fails()
        .stderr_contains("'99': invalid signal");
}

#[test]
fn test_kill_list_one_signal_from_name() {
    // Use SIGKILL because it is 9 on all unixes.
    new_ucmd!()
        .arg("-l")
        .arg("KILL")
        .succeeds()
        .stdout_matches(&Regex::new("\\b9\\b").unwrap());
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_kill_list_sigio_alias() {
    for signal in ["IO", "SIGIO"] {
        new_ucmd!()
            .arg("-l")
            .arg(signal)
            .succeeds()
            .stdout_only(format!("{}\n", libc::SIGIO));
    }
}

#[test]
fn test_kill_list_one_signal_ignore_case() {
    // Use SIGKILL because it is 9 on all unixes.
    new_ucmd!()
        .arg("-l")
        .arg("KiLl")
        .succeeds()
        .stdout_matches(&Regex::new("\\b9\\b").unwrap());
}

#[test]
fn test_kill_list_unknown_must_match_input_case() {
    new_ucmd!()
        .arg("-l")
        .arg("IaMnOtAsIgNaL")
        .fails()
        .stderr_contains("IaMnOtAsIgNaL");
}

#[test]
fn test_kill_list_all_vertically() {
    // Check for a few signals.  Do not try to be comprehensive.
    let command = new_ucmd!().arg("-l").succeeds();
    let signals = command.stdout_str().split('\n').collect::<Vec<&str>>();
    assert!(signals.contains(&"KILL"));
    assert!(signals.contains(&"TERM"));
    assert!(signals.contains(&"HUP"));
    assert!(signals.contains(&"EXIT"));
}

#[test]
fn test_kill_list_two_signal_from_name() {
    new_ucmd!()
        .arg("-l")
        .arg("INT")
        .arg("KILL")
        .succeeds()
        .stdout_matches(&Regex::new("\\d\n\\d").unwrap());
}

#[test]
fn test_kill_list_three_signal_first_unknown() {
    new_ucmd!()
        .arg("-l")
        .arg("IAMNOTASIGNAL")
        .arg("INT")
        .arg("KILL")
        .fails()
        .stderr_contains("'IAMNOTASIGNAL': invalid signal")
        .stdout_matches(&Regex::new("\\d\n\\d").unwrap());
}

#[test]
fn test_kill_set_bad_signal_name() {
    new_ucmd!()
        .arg("-s")
        .arg("IAMNOTASIGNAL")
        .fails()
        .stderr_contains("'IAMNOTASIGNAL': invalid signal");
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_kill_accepts_sigio_alias_for_sending() {
    for args in [["--signal=SIGIO", "999999999"], ["-SIGIO", "999999999"]] {
        new_ucmd!()
            .args(&args)
            .fails_with_code(1)
            .stderr_contains("sending signal")
            .stderr_does_not_contain("invalid signal");
    }
}

#[test]
fn test_kill_out_of_range_signal_is_rejected_not_sent() {
    // An out-of-range signal number must be rejected up front (like GNU), not
    // fall through to be parsed as a negative PID and signalled with the
    // default SIGTERM. Regression for GHSA-3jmh-xh36-pj6v.
    for bad in ["-65", "-129"] {
        let mut target = Target::new();
        new_ucmd!()
            .arg(bad)
            .arg(format!("{}", target.pid()))
            .fails_with_code(1)
            .stderr_contains("invalid signal");
        // The target must have survived: kill it for real and confirm the
        // exit came from std's kill (SIGKILL / code 1), not a stray uu-kill
        // TERM (which would report 143 on windows).
        target.child.kill().expect("cannot kill surviving target");
        let status = target.reap();
        #[cfg(unix)]
        assert_eq!(status.signal(), Some(9));
        #[cfg(windows)]
        assert_eq!(status.code(), Some(1));
    }
}

#[test]
fn test_kill_with_default_signal() {
    let mut target = Target::new();
    new_ucmd!().arg(format!("{}", target.pid())).succeeds();
    target.assert_signaled(15);
}

#[test]
fn test_kill_with_signal_number_old_form() {
    let mut target = Target::new();
    new_ucmd!()
        .arg("-9")
        .arg(format!("{}", target.pid()))
        .succeeds();
    target.assert_signaled(9);
}

#[test]
fn test_kill_with_signal_name_old_form() {
    for arg in ["-Kill", "-KILL"] {
        let mut target = Target::new();
        new_ucmd!()
            .arg(arg)
            .arg(format!("{}", target.pid()))
            .succeeds();
        target.assert_signaled(9);
    }
}

#[test]
fn test_kill_with_lower_case_signal_name_old_form() {
    let target = Target::new();
    new_ucmd!()
        .arg("-kill")
        .arg(format!("{}", target.pid()))
        .fails()
        .stderr_contains("unexpected argument");
}

#[test]
fn test_kill_with_signal_prefixed_name_old_form() {
    let mut target = Target::new();
    new_ucmd!()
        .arg("-SIGKILL")
        .arg(format!("{}", target.pid()))
        .succeeds();
    target.assert_signaled(9);
}

#[test]
fn test_kill_with_signal_number_new_form() {
    let mut target = Target::new();
    new_ucmd!()
        .arg("-s")
        .arg("9")
        .arg(format!("{}", target.pid()))
        .succeeds();
    target.assert_signaled(9);
}

#[test]
fn test_kill_with_signal_name_new_form() {
    let mut target = Target::new();
    new_ucmd!()
        .arg("-s")
        .arg("KILL")
        .arg(format!("{}", target.pid()))
        .succeeds();
    target.assert_signaled(9);
}

#[test]
fn test_kill_with_signal_name_new_form_ignore_case() {
    let mut target = Target::new();
    new_ucmd!()
        .arg("-s")
        .arg("KiLl")
        .arg(format!("{}", target.pid()))
        .succeeds();
    target.assert_signaled(9);
}

#[test]
fn test_kill_with_signal_prefixed_name_new_form() {
    let mut target = Target::new();
    new_ucmd!()
        .arg("-s")
        .arg("SIGKILL")
        .arg(format!("{}", target.pid()))
        .succeeds();
    target.assert_signaled(9);
}

#[test]
fn test_kill_with_signal_prefixed_name_new_form_ignore_case() {
    let mut target = Target::new();
    new_ucmd!()
        .arg("-s")
        .arg("SiGKiLl")
        .arg(format!("{}", target.pid()))
        .succeeds();
    target.assert_signaled(9);
}

#[test]
fn test_kill_with_signal_name_new_form_unknown_must_match_input_case() {
    let target = Target::new();
    new_ucmd!()
        .arg("-s")
        .arg("IaMnOtAsIgNaL")
        .arg(format!("{}", target.pid()))
        .fails()
        .stderr_contains("'IaMnOtAsIgNaL': invalid signal");
}

#[test]
fn test_kill_no_pid_provided() {
    new_ucmd!()
        .fails()
        .stderr_contains("no process ID specified");
}

#[test]
fn test_kill_with_signal_exit_new_form() {
    let target = Target::new();
    new_ucmd!()
        .arg("-s")
        .arg("EXIT")
        .arg(format!("{}", target.pid()))
        .succeeds();
}

#[test]
fn test_kill_with_signal_number_hidden_compatibility_option() {
    let mut target = Target::new();
    new_ucmd!()
        .arg("-n")
        .arg("9")
        .arg(format!("{}", target.pid()))
        .succeeds();
    target.assert_signaled(9);
}

#[test]
fn test_kill_with_signal_and_list() {
    let target = Target::new();
    new_ucmd!()
        .arg("-s")
        .arg("EXIT")
        .arg(format!("{}", target.pid()))
        .arg("-l")
        .fails();
}

#[test]
fn test_kill_with_list_lower_bits() {
    new_ucmd!()
        .arg("-l")
        .arg("128")
        .succeeds()
        .stdout_contains("EXIT");

    new_ucmd!()
        .arg("-l")
        .arg("143")
        .succeeds()
        .stdout_contains("TERM");

    new_ucmd!()
        .arg("-l")
        .arg("256")
        .succeeds()
        .stdout_contains("EXIT");

    new_ucmd!()
        .arg("-l")
        .arg("2304")
        .succeeds()
        .stdout_contains("EXIT");
}

#[test]
fn test_kill_with_list_lower_bits_unrecognized() {
    new_ucmd!().arg("-l").arg("111").fails();
    new_ucmd!().arg("-l").arg("384").fails();
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_kill_with_list_unnamed_signal_numbers() {
    new_ucmd!()
        .arg("-l")
        .arg("32")
        .succeeds()
        .stdout_only("32\n");
    new_ucmd!()
        .arg("-l")
        .arg("33")
        .succeeds()
        .stdout_only("33\n");
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_kill_with_list_all_signal_numbers_up_to_last_named_signal() {
    let last_signal_name = new_ucmd!()
        .arg("-l")
        .succeeds()
        .stdout_str()
        .lines()
        .last()
        .unwrap()
        .to_string();

    let last_signal_number: usize = new_ucmd!()
        .arg("-l")
        .arg("--")
        .arg(&last_signal_name)
        .succeeds()
        .stdout_str()
        .trim()
        .parse()
        .unwrap();

    let args = std::iter::once(String::from("--"))
        .chain((0..=last_signal_number).map(|signal| signal.to_string()))
        .collect::<Vec<_>>();

    new_ucmd!().arg("-l").args(&args).succeeds();
}

#[test]
fn test_kill_with_signal_and_table() {
    let target = Target::new();
    new_ucmd!()
        .arg("-s")
        .arg("EXIT")
        .arg(format!("{}", target.pid()))
        .arg("-t")
        .fails();
}

// Listing signals to a full device must report the write error and exit
// non-zero, not panic/abort. Covers -l, -l <name>, --list <number>, and --table.
#[cfg(target_os = "linux")]
#[test]
fn test_kill_list_signals_write_error_is_reported() {
    for args in [
        vec!["-l"],
        vec!["-l", "TERM"],
        vec!["--list", "9"],
        vec!["--table"],
    ] {
        new_ucmd!()
            .args(&args)
            .set_stdout(std::fs::File::create("/dev/full").unwrap())
            .fails()
            .stderr_is("kill: write error: No space left on device\n");
    }
}

/// Test that `kill -1` (signal without PID) reports "no process ID" error
/// instead of being misinterpreted as pid=-1 which would kill all processes.
/// This matches GNU kill behavior.
#[test]
fn test_kill_signal_only_no_pid() {
    // Test with -1 (SIGHUP)
    new_ucmd!()
        .arg("-1")
        .fails()
        .stderr_contains("no process ID specified");

    // Test with -9 (SIGKILL)
    new_ucmd!()
        .arg("-9")
        .fails()
        .stderr_contains("no process ID specified");

    // Test with -TERM
    new_ucmd!()
        .arg("-TERM")
        .fails()
        .stderr_contains("no process ID specified");
}

#[test]
fn test_kill_signal_zero_process() {
    let target = Target::new();
    // kill -0 should succeed for a running process (signal 0 = existence check)
    new_ucmd!()
        .arg("-0")
        .arg(format!("{}", target.pid()))
        .succeeds();
}

#[test]
fn test_kill_signal_zero_new_form() {
    let target = Target::new();
    // kill -s 0 should also work
    new_ucmd!()
        .arg("-s")
        .arg("0")
        .arg(format!("{}", target.pid()))
        .succeeds();
}

#[test]
fn test_kill_signal_zero_nonexistent() {
    // kill -0 with a nonexistent PID should fail
    new_ucmd!().arg("-0").arg("999999999").fails();
}

#[test]
fn test_kill_signal_zero_current_process_group() {
    // Signal 0 never terminates anything (windows opens group members with
    // SYNCHRONIZE only), so this is safe un-isolated. It proves only that the
    // enumeration does not error: probing yourself always succeeds, so it would
    // pass on an empty pid list too.
    new_ucmd!().arg("-0").arg("0").succeeds();
}

#[cfg(windows)]
#[test]
fn test_kill_windows_help_has_platform_notes() {
    // Every asserted phrase sits entirely inside one `kill-after-help-windows`
    // continuation line: clap's `wrap_help` re-wraps lines longer than the
    // terminal width but never joins short ones, so the .ftl line breaks
    // survive verbatim and these assertions cannot straddle a break.
    new_ucmd!()
        .arg("--help")
        .succeeds()
        .stdout_contains("Windows notes")
        .stdout_contains("PID 0 targets the Job object")
        .stdout_contains("Outside a job")
        .stdout_contains("a Job object is usually not yours");
}

#[cfg(windows)]
#[test]
fn test_kill_windows_int_and_hup_terminate_directly() {
    for (name, sig) in [("INT", 2), ("HUP", 1)] {
        let mut target = Target::new();
        new_ucmd!()
            .args(&["-s", name, &target.pid().to_string()])
            .succeeds();
        target.assert_signaled(sig);
    }
}

#[cfg(windows)]
#[test]
fn test_kill_windows_ignored_signals_are_noops() {
    let mut target = Target::new();
    for sig in ["CHLD", "CONT"] {
        new_ucmd!()
            .args(&["-s", sig, &target.pid().to_string()])
            .succeeds();
    }
    target.assert_alive();
}

#[cfg(windows)]
#[test]
fn test_kill_windows_stop_is_rejected() {
    let mut target = Target::new();
    new_ucmd!()
        .args(&["-s", "STOP", &target.pid().to_string()])
        .fails_with_code(1)
        .stderr_contains("unsupported signal");
    target.assert_alive();
}

#[cfg(windows)]
#[test]
fn test_kill_windows_negative_pid_unsupported() {
    // `-1` is the dangerous one on unix ("every process you may signal"); on
    // windows it must be inert, and `u32::try_from` fails before any Win32 call.
    new_ucmd!()
        .args(&["-9", "-1"])
        .fails_with_code(1)
        .stderr_contains("a negative PID");

    let mut target = Target::new();
    new_ucmd!()
        .arg("--")
        .arg(format!("-{}", target.pid()))
        .fails_with_code(1)
        .stderr_contains("a negative PID");
    target.assert_alive();
}

/// `kill 0` must reach *other* members of the job, not just the caller.
///
/// The victim is the proof: it is neither the killer's parent nor its child, so
/// the only path by which it can die is the job pid list. An implementation
/// that regressed to "signal only myself" would still make cmd exit 137, so the
/// exit code alone would not catch it.
#[cfg(windows)]
#[test]
fn test_kill_windows_pid_zero_terminates_the_whole_job() {
    for (args, signal) in [("-9 0", 9), ("0", 15)] {
        let shell = JobShell::new();
        let mut victim = Target::new();
        shell.adopt(&victim.child);

        // Only now can anything in the job run.
        let (code, stderr) = shell.run(&format!("\"{}\" kill {args}", get_tests_binary()));

        // Read cmd's status first, so a broken kill surfaces its own message
        // rather than only a 30-second victim timeout below.
        assert_eq!(
            code,
            Some(128 + signal),
            "cmd survived `kill {args}`; kill said: {stderr}"
        );
        victim.assert_signaled(signal);
    }
}

// The "not in a job" fallback has no integration test on purpose: nothing can
// guarantee a spawned process is job-less. `CREATE_BREAKAWAY_FROM_JOB` detaches
// only from the immediate job, so under a nested chain (`cargo nextest` inside
// `cargo`) the child stays in an ancestor job and `kill -9 0` terminates the
// test runner — an earlier version of this test did exactly that. Covered by
// `with_self_last` in uucore instead, whose empty-member case is that case.

#[cfg(windows)]
#[test]
fn test_kill_windows_nonexistent_pid_no_such_process() {
    new_ucmd!()
        .arg("999999999")
        .fails_with_code(1)
        .stderr_contains("No such process");
    new_ucmd!()
        .args(&["-0", "999999999"])
        .fails_with_code(1)
        .stderr_contains("No such process");
}

#[cfg(windows)]
#[test]
fn test_kill_windows_exited_target_with_held_handle() {
    // The Child handle pins the exited process object: terminating still
    // succeeds (unix kill-on-unreaped parity) while -0 reports it gone.
    let mut target = Target::new();
    target.child.kill().expect("cannot kill target");
    target.reap();
    let pid = target.pid().to_string();
    new_ucmd!().arg(&pid).succeeds();
    new_ucmd!()
        .args(&["-0", &pid])
        .fails_with_code(1)
        .stderr_contains("No such process");
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_kill_realtime_signal() {
    let mut target = Target::new();
    // kill -s RTMIN should send SIGRTMIN and terminate the process
    new_ucmd!()
        .arg("-s")
        .arg("RTMIN")
        .arg(format!("{}", target.pid()))
        .succeeds();
    target.assert_signaled(libc::SIGRTMIN());
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_kill_with_rtmax_offset() {
    let (_, rtmax) = realtime_signal_bounds().unwrap();
    let sig: i32 = (rtmax as i32) - 7;

    let mut target = Target::new();
    new_ucmd!()
        .arg("-s")
        .arg("SIGRTMAX-7")
        .arg(format!("{}", target.pid()))
        .succeeds();
    target.assert_signaled(sig);
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_kill_with_rtmin_offset() {
    let (rtmin, _) = realtime_signal_bounds().unwrap();
    let sig: i32 = (rtmin as i32) + 7;

    let mut target = Target::new();
    new_ucmd!()
        .arg("-s")
        .arg("SIGRTMIN+7")
        .arg(format!("{}", target.pid()))
        .succeeds();
    target.assert_signaled(sig);
}
