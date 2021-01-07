/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alexander Batischev <eual.jp@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[cfg(unix)]
pub use self::unix::{links_count, supports_links_count, supports_pid_checks, Pid, ProcessChecker};

#[cfg(windows)]
pub use self::windows::{links_count, supports_pid_checks, Pid, ProcessChecker};

#[cfg(target_os = "redox")]
pub use self::redox::{links_count, supports_pid_checks, Pid, ProcessChecker};

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;

#[cfg(target_os = "redox")]
mod redox;
