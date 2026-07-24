// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (win-api) WAITABLE Waitable PHANDLER unsignaled
// spell-checker:ignore (signals) CHLD TSTP TTIN TTOU WINCH ESRCH
// spell-checker:ignore catchable targetable wakeup

//! Windows emulation of POSIX signal delivery for child processes.
//!
//! Windows has no signals, so this module emulates the POSIX default
//! dispositions with native primitives: signal numbers follow the Linux
//! layout (matching `uucore::signals::ALL_SIGNALS`), "terminate" signals
//! force-exit with exit code `128 + n`, and `INT`/`QUIT` map to a
//! `CTRL_BREAK_EVENT` on a console process group. [`Job`] gives process-tree
//! termination, and [`enable_ctrl_forwarding`]/[`take_last_ctrl_signal`]
//! surface console control events (Ctrl-C, Ctrl-Break, close) as POSIX signal
//! numbers for forwarding. All raw Win32 calls live in the safe [`sys`]
//! wrappers.

use std::io;
use std::os::windows::io::{AsHandle, BorrowedHandle, OwnedHandle};
use std::process::{Child, Command, ExitStatus};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::time::Duration;

use windows_sys::Win32::System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT, CTRL_CLOSE_EVENT};
use windows_sys::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, INFINITE};
use windows_sys::core::BOOL;

use super::ChildExt;

/// Safe wrappers around the raw Win32 calls used for process control: each
/// validates results into [`io::Error`] and takes
/// [`BorrowedHandle`]/[`OwnedHandle`] so callers never touch raw `HANDLE`s.
pub mod sys {
    use std::io;
    use std::os::windows::io::{AsRawHandle, BorrowedHandle, HandleOrNull, OwnedHandle};

    use windows_sys::Win32::Foundation::{
        FALSE, HANDLE, TRUE, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT,
    };
    use windows_sys::Win32::System::Console::{
        GenerateConsoleCtrlEvent, PHANDLER_ROUTINE, SetConsoleCtrlHandler,
    };
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, TerminateJobObject,
    };
    use windows_sys::Win32::System::Threading::{
        CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, CreateEventW, CreateWaitableTimerExW, ResetEvent,
        SetEvent, SetWaitableTimer, TIMER_ALL_ACCESS, TerminateProcess, WaitForMultipleObjects,
        WaitForSingleObject,
    };
    use windows_sys::core::BOOL;

    /// The outcome of a successful bounded wait.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum WaitOutcome {
        /// The object at this index in the wait set became signaled.
        Object(u32),
        /// The wait interval elapsed first.
        TimedOut,
    }

    /// Convert a Win32 `BOOL` result into an [`io::Result`]: zero (`FALSE`)
    /// becomes the error from `GetLastError`.
    fn cvt(result: BOOL) -> io::Result<()> {
        if result == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Take ownership of a `HANDLE` from a Win32 create-function, treating
    /// null as failure.
    ///
    /// # Safety
    ///
    /// `raw` must be either null or a valid handle exclusively owned by the
    /// caller (i.e. freshly returned by a Win32 function that transfers
    /// ownership).
    unsafe fn cvt_created_handle(raw: HANDLE) -> io::Result<OwnedHandle> {
        // SAFETY: per this function's contract, `raw` is null or owned.
        OwnedHandle::try_from(unsafe { HandleOrNull::from_raw_handle(raw) })
            .map_err(|_| io::Error::last_os_error())
    }

    /// Create an anonymous job object.
    pub fn create_job_object() -> io::Result<OwnedHandle> {
        // SAFETY: null attributes and name are documented as valid; the
        // returned handle is owned by us.
        unsafe { cvt_created_handle(CreateJobObjectW(std::ptr::null(), std::ptr::null())) }
    }

    /// Assign the process behind `process` to the job behind `job`.
    pub fn assign_process_to_job(job: BorrowedHandle, process: BorrowedHandle) -> io::Result<()> {
        // SAFETY: both handles are valid for the duration of the call by
        // construction of `BorrowedHandle`.
        cvt(unsafe {
            AssignProcessToJobObject(job.as_raw_handle() as HANDLE, process.as_raw_handle())
        })
    }

    /// Terminate every process in the job with the given exit code.
    pub fn terminate_job_object(job: BorrowedHandle, exit_code: u32) -> io::Result<()> {
        // SAFETY: the job handle is valid for the duration of the call.
        cvt(unsafe { TerminateJobObject(job.as_raw_handle() as HANDLE, exit_code) })
    }

    /// Terminate the process behind `process` with the given exit code.
    pub fn terminate_process(process: BorrowedHandle, exit_code: u32) -> io::Result<()> {
        // SAFETY: the process handle is valid for the duration of the call;
        // terminating an already-exited process fails cleanly.
        cvt(unsafe { TerminateProcess(process.as_raw_handle() as HANDLE, exit_code) })
    }

    /// Create an unnamed manual-reset event, initially unsignaled.
    pub fn create_manual_reset_event() -> io::Result<OwnedHandle> {
        // SAFETY: null attributes and name are documented as valid; the
        // returned handle is owned by us.
        unsafe {
            cvt_created_handle(CreateEventW(
                std::ptr::null(),
                TRUE,
                FALSE,
                std::ptr::null(),
            ))
        }
    }

    /// Signal a (manual-reset or auto-reset) event.
    pub fn set_event(event: BorrowedHandle) -> io::Result<()> {
        // SAFETY: the event handle is valid for the duration of the call.
        cvt(unsafe { SetEvent(event.as_raw_handle() as HANDLE) })
    }

    /// Return a manual-reset event to the unsignaled state.
    pub fn reset_event(event: BorrowedHandle) -> io::Result<()> {
        // SAFETY: the event handle is valid for the duration of the call.
        cvt(unsafe { ResetEvent(event.as_raw_handle() as HANDLE) })
    }

    /// Create a one-shot waitable timer with the best resolution the OS
    /// offers: a high-resolution timer (not coalesced to the ~15.6 ms
    /// scheduler tick; Windows 10 1803+) when supported, a standard
    /// waitable timer otherwise.
    pub fn create_waitable_timer() -> io::Result<OwnedHandle> {
        create_waitable_timer_with(CREATE_WAITABLE_TIMER_HIGH_RESOLUTION)
            // Pre-1803 systems reject the high-resolution flag.
            .or_else(|_| create_waitable_timer_with(0))
    }

    fn create_waitable_timer_with(flags: u32) -> io::Result<OwnedHandle> {
        // SAFETY: null attributes and name are documented as valid; the
        // returned handle is owned by us.
        unsafe {
            cvt_created_handle(CreateWaitableTimerExW(
                std::ptr::null(),
                std::ptr::null(),
                flags,
                TIMER_ALL_ACCESS,
            ))
        }
    }

    /// Arm `timer` to fire once after `ticks_100ns` (in 100 ns units).
    pub fn set_relative_timer(timer: BorrowedHandle, ticks_100ns: i64) -> io::Result<()> {
        // A negative due time means a relative wait.
        let due_time = -ticks_100ns;
        // SAFETY: the timer handle is valid, the due-time pointer is valid
        // for the duration of the call, and no completion routine is used.
        cvt(unsafe {
            SetWaitableTimer(
                timer.as_raw_handle() as HANDLE,
                &raw const due_time,
                0,
                None,
                std::ptr::null(),
                FALSE,
            )
        })
    }

    /// Wait until any handle in `handles` is signaled or `timeout_ms`
    /// elapses (`INFINITE` for no limit). On simultaneous completion the
    /// lowest index wins.
    ///
    /// Mutexes are not supported (an abandoned-mutex result is reported as
    /// an error).
    pub fn wait_for_any(handles: &[BorrowedHandle], timeout_ms: u32) -> io::Result<WaitOutcome> {
        let count = u32::try_from(handles.len())
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
        // SAFETY: `BorrowedHandle` is `repr(transparent)` over a raw handle,
        // so the slice is layout-compatible with an array of `HANDLE`, and
        // every element is a valid open handle for the duration of the call.
        let result =
            unsafe { WaitForMultipleObjects(count, handles.as_ptr().cast(), FALSE, timeout_ms) };
        wait_outcome(result, count)
    }

    /// Wait until `handle` is signaled or `timeout_ms` elapses. A zero
    /// timeout makes this a state poll.
    pub fn wait_for_one(handle: BorrowedHandle, timeout_ms: u32) -> io::Result<WaitOutcome> {
        // SAFETY: the handle is valid for the duration of the call.
        let result = unsafe { WaitForSingleObject(handle.as_raw_handle() as HANDLE, timeout_ms) };
        wait_outcome(result, 1)
    }

    fn wait_outcome(result: u32, count: u32) -> io::Result<WaitOutcome> {
        if result == WAIT_TIMEOUT {
            return Ok(WaitOutcome::TimedOut);
        }
        if result == WAIT_FAILED {
            return Err(io::Error::last_os_error());
        }
        let index = result.wrapping_sub(WAIT_OBJECT_0);
        if index < count {
            Ok(WaitOutcome::Object(index))
        } else {
            // WAIT_ABANDONED_0..: only possible for mutexes, which this
            // module never waits on; surface it instead of guessing.
            Err(io::Error::other(format!("unexpected wait result {result}")))
        }
    }

    /// Send a console control event (`CTRL_C_EVENT`/`CTRL_BREAK_EVENT`) to
    /// the console process group led by `process_group_id`.
    pub fn generate_console_ctrl_event(ctrl_event: u32, process_group_id: u32) -> io::Result<()> {
        // SAFETY: no pointers involved; fails cleanly without a console.
        cvt(unsafe { GenerateConsoleCtrlEvent(ctrl_event, process_group_id) })
    }

    /// Register `handler` to receive console control events.
    ///
    /// # Safety
    ///
    /// The OS invokes `handler` on a dedicated thread at any moment,
    /// including while other threads run arbitrary code; it must be sound
    /// under those conditions (touch only atomics and other thread-safe,
    /// non-allocating state).
    pub unsafe fn set_console_ctrl_handler(handler: PHANDLER_ROUTINE) -> io::Result<()> {
        // SAFETY: the handler contract is upheld by the caller.
        cvt(unsafe { SetConsoleCtrlHandler(handler, TRUE) })
    }
}

// POSIX (Linux-layout) signal numbers, matching the Windows `ALL_SIGNALS`
// table in `uucore::signals`. Kept local so the `process` feature does not
// depend on the `signals` feature.
const SIGNAL_HUP: i32 = 1;
const SIGNAL_INT: i32 = 2;
const SIGNAL_QUIT: i32 = 3;

/// What delivering a given POSIX signal number means on Windows.
enum Disposition {
    Probe,
    /// Discarded-by-default and stop signals: accepted, nothing to do.
    Ignore,
    /// `INT`/`QUIT`: deliverable to a console process group as CTRL_BREAK.
    Interrupt,
    Terminate,
}

fn disposition(signal: usize) -> io::Result<Disposition> {
    match signal {
        0 => Ok(Disposition::Probe),
        2 | 3 => Ok(Disposition::Interrupt),
        // Discarded-by-default (CHLD, CONT, URG, WINCH) and the stop family
        // (STOP, TSTP, TTIN, TTOU): no documented emulation, so no-ops.
        17..=23 | 28 => Ok(Disposition::Ignore),
        1..=31 => Ok(Disposition::Terminate),
        _ => Err(io::ErrorKind::InvalidInput.into()),
    }
}

/// Terminate so the child's exit status becomes `128 + signal`, emulating
/// "killed by signal" for exit-code observers.
fn terminate_with_signal(handle: BorrowedHandle, signal: usize) -> io::Result<()> {
    sys::terminate_process(handle, (128 + signal) as u32)
}

/// Deliver `signal` (POSIX numbering) to the child process only.
///
/// A console control event cannot target a single process, so `INT`/`QUIT`
/// fall back to termination here; callers wanting a catchable interrupt must
/// use [`send_signal_to_console_group`].
pub fn send_signal_to_process(child: &Child, signal: usize) -> io::Result<()> {
    match disposition(signal)? {
        Disposition::Probe => match sys::wait_for_one(child.as_handle(), 0)? {
            sys::WaitOutcome::TimedOut => Ok(()),
            // The process has exited: the POSIX analog is ESRCH.
            sys::WaitOutcome::Object(_) => Err(io::ErrorKind::NotFound.into()),
        },
        Disposition::Ignore => Ok(()),
        Disposition::Interrupt | Disposition::Terminate => {
            terminate_with_signal(child.as_handle(), signal)
        }
    }
}

/// Deliver `signal` (POSIX numbering) to the console process group led by
/// `pid` (the process must have been created with `CREATE_NEW_PROCESS_GROUP`,
/// e.g. via [`configure_process_group`]).
///
/// Only `INT`/`QUIT` are deliverable this way (as `CTRL_BREAK_EVENT`);
/// terminating a whole group requires a [`Job`].
pub fn send_signal_to_console_group(pid: u32, signal: usize) -> io::Result<()> {
    match disposition(signal)? {
        Disposition::Probe | Disposition::Ignore => Ok(()),
        // CTRL_C_EVENT cannot target a nonzero group (it broadcasts to the
        // whole console, including the sender), so INT/QUIT both use
        // CTRL_BREAK: targetable, catchable, fatal by default.
        Disposition::Interrupt => sys::generate_console_ctrl_event(CTRL_BREAK_EVENT, pid),
        Disposition::Terminate => Err(io::ErrorKind::Unsupported.into()),
    }
}

/// Deliver `signal` (POSIX numbering) to the child's whole process tree.
///
/// Terminating signals (including `INT`/`QUIT`, which cannot reach a tree as
/// console events) terminate the job with exit code `128 + n`, falling back
/// to the direct child when `job` is `None` or the job terminate fails;
/// probe/ignored signals behave as [`send_signal_to_process`].
pub fn send_signal_to_tree(child: &Child, job: Option<&Job>, signal: usize) -> io::Result<()> {
    match disposition(signal)? {
        Disposition::Probe | Disposition::Ignore => send_signal_to_process(child, signal),
        Disposition::Interrupt | Disposition::Terminate => {
            if let Some(job) = job {
                if job.terminate((128 + signal) as u32).is_ok() {
                    return Ok(());
                }
            }
            terminate_with_signal(child.as_handle(), signal)
        }
    }
}

/// Make `cmd` spawn its child as the leader of a new console process group,
/// so `CTRL_BREAK_EVENT` can target exactly that group and the console's own
/// Ctrl-C no longer reaches the child (the analog of unix `setpgid(0, 0)`).
///
/// Overwrites any creation flags set earlier via `CommandExt::creation_flags`.
pub fn configure_process_group(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
}

/// An anonymous Job Object: the Windows primitive for operating on a whole
/// process tree, used here as the analog of signalling a process group.
///
/// No limits (in particular no `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) are set:
/// the job is only a handle for [`Job::terminate`], and processes that
/// outlive the job's owner deliberately keep running, just like a process
/// group outlives its creator on unix.
pub struct Job(OwnedHandle);

impl Job {
    /// Create a new anonymous job object.
    pub fn new() -> io::Result<Self> {
        sys::create_job_object().map(Self)
    }

    /// Assign `child` (and, transitively, every process it spawns from then
    /// on) to this job.
    ///
    /// This can fail when nested jobs are unsupported (pre-Windows 8) and the
    /// current process already runs inside a job; callers should degrade to
    /// per-process operations in that case.
    pub fn assign(&self, child: &Child) -> io::Result<()> {
        sys::assign_process_to_job(self.0.as_handle(), child.as_handle())
    }

    /// Terminate every process in the job with the given exit code
    /// (pass `128 + signal` to emulate death by signal).
    pub fn terminate(&self, exit_code: u32) -> io::Result<()> {
        sys::terminate_job_object(self.0.as_handle(), exit_code)
    }
}

/// Manual-reset event signalled by the console control handler to wake
/// [`ChildExt::wait_or_timeout`]. Unset until [`enable_ctrl_forwarding`]
/// runs; the handle intentionally lives for the rest of the process.
///
/// Ownership of the *signaled state*: `wait_or_timeout` resets the event when
/// it consumes a wake, and [`enable_ctrl_forwarding`] resets it so each
/// forwarding session starts clean; the handler only ever sets it.
static WAKE_EVENT: OnceLock<OwnedHandle> = OnceLock::new();
/// POSIX signal number of the last console control event received (0 = none).
/// Consumed (reset to 0) by [`take_last_ctrl_signal`] and cleared by
/// [`enable_ctrl_forwarding`], so one event is observed at most once.
static LAST_CTRL_SIGNAL: AtomicI32 = AtomicI32::new(0);

/// Console control handler. Runs on an OS-spawned thread, so it only touches
/// atomics and signals the pre-created event — no allocation, no locks.
///
/// # Safety
///
/// Registered only via [`sys::set_console_ctrl_handler`] and invoked by the
/// system with a valid `ctrl_type`.
unsafe extern "system" fn console_ctrl_handler(ctrl_type: u32) -> BOOL {
    let signal = match ctrl_type {
        CTRL_C_EVENT => SIGNAL_INT,
        CTRL_BREAK_EVENT => SIGNAL_QUIT,
        // Console window closing (analog of losing the controlling terminal);
        // the system force-terminates us after a grace period, so react now.
        CTRL_CLOSE_EVENT => SIGNAL_HUP,
        // Logoff/shutdown reach only services; leave them to the default handler.
        _ => return 0,
    };
    LAST_CTRL_SIGNAL.store(signal, Ordering::Release);
    if let Some(event) = WAKE_EVENT.get() {
        let _ = sys::set_event(event.as_handle());
    }
    1
}

/// Install a console control handler that records Ctrl-C, Ctrl-Break and
/// console-close events as POSIX signal numbers (INT, QUIT, HUP) instead of
/// letting them terminate this process, and wakes any pending
/// [`ChildExt::wait_or_timeout`] call that was given a `signaled` flag.
///
/// Use [`take_last_ctrl_signal`] to consume the last event received.
///
/// Idempotent; every call also discards any event latched by a previous
/// forwarding session, so repeated in-process runs (benchmarks, fuzzing)
/// start clean. Events arriving after this call still latch until consumed.
pub fn enable_ctrl_forwarding() -> io::Result<()> {
    if WAKE_EVENT.get().is_none() {
        // Manual-reset so a wakeup latched before a wait starts is never lost.
        let event = sys::create_manual_reset_event()?;
        // A racing second caller just drops its event; registering the handler
        // twice is harmless.
        let _ = WAKE_EVENT.set(event);
        // SAFETY: the handler only touches atomics, the immutable `WAKE_EVENT`
        // cell and a live event handle — sound for arbitrary-thread invocation.
        unsafe { sys::set_console_ctrl_handler(Some(console_ctrl_handler))? };
    }
    LAST_CTRL_SIGNAL.store(0, Ordering::Release);
    if let Some(event) = WAKE_EVENT.get() {
        let _ = sys::reset_event(event.as_handle());
    }
    Ok(())
}

/// Consume the POSIX signal number of the last console control event received
/// since [`enable_ctrl_forwarding`], if any, resetting the latch so a stale
/// event can never influence a later decision.
pub fn take_last_ctrl_signal() -> Option<usize> {
    match LAST_CTRL_SIGNAL.swap(0, Ordering::AcqRel) {
        0 => None,
        signal => Some(signal as usize),
    }
}

/// Create a one-shot waitable timer that fires after `timeout`.
fn start_relative_timer(timeout: Duration) -> io::Result<OwnedHandle> {
    let timer = sys::create_waitable_timer()?;
    // 100 ns ticks: round up so a sub-tick duration never fires early, and
    // clamp huge durations.
    let ticks = timeout.as_nanos().div_ceil(100).min(i64::MAX as u128) as i64;
    sys::set_relative_timer(timer.as_handle(), ticks)?;
    Ok(timer)
}

impl ChildExt for Child {
    fn send_signal(&mut self, signal: usize) -> io::Result<()> {
        send_signal_to_process(self, signal)
    }

    fn send_signal_group(&mut self, signal: usize) -> io::Result<()> {
        // Unlike unix (which signals the caller's own process group), this
        // targets the child's console group — the only group Windows lets us
        // address at all.
        let pid = self.id();
        send_signal_to_console_group(pid, signal)
    }

    fn wait_or_timeout(
        &mut self,
        timeout: Duration,
        signaled: Option<&AtomicBool>,
    ) -> io::Result<Option<ExitStatus>> {
        // The unix implementation drops stdin so the child sees EOF; match it.
        drop(self.stdin.take());

        // Zero disables the timeout: no timer, so the wait blocks until the
        // child exits (or a console event fires).
        let timer = if timeout.is_zero() {
            None
        } else {
            Some(start_relative_timer(timeout)?)
        };
        // The wake event is left out when this wait must ignore console events
        // (e.g. the kill-after grace period).
        let wake_event = if signaled.is_some() {
            WAKE_EVENT.get()
        } else {
            None
        };

        // A single blocking wait over up to three handles. On simultaneous
        // completion the lowest index wins, so child-exit beats a console
        // event, which beats timer expiry — matching the unix races.
        let mut handles: Vec<BorrowedHandle> = Vec::with_capacity(3);
        handles.push(self.as_handle());
        let wake_index = wake_event.map(|event| {
            handles.push(event.as_handle());
            handles.len() - 1
        });
        if let Some(timer) = &timer {
            handles.push(timer.as_handle());
        }

        let index = match sys::wait_for_any(&handles, INFINITE)? {
            sys::WaitOutcome::Object(index) => index as usize,
            // Unreachable with an INFINITE wait; treat as timer expiry.
            sys::WaitOutcome::TimedOut => return Ok(None),
        };
        drop(handles);

        if index == 0 {
            // Child exited; reap it without blocking.
            return self.wait().map(Some);
        }
        if Some(index) == wake_index {
            // Consume the wake so a handled console event cannot satisfy a
            // later wait; the latched signal number stays for the caller to
            // take via `take_last_ctrl_signal`.
            if let Some(event) = wake_event {
                let _ = sys::reset_event(event.as_handle());
            }
            if let Some(flag) = signaled {
                flag.store(true, Ordering::Relaxed);
            }
        }
        // Console event or timer expiry: the child is still running.
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};
    use std::time::Instant;

    /// Poll whether the wake event is currently signaled.
    fn wake_event_signaled() -> bool {
        let event = WAKE_EVENT.get().expect("forwarding enabled");
        matches!(
            sys::wait_for_one(event.as_handle(), 0).unwrap(),
            sys::WaitOutcome::Object(0)
        )
    }

    /// `LAST_CTRL_SIGNAL` and `WAKE_EVENT` are process-global, so every phase
    /// runs inside this single test to keep them free of cross-test races
    /// (cargo runs tests on parallel threads).
    #[test]
    fn ctrl_forwarding_latch_and_wake_lifecycle() {
        // Phase 1: enabling starts clean.
        enable_ctrl_forwarding().unwrap();
        assert_eq!(take_last_ctrl_signal(), None);
        assert!(!wake_event_signaled());

        // Phase 2: a console event latches its signal number exactly once and
        // signals the wake event; consuming resets the latch.
        // SAFETY: the handler only touches atomics and the event handle.
        assert_eq!(unsafe { console_ctrl_handler(CTRL_C_EVENT) }, 1);
        assert!(wake_event_signaled());
        assert_eq!(take_last_ctrl_signal(), Some(SIGNAL_INT as usize));
        assert_eq!(take_last_ctrl_signal(), None);

        // Phase 3: unhandled control types latch nothing (5 = logoff).
        // SAFETY: as above.
        assert_eq!(unsafe { console_ctrl_handler(5) }, 0);
        assert_eq!(take_last_ctrl_signal(), None);

        // Phase 4: re-enabling discards state latched by a previous session.
        // SAFETY: as above.
        assert_eq!(unsafe { console_ctrl_handler(CTRL_BREAK_EVENT) }, 1);
        enable_ctrl_forwarding().unwrap();
        assert_eq!(take_last_ctrl_signal(), None);
        assert!(!wake_event_signaled());

        // Phase 5: `wait_or_timeout` consumes a wake: it returns promptly
        // without the child having exited, sets the caller's flag, and leaves
        // the event unsignaled so it cannot satisfy a later wait.
        let mut child = Command::new("ping")
            .args(["-n", "10", "127.0.0.1"])
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        sys::set_event(WAKE_EVENT.get().unwrap().as_handle()).unwrap();
        let flag = AtomicBool::new(false);
        let started = Instant::now();
        let result = child
            .wait_or_timeout(Duration::from_secs(60), Some(&flag))
            .unwrap();
        assert_eq!(result, None);
        assert!(flag.load(Ordering::Relaxed));
        assert!(started.elapsed() < Duration::from_secs(30));
        assert!(!wake_event_signaled());
        child.kill().unwrap();
        child.wait().unwrap();
    }
}
