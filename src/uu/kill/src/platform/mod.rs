// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Platform-specific piece of `kill`: delivering one signal to one pid via
//! the [`send_signal`] facade, provided by both submodules with an identical
//! signature. The shared control flow stays in `kill.rs`.

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(crate) use unix::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::*;
