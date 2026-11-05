// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Platform-specific pieces of `timeout`: process-group / job setup, signal
//! sending and signal-forwarding state. The shared control flow in
//! `timeout.rs` only talks to the facade functions re-exported here, which
//! both submodules provide with identical signatures; per-spawn platform
//! state travels through the opaque `SpawnState` returned by `post_spawn`
//! (a job object on Windows, empty on unix).

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(crate) use unix::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::*;
