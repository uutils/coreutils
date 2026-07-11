// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (win-api) WAITABLE Waitable HIRES
// spell-checker:ignore (signals) CHLD TSTP TTIN TTOU WINCH

//! Windows emulation of POSIX signal delivery for child processes.
//!
//! Windows has no signals, so this module emulates the POSIX *default
//! dispositions* using native primitives:
//!
//! - Signal numbers follow the Linux layout (the same table
//!   `uucore::signals::ALL_SIGNALS` uses on Windows), so `-s HUP`, `-s 9`,
//!   etc. keep meaning what cross-platform scripts expect.
//! - "Terminate" signals force-exit the target with exit code `128 + n`,
//!   which is what observers of the exit status see on unix when a process
//!   dies to signal `n`.
//! - `INT`/`QUIT` can be delivered to a console process group as a
//!   `CTRL_BREAK_EVENT`: targetable, catchable by the child, and fatal by
//!   default — the closest analog to a catchable SIGINT. (`CTRL_C_EVENT`
//!   cannot target a specific group; it would be broadcast to the whole
//!   console, including the sender.)
//! - Discard-by-default signals (`CHLD`, `CONT`, `URG`, `WINCH`) and the
//!   stop family (`STOP`, `TSTP`, `TTIN`, `TTOU`), which cannot be emulated
//!   with documented APIs, are accepted as no-ops.
//!
//! [`Job`] provides process-tree termination (the analog of signalling a
//! process group), and [`enable_ctrl_forwarding`] + [`last_ctrl_signal`]
//! translate console control events (Ctrl-C, Ctrl-Break, console close) into
//! POSIX signal numbers so callers can implement signal forwarding.

use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::process::{Child, Command, ExitStatus};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, Ordering};
use std::time::Duration;

use windows_sys::Win32::Foundation::{
    FALSE, HANDLE, TRUE, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::System::Console::{
    CTRL_BREAK_EVENT, CTRL_C_EVENT, CTRL_CLOSE_EVENT, GenerateConsoleCtrlEvent,
    SetConsoleCtrlHandler,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, TerminateJobObject,
};
use windows_sys::Win32::System::Threading::{
    CREATE_NEW_PROCESS_GROUP, CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, CreateEventW,
    CreateWaitableTimerExW, INFINITE, SetEvent, SetWaitableTimer, TIMER_ALL_ACCESS,
    TerminateProcess, WaitForMultipleObjects, WaitForSingleObject,
};
use windows_sys::core::BOOL;

use super::ChildExt;

// POSIX (Linux-layout) signal numbers, matching the Windows `ALL_SIGNALS`
// table in `uucore::signals`. Kept local so the `process` feature does not
// depend on the `signals` feature.
const SIGNAL_HUP: i32 = 1;
const SIGNAL_INT: i32 = 2;
const SIGNAL_QUIT: i32 = 3;

/// What delivering a given POSIX signal number means on Windows.
enum Disposition {
    /// Signal 0: existence check only.
    Probe,
    /// Discarded-by-default and stop signals: accepted, nothing to do.
    Ignore,
    /// `INT`/`QUIT`: deliverable to a console process group as CTRL_BREAK.
    Interrupt,
    /// Everything else: forced termination with exit code `128 + n`.
    Terminate,
}

fn disposition(signal: usize) -> io::Result<Disposition> {
    match signal {
        0 => Ok(Disposition::Probe),
        2 | 3 => Ok(Disposition::Interrupt),
        // CHLD, CONT, URG and WINCH are discarded by default on POSIX; the
        // stop family (STOP, TSTP, TTIN, TTOU) cannot be emulated with
        // documented APIs. All are accepted as no-ops.
        17..=23 | 28 => Ok(Disposition::Ignore),
        1..=31 => Ok(Disposition::Terminate),
        _ => Err(io::ErrorKind::InvalidInput.into()),
    }
}

/// Terminate the process behind `handle` so that its exit status becomes
/// `128 + signal`, emulating "killed by signal" for exit-code observers.
fn terminate_with_signal(handle: HANDLE, signal: usize) -> io::Result<()> {
    // SAFETY: the handle is a valid process handle; terminating an
    // already-exited process fails cleanly with an OS error.
    if unsafe { TerminateProcess(handle, (128 + signal) as u32) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Deliver `signal` (POSIX numbering) to the child process only.
///
/// A console control event cannot target a single process, so `INT`/`QUIT`
/// fall back to their POSIX default disposition here: termination. Callers
/// that want a catchable interrupt must target a process group via
/// [`send_signal_to_console_group`].
pub fn send_signal_to_process(child: &Child, signal: usize) -> io::Result<()> {
    let handle = child.as_raw_handle() as HANDLE;
    match disposition(signal)? {
        Disposition::Probe => {
            // SAFETY: valid process handle; a zero timeout makes this a poll.
            match unsafe { WaitForSingleObject(handle, 0) } {
                WAIT_TIMEOUT => Ok(()),
                // The process has exited: the POSIX analog is ESRCH.
                WAIT_OBJECT_0 => Err(io::ErrorKind::NotFound.into()),
                _ => Err(io::Error::last_os_error()),
            }
        }
        Disposition::Ignore => Ok(()),
        Disposition::Interrupt | Disposition::Terminate => {
            terminate_with_signal(handle, signal)
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
        Disposition::Interrupt => {
            // SAFETY: no pointers involved; fails cleanly without a console.
            if unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pid) } == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }
        Disposition::Terminate => Err(io::ErrorKind::Unsupported.into()),
    }
}

/// Deliver `signal` (POSIX numbering) to the child's whole process tree:
/// terminating signals (including `INT`/`QUIT`, which cannot reach a whole
/// tree as console events) terminate the job with exit code `128 + n`,
/// falling back to the direct child when `job` is `None` or termination via
/// the job fails; probe and ignored signals behave as in
/// [`send_signal_to_process`].
pub fn send_signal_to_tree(child: &Child, job: Option<&Job>, signal: usize) -> io::Result<()> {
    match disposition(signal)? {
        Disposition::Probe | Disposition::Ignore => send_signal_to_process(child, signal),
        Disposition::Interrupt | Disposition::Terminate => {
            if let Some(job) = job {
                if job.terminate((128 + signal) as u32).is_ok() {
                    return Ok(());
                }
            }
            terminate_with_signal(child.as_raw_handle() as HANDLE, signal)
        }
    }
}

/// Make `cmd` spawn its child as the leader of a new console process group,
/// so that `CTRL_BREAK_EVENT` can be targeted at exactly that child's group
/// and the console's own Ctrl-C no longer reaches the child directly (the
/// analog of `setpgid(0, 0)` isolation on unix).
///
/// Note: this sets the command's creation flags, overwriting any flags set
/// earlier via `CommandExt::creation_flags`.
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
        // SAFETY: null attributes and name are documented as valid.
        let raw = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
        if raw.is_null() {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: `raw` is a valid handle exclusively owned by us.
        Ok(Self(unsafe { OwnedHandle::from_raw_handle(raw) }))
    }

    /// Assign `child` (and, transitively, every process it spawns from then
    /// on) to this job.
    ///
    /// This can fail when nested jobs are unsupported (pre-Windows 8) and the
    /// current process already runs inside a job; callers should degrade to
    /// per-process operations in that case.
    pub fn assign(&self, child: &Child) -> io::Result<()> {
        // SAFETY: both handles are valid for the duration of the call.
        if unsafe {
            AssignProcessToJobObject(
                self.0.as_raw_handle() as HANDLE,
                child.as_raw_handle() as HANDLE,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Terminate every process in the job with the given exit code
    /// (pass `128 + signal` to emulate death by signal).
    pub fn terminate(&self, exit_code: u32) -> io::Result<()> {
        // SAFETY: the job handle is valid for the duration of the call.
        if unsafe { TerminateJobObject(self.0.as_raw_handle() as HANDLE, exit_code) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

/// Manual-reset event signalled by the console control handler to wake
/// [`ChildExt::wait_or_timeout`]. Null until [`enable_ctrl_forwarding`] runs.
/// Intentionally lives for the rest of the process; the OS reclaims it.
static WAKE_EVENT: AtomicPtr<core::ffi::c_void> = AtomicPtr::new(ptr::null_mut());
/// POSIX signal number of the last console control event received (0 = none).
static LAST_CTRL_SIGNAL: AtomicI32 = AtomicI32::new(0);

/// The console control handler runs on a system-spawned thread: it must only
/// touch atomics and signal the pre-created event (no allocation, no locks).
///
/// # Safety
///
/// Only registered via `SetConsoleCtrlHandler` and called by the system with
/// a valid `ctrl_type`; touches nothing but atomics and a live event handle.
unsafe extern "system" fn console_ctrl_handler(ctrl_type: u32) -> BOOL {
    let signal = match ctrl_type {
        CTRL_C_EVENT => SIGNAL_INT,
        CTRL_BREAK_EVENT => SIGNAL_QUIT,
        // The console window is going away; the analog of losing the
        // controlling terminal. The system still terminates us after a grace
        // period, so the main thread must react promptly.
        CTRL_CLOSE_EVENT => SIGNAL_HUP,
        // Logoff/shutdown notifications are only delivered to services;
        // leave them to the default handler.
        _ => return FALSE,
    };
    LAST_CTRL_SIGNAL.store(signal, Ordering::Release);
    let event = WAKE_EVENT.load(Ordering::Acquire);
    if !event.is_null() {
        // SAFETY: the event handle is created before the handler is
        // registered and is never closed.
        unsafe { SetEvent(event) };
    }
    TRUE
}

/// Install a console control handler that records Ctrl-C, Ctrl-Break and
/// console-close events as POSIX signal numbers (INT, QUIT, HUP) instead of
/// letting them terminate this process, and wakes any pending
/// [`ChildExt::wait_or_timeout`] call that was given a `signaled` flag.
///
/// Use [`last_ctrl_signal`] to read the last event received. Idempotent.
pub fn enable_ctrl_forwarding() -> io::Result<()> {
    if !WAKE_EVENT.load(Ordering::Acquire).is_null() {
        return Ok(());
    }
    // Manual-reset so a wakeup latched before a wait starts is never lost.
    // SAFETY: null attributes and name are documented as valid.
    let event = unsafe { CreateEventW(ptr::null(), TRUE, FALSE, ptr::null()) };
    if event.is_null() {
        return Err(io::Error::last_os_error());
    }
    WAKE_EVENT.store(event, Ordering::Release);
    // SAFETY: the handler only touches atomics and a live event handle.
    if unsafe { SetConsoleCtrlHandler(Some(console_ctrl_handler), TRUE) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// The POSIX signal number corresponding to the last console control event
/// received since [`enable_ctrl_forwarding`], if any.
pub fn last_ctrl_signal() -> Option<usize> {
    match LAST_CTRL_SIGNAL.load(Ordering::Acquire) {
        0 => None,
        signal => Some(signal as usize),
    }
}

/// Create a one-shot waitable timer that fires after `timeout`.
///
/// Uses a high-resolution timer (100 ns due-time granularity, not coalesced
/// to the ~15.6 ms scheduler tick) when the OS supports it (Windows 10 1803+),
/// falling back to a standard waitable timer otherwise.
fn create_relative_timer(timeout: Duration) -> io::Result<OwnedHandle> {
    // SAFETY: null attributes and name are documented as valid.
    let mut raw = unsafe {
        CreateWaitableTimerExW(
            ptr::null(),
            ptr::null(),
            CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
            TIMER_ALL_ACCESS,
        )
    };
    if raw.is_null() {
        // Pre-1803 systems reject the high-resolution flag.
        // SAFETY: as above.
        raw = unsafe { CreateWaitableTimerExW(ptr::null(), ptr::null(), 0, TIMER_ALL_ACCESS) };
    }
    if raw.is_null() {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `raw` is a valid handle exclusively owned by us.
    let timer = unsafe { OwnedHandle::from_raw_handle(raw) };

    // A negative due time is relative, in 100 ns units. Round up so a
    // sub-tick duration never fires early, and clamp huge durations.
    let ticks = timeout.as_nanos().div_ceil(100).min(i64::MAX as u128) as i64;
    let due_time = -ticks;
    // SAFETY: the timer handle is valid; no completion routine is used.
    if unsafe {
        SetWaitableTimer(
            timer.as_raw_handle() as HANDLE,
            &raw const due_time,
            0,
            None,
            ptr::null(),
            FALSE,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(timer)
}

impl ChildExt for Child {
    fn send_signal(&mut self, signal: usize) -> io::Result<()> {
        send_signal_to_process(self, signal)
    }

    fn send_signal_group(&mut self, signal: usize) -> io::Result<()> {
        // Unlike unix (which signals the caller's own process group), this
        // targets the child's console process group: on Windows only a group
        // the child leads can be addressed at all.
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

        // A single blocking wait over up to three handles. Ordering matters:
        // on simultaneous completion `WaitForMultipleObjects` reports the
        // lowest index, so child-exit wins over a console event, which wins
        // over timer expiry — matching the unix implementation's races.
        let mut handles: [HANDLE; 3] = [ptr::null_mut(); 3];
        let mut count: u32 = 0;

        handles[count as usize] = self.as_raw_handle() as HANDLE;
        count += 1;

        let wake_index = if signaled.is_some() {
            let event = WAKE_EVENT.load(Ordering::Acquire);
            if event.is_null() {
                None
            } else {
                handles[count as usize] = event;
                count += 1;
                Some(count - 1)
            }
        } else {
            // The caller wants this wait to ignore console events (e.g. the
            // kill-after grace period), so the wake event is left out.
            None
        };

        // A timeout of zero disables the timeout: no timer handle, so the
        // wait below blocks until the child exits (or a console event fires).
        let _timer: Option<OwnedHandle> = if timeout.is_zero() {
            None
        } else {
            let timer = create_relative_timer(timeout)?;
            handles[count as usize] = timer.as_raw_handle() as HANDLE;
            count += 1;
            Some(timer)
        };

        // SAFETY: `handles[..count]` are valid, open handles for the whole
        // call; INFINITE is safe because at least the process handle (and
        // usually the timer) will eventually be signalled.
        let result = unsafe { WaitForMultipleObjects(count, handles.as_ptr(), FALSE, INFINITE) };
        let index = result.wrapping_sub(WAIT_OBJECT_0);
        if index >= count {
            // WAIT_FAILED (or an impossible WAIT_ABANDONED: no mutexes here).
            return Err(io::Error::last_os_error());
        }
        if index == 0 {
            // The child exited; this reaps it without blocking.
            return self.wait().map(Some);
        }
        if Some(index) == wake_index {
            if let Some(flag) = signaled {
                flag.store(true, Ordering::Relaxed);
            }
        }
        // Console event or timer expiry: the child is still running.
        Ok(None)
    }
}
