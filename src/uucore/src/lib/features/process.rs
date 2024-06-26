// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) cvar exitstatus cmdline kworker
// spell-checker:ignore (sys/unix) WIFSIGNALED
// spell-checker:ignore pgrep pwait snice

//! Set of functions to manage IDs
//!
//! This module provide [`ProcessInformation`] and [`TerminalType`] and corresponding
//! functions for obtaining process information.
//!
//! And also provide [`walk_process`] function to collecting all the information of
//! processes in current system.
//!
//! Utilities that rely on this module:
//! `pgrep` (TBD)
//! `pwait` (TBD)
//! `snice` (TBD)
//!

use libc::{gid_t, pid_t, uid_t};
use std::io;
use std::process::Child;
use std::process::ExitStatus;
use std::thread;
use std::time::{Duration, Instant};
use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Display, Formatter},
    fs,
    path::PathBuf,
    rc::Rc,
};
use walkdir::{DirEntry, WalkDir};

// SAFETY: These functions always succeed and return simple integers.

/// `geteuid()` returns the effective user ID of the calling process.
pub fn geteuid() -> uid_t {
    unsafe { libc::geteuid() }
}

/// `getegid()` returns the effective group ID of the calling process.
pub fn getegid() -> gid_t {
    unsafe { libc::getegid() }
}

/// `getgid()` returns the real group ID of the calling process.
pub fn getgid() -> gid_t {
    unsafe { libc::getgid() }
}

/// `getuid()` returns the real user ID of the calling process.
pub fn getuid() -> uid_t {
    unsafe { libc::getuid() }
}

/// Missing methods for Child objects
pub trait ChildExt {
    /// Send a signal to a Child process.
    ///
    /// Caller beware: if the process already exited then you may accidentally
    /// send the signal to an unrelated process that recycled the PID.
    fn send_signal(&mut self, signal: usize) -> io::Result<()>;

    /// Send a signal to a process group.
    fn send_signal_group(&mut self, signal: usize) -> io::Result<()>;

    /// Wait for a process to finish or return after the specified duration.
    /// A `timeout` of zero disables the timeout.
    fn wait_or_timeout(&mut self, timeout: Duration) -> io::Result<Option<ExitStatus>>;
}

impl ChildExt for Child {
    fn send_signal(&mut self, signal: usize) -> io::Result<()> {
        if unsafe { libc::kill(self.id() as pid_t, signal as i32) } == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    fn send_signal_group(&mut self, signal: usize) -> io::Result<()> {
        // Ignore the signal, so we don't go into a signal loop.
        if unsafe { libc::signal(signal as i32, libc::SIG_IGN) } != 0 {
            return Err(io::Error::last_os_error());
        }
        if unsafe { libc::kill(0, signal as i32) } == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    fn wait_or_timeout(&mut self, timeout: Duration) -> io::Result<Option<ExitStatus>> {
        if timeout == Duration::from_micros(0) {
            return self.wait().map(Some);
        }
        // .try_wait() doesn't drop stdin, so we do it manually
        drop(self.stdin.take());

        let start = Instant::now();
        loop {
            if let Some(status) = self.try_wait()? {
                return Ok(Some(status));
            }

            if start.elapsed() >= timeout {
                break;
            }

            // XXX: this is kinda gross, but it's cleaner than starting a thread just to wait
            //      (which was the previous solution).  We might want to use a different duration
            //      here as well
            thread::sleep(Duration::from_millis(100));
        }

        Ok(None)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TerminalType {
    Tty(u64),
    TtyS(u64),
    Pts(u64),
}

impl TryFrom<String> for TerminalType {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(PathBuf::from(value))
    }
}

impl TryFrom<&str> for TerminalType {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(PathBuf::from(value))
    }
}

impl TryFrom<PathBuf> for TerminalType {
    type Error = ();

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        // Three case: /dev/pts/* , /dev/ttyS**, /dev/tty**

        let mut iter = value.iter();
        // Case 1

        // Considering this format: **/**/pts/<num>
        if let (Some(_), Some(num)) = (iter.find(|it| *it == "pts"), iter.next()) {
            return num
                .to_str()
                .ok_or(())?
                .parse::<u64>()
                .map_err(|_| ())
                .map(TerminalType::Pts);
        };

        // Considering this format: **/**/ttyS** then **/**/tty**
        let path = value.to_str().ok_or(())?;

        let f = |prefix: &str| {
            value
                .iter()
                .last()?
                .to_str()?
                .strip_prefix(prefix)?
                .parse::<u64>()
                .ok()
        };

        if path.contains("ttyS") {
            // Case 2
            f("ttyS").ok_or(()).map(TerminalType::TtyS)
        } else if path.contains("tty") {
            // Case 3
            f("tty").ok_or(()).map(TerminalType::Tty)
        } else {
            Err(())
        }
    }
}

/// State or process
#[derive(Debug, PartialEq, Eq)]
pub enum RunState {
    ///`R`, running
    Running,
    ///`S`, sleeping
    Sleeping,
    ///`D`, sleeping in an uninterruptible wait
    UninterruptibleWait,
    ///`Z`, zombie
    Zombie,
    ///`T`, traced or stopped
    Stopped,
}

impl Display for RunState {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Running => write!(f, "R"),
            Self::Sleeping => write!(f, "S"),
            Self::UninterruptibleWait => write!(f, "D"),
            Self::Zombie => write!(f, "Z"),
            Self::Stopped => write!(f, "T"),
        }
    }
}

impl TryFrom<char> for RunState {
    type Error = io::Error;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'R' => Ok(Self::Running),
            'S' => Ok(Self::Sleeping),
            'D' => Ok(Self::UninterruptibleWait),
            'Z' => Ok(Self::Zombie),
            'T' => Ok(Self::Stopped),
            _ => Err(io::ErrorKind::InvalidInput.into()),
        }
    }
}

impl TryFrom<&str> for RunState {
    type Error = io::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() != 1 {
            return Err(io::ErrorKind::InvalidInput.into());
        }

        Self::try_from(
            value
                .chars()
                .nth(0)
                .ok_or::<io::Error>(io::ErrorKind::InvalidInput.into())?,
        )
    }
}

impl TryFrom<String> for RunState {
    type Error = io::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl TryFrom<&String> for RunState {
    type Error = io::Error;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

/// Process ID and its information
#[derive(Debug, Clone, Default)]
pub struct ProcessInformation {
    pub pid: usize,
    pub cmdline: String,

    inner_status: String,
    inner_stat: String,

    /// Processed `/proc/self/status` file
    cached_status: Option<Rc<HashMap<String, String>>>,
    /// Processed `/proc/self/stat` file
    cached_stat: Option<Rc<Vec<String>>>,

    cached_start_time: Option<u64>,
    cached_tty: Option<Rc<HashSet<TerminalType>>>,
}

impl ProcessInformation {
    /// Try new with pid path such as `/proc/self`
    ///
    /// # Error
    ///
    /// If the files in path cannot be parsed into [ProcessInformation],
    /// it almost caused by wrong filesystem structure.
    ///
    /// - [The /proc Filesystem](https://docs.kernel.org/filesystems/proc.html#process-specific-subdirectories)
    pub fn try_new(value: PathBuf) -> Result<Self, io::Error> {
        let dir_append = |mut path: PathBuf, str: String| {
            path.push(str);
            path
        };

        let value = if value.is_symlink() {
            fs::read_link(value)?
        } else {
            value
        };

        let pid = {
            value
                .iter()
                .last()
                .ok_or(io::ErrorKind::Other)?
                .to_str()
                .ok_or(io::ErrorKind::InvalidData)?
                .parse::<usize>()
                .map_err(|_| io::ErrorKind::InvalidData)?
        };
        let cmdline = fs::read_to_string(dir_append(value.clone(), "cmdline".into()))?
            .replace('\0', " ")
            .trim_end()
            .into();

        Ok(Self {
            pid,
            cmdline,
            inner_status: fs::read_to_string(dir_append(value.clone(), "status".into()))?,
            inner_stat: fs::read_to_string(dir_append(value.clone(), "stat".into()))?,
            ..Default::default()
        })
    }

    pub fn inner_status(&self) -> &str {
        &self.inner_status
    }

    pub fn inner_stat(&self) -> &str {
        &self.inner_stat
    }

    /// Collect information from `/proc/<pid>/status` file
    pub fn status(&mut self) -> Rc<HashMap<String, String>> {
        if let Some(c) = &self.cached_status {
            return Rc::clone(c);
        }

        let result = self
            .inner_status
            .lines()
            .filter_map(|it| it.split_once(':'))
            .map(|it| (it.0.to_string(), it.1.trim_start().to_string()))
            .collect::<HashMap<_, _>>();

        let result = Rc::new(result);
        self.cached_status = Some(Rc::clone(&result));
        Rc::clone(&result)
    }

    /// Collect information from `/proc/<pid>/stat` file
    fn stat(&mut self) -> Rc<Vec<String>> {
        if let Some(c) = &self.cached_stat {
            return Rc::clone(c);
        }

        let result: Vec<_> = stat_split(&self.inner_stat);

        let result = Rc::new(result);
        self.cached_stat = Some(Rc::clone(&result));
        Rc::clone(&result)
    }

    /// Fetch start time from [ProcessInformation::cached_stat]
    ///
    /// - [The /proc Filesystem: Table 1-4](https://docs.kernel.org/filesystems/proc.html#id10)
    pub fn start_time(&mut self) -> Result<u64, io::Error> {
        if let Some(time) = self.cached_start_time {
            return Ok(time);
        }

        // Kernel doc: https://docs.kernel.org/filesystems/proc.html#process-specific-subdirectories
        // Table 1-4
        let time = self
            .stat()
            .get(21)
            .ok_or(io::ErrorKind::InvalidData)?
            .parse::<u64>()
            .map_err(|_| io::ErrorKind::InvalidData)?;

        self.cached_start_time = Some(time);

        Ok(time)
    }

    /// Fetch run state from [ProcessInformation::cached_stat]
    ///
    /// - [The /proc Filesystem: Table 1-4](https://docs.kernel.org/filesystems/proc.html#id10)
    ///
    /// # Error
    ///
    /// If parsing failed, this function will return [io::ErrorKind::InvalidInput]
    pub fn run_state(&mut self) -> Result<RunState, io::Error> {
        RunState::try_from(self.stat().get(2).unwrap().as_str())
    }

    /// This function will scan the `/proc/<pid>/fd` directory
    ///
    /// # Error
    ///
    /// If scanned pid had mismatched permission,
    /// it will caused [std::io::ErrorKind::PermissionDenied] error.
    pub fn ttys(&mut self) -> Result<Rc<HashSet<TerminalType>>, io::Error> {
        if let Some(tty) = &self.cached_tty {
            return Ok(Rc::clone(tty));
        }

        let path = PathBuf::from(format!("/proc/{}/fd", self.pid));

        let result = Rc::new(
            fs::read_dir(path)?
                .flatten()
                .filter(|it| it.path().is_symlink())
                .flat_map(|it| fs::read_link(it.path()))
                .flat_map(TerminalType::try_from)
                .collect::<HashSet<_>>(),
        );

        self.cached_tty = Some(Rc::clone(&result));

        Ok(result)
    }
}

impl TryFrom<DirEntry> for ProcessInformation {
    type Error = io::Error;

    fn try_from(value: DirEntry) -> Result<Self, Self::Error> {
        let value = value.into_path();

        Self::try_new(value)
    }
}

/// Parsing `/proc/self/stat` file.
///
/// In some case, the first pair (and the only one pair) will contains whitespace,
/// so if we want to parse it, we have to write new algorithm.
///
/// TODO: If possible, test and use regex to replace this algorithm.
fn stat_split(stat: &str) -> Vec<String> {
    let stat = String::from(stat);

    let mut buf = String::with_capacity(stat.len());

    let l = stat.find('(');
    let r = stat.find(')');
    let content = if let (Some(l), Some(r)) = (l, r) {
        let replaced = stat[(l + 1)..r].replace(' ', "$$");

        buf.push_str(&stat[..l]);
        buf.push_str(&replaced);
        buf.push_str(&stat[(r + 1)..stat.len()]);

        &buf
    } else {
        &stat
    };

    content
        .split_whitespace()
        .map(|it| it.replace("$$", " "))
        .collect()
}

/// Iterating pid in current system
pub fn walk_process() -> impl Iterator<Item = ProcessInformation> {
    WalkDir::new("/proc/")
        .max_depth(1)
        .follow_links(false)
        .into_iter()
        .flatten()
        .filter(|it| it.path().is_dir())
        .flat_map(ProcessInformation::try_from)
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_tty_convention() {
        assert_eq!(
            TerminalType::try_from("/dev/tty1").unwrap(),
            TerminalType::Tty(1)
        );
        assert_eq!(
            TerminalType::try_from("/dev/tty10").unwrap(),
            TerminalType::Tty(10)
        );
        assert_eq!(
            TerminalType::try_from("/dev/pts/1").unwrap(),
            TerminalType::Pts(1)
        );
        assert_eq!(
            TerminalType::try_from("/dev/pts/10").unwrap(),
            TerminalType::Pts(10)
        );
        assert_eq!(
            TerminalType::try_from("/dev/ttyS1").unwrap(),
            TerminalType::TtyS(1)
        );
        assert_eq!(
            TerminalType::try_from("/dev/ttyS10").unwrap(),
            TerminalType::TtyS(10)
        );
        assert_eq!(
            TerminalType::try_from("ttyS10").unwrap(),
            TerminalType::TtyS(10)
        );

        assert!(TerminalType::try_from("value").is_err());
        assert!(TerminalType::try_from("TtyS10").is_err());
    }

    #[test]
    fn test_run_state_conversion() {
        assert_eq!(RunState::try_from("R").unwrap(), RunState::Running);
        assert_eq!(RunState::try_from("S").unwrap(), RunState::Sleeping);
        assert_eq!(
            RunState::try_from("D").unwrap(),
            RunState::UninterruptibleWait
        );
        assert_eq!(RunState::try_from("T").unwrap(), RunState::Stopped);
        assert_eq!(RunState::try_from("Z").unwrap(), RunState::Zombie);

        assert!(RunState::try_from("G").is_err());
        assert!(RunState::try_from("Rg").is_err());
    }

    fn current_pid() -> usize {
        // Direct read link of /proc/self.
        // It's result must be current programs pid.
        fs::read_link("/proc/self")
            .unwrap()
            .to_str()
            .unwrap()
            .parse::<usize>()
            .unwrap()
    }

    #[test]
    fn test_walk_pid() {
        let current_pid = current_pid();

        let find = walk_process().find(|it| it.pid == current_pid);

        assert!(find.is_some());
    }

    #[test]
    fn test_pid_entry() {
        let current_pid = current_pid();

        let mut pid_entry = ProcessInformation::try_new(
            PathBuf::from_str(&format!("/proc/{}", current_pid)).unwrap(),
        )
        .unwrap();

        let result = WalkDir::new(format!("/proc/{}/fd", current_pid))
            .into_iter()
            .flatten()
            .map(DirEntry::into_path)
            .flat_map(|it| it.read_link())
            .flat_map(TerminalType::try_from)
            .collect::<HashSet<_>>();

        assert_eq!(pid_entry.ttys().unwrap(), result.into());
    }

    #[test]
    fn test_stat_split() {
        let case = "32 (idle_inject/3) S 2 0 0 0 -1 69238848 0 0 0 0 0 0 0 0 -51 0 1 0 34 0 0 18446744073709551615 0 0 0 0 0 0 0 2147483647 0 0 0 0 17 3 50 1 0 0 0 0 0 0 0 0 0 0 0";
        assert!(stat_split(case)[1] == "idle_inject/3");

        let case = "3508 (sh) S 3478 3478 3478 0 -1 4194304 67 0 0 0 0 0 0 0 20 0 1 0 11911 2961408 238 18446744073709551615 94340156948480 94340157028757 140736274114368 0 0 0 0 4096 65538 1 0 0 17 8 0 0 0 0 0 94340157054704 94340157059616 94340163108864 140736274122780 140736274122976 140736274122976 140736274124784 0";
        assert!(stat_split(case)[1] == "sh");

        let case = "47246 (kworker /10:1-events) I 2 0 0 0 -1 69238880 0 0 0 0 17 29 0 0 20 0 1 0 1396260 0 0 18446744073709551615 0 0 0 0 0 0 0 2147483647 0 0 0 0 17 10 0 0 0 0 0 0 0 0 0 0 0 0 0";
        assert!(stat_split(case)[1] == "kworker /10:1-events");
    }
}
