// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Platform-specific pieces of `df`: the `--sync` flush, the usage probe,
//! over-mount detection, the fallback used when the mount table cannot be
//! read, and the Windows `-i` bail-out. `df.rs` and `filesystem.rs` only talk
//! to the facade functions re-exported here, which both submodules provide
//! with identical signatures.

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(crate) use unix::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::*;
