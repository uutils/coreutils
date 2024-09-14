// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore exitstatus cmdline kworker pgrep pwait snice

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

use crate::features::tty::Teletype;
use std::hash::Hash;
use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    fs, io,
    path::PathBuf,
    rc::Rc,
};
use walkdir::{DirEntry, WalkDir};

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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
            inner_stat: fs::read_to_string(dir_append(value, "stat".into()))?,
            ..Default::default()
        })
    }

    pub fn proc_status(&self) -> &str {
        &self.inner_status
    }

    pub fn proc_stat(&self) -> &str {
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
    pub fn stat(&mut self) -> Rc<Vec<String>> {
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
    /// If the process does not belong to any terminal and mismatched permission,
    /// the result will contain [TerminalType::Unknown].
    ///
    /// Otherwise [TerminalType::Unknown] does not appear in the result.
    pub fn tty(&self) -> Teletype {
        let path = PathBuf::from(format!("/proc/{}/fd", self.pid));

        let Ok(result) = fs::read_dir(path) else {
            return Teletype::Unknown;
        };

        for dir in result.flatten().filter(|it| it.path().is_symlink()) {
            if let Ok(path) = fs::read_link(dir.path()) {
                if let Ok(tty) = Teletype::try_from(path) {
                    return tty;
                }
            }
        }

        Teletype::Unknown
    }
}

impl TryFrom<DirEntry> for ProcessInformation {
    type Error = io::Error;

    fn try_from(value: DirEntry) -> Result<Self, Self::Error> {
        let value = value.into_path();

        Self::try_new(value)
    }
}

impl Hash for ProcessInformation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Make it faster.
        self.pid.hash(state);
        self.inner_status.hash(state);
        self.inner_stat.hash(state);
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
    use crate::features::tty::Teletype;
    use std::{collections::HashSet, str::FromStr};

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
            .flat_map(Teletype::try_from)
            .collect::<HashSet<_>>();

        assert_eq!(result.len(), 1);
        assert_eq!(
            pid_entry.tty(),
            Vec::from_iter(result.into_iter()).first().unwrap().clone()
        );
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
