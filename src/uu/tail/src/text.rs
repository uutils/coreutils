// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) kqueue

pub const DASH: &str = "-";
pub const DEV_STDIN: &str = "/dev/stdin";
pub const STDIN_HEADER: &str = "standard input";
pub const NO_FILES_REMAINING: &str = "no files remaining";
pub const NO_SUCH_FILE: &str = "No such file or directory";
pub const BECOME_INACCESSIBLE: &str = "has become inaccessible";
pub const BAD_FD: &str = "Bad file descriptor";
#[cfg(target_os = "linux")]
pub const BACKEND: &str = "inotify";
#[cfg(all(unix, not(target_os = "linux")))]
pub const BACKEND: &str = "kqueue";
#[cfg(target_os = "windows")]
pub const BACKEND: &str = "ReadDirectoryChanges";
pub const FD0: &str = "/dev/fd/0";
pub const IS_A_DIRECTORY: &str = "Is a directory";
pub const DEV_TTY: &str = "/dev/tty";
pub const DEV_PTMX: &str = "/dev/ptmx";
