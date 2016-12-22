/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alexander Batischev <eual.jp@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[cfg(unix)]
pub use self::unix::{Pid, supports_pid_checks, ProcessChecker};

#[cfg(windows)]
pub use self::windows::{Pid, supports_pid_checks, ProcessChecker};

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;
