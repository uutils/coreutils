//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) kqueue

pub static DASH: &str = "-";
pub static DEV_STDIN: &str = "/dev/stdin";
pub static STDIN_HEADER: &str = "standard input";
pub static NO_FILES_REMAINING: &str = "no files remaining";
pub static NO_SUCH_FILE: &str = "No such file or directory";
pub static BECOME_INACCESSIBLE: &str = "has become inaccessible";
pub static BAD_FD: &str = "Bad file descriptor";
#[cfg(target_os = "linux")]
pub static BACKEND: &str = "inotify";
#[cfg(all(unix, not(target_os = "linux")))]
pub static BACKEND: &str = "kqueue";
#[cfg(target_os = "windows")]
pub static BACKEND: &str = "ReadDirectoryChanges";
