// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[cfg(unix)]
pub use self::unix::is_unsafe_overwrite;

#[cfg(windows)]
pub use self::windows::is_unsafe_overwrite;

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;
