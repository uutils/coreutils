/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alexander Batischev <eual.jp@gmail.com>
 * (c) Thomas Queiroz <thomasqueirozb@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[cfg(unix)]
pub use self::unix::{stdin_is_pipe_or_fifo, supports_pid_checks, Pid, ProcessChecker};

#[cfg(windows)]
pub use self::windows::{supports_pid_checks, Pid, ProcessChecker};

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;
