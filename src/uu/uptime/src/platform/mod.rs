// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Platform-specific pieces of `uptime`: the utmp-file operand (which only
//! exists where utmp files do) and the boot-time source for `--since`. The
//! shared control flow in `uptime.rs` only talks to the facade functions
//! re-exported here, which both submodules provide with identical
//! signatures; on Windows the utmp-file facades degenerate to
//! identity/`None`.

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(crate) use unix::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::*;
