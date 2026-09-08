// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[cfg(all(unix, not(target_os = "openbsd")))]
mod unix;
#[cfg(all(unix, not(target_os = "openbsd")))]
pub(crate) use unix::*;

#[cfg(windows)]
mod windows;

#[cfg(target_os = "openbsd")]
mod openbsd;
