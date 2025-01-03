// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "linux", target_os = "android"))
))]
mod other_unix;
#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "linux", target_os = "android"))
))]
pub(crate) use self::other_unix::copy_on_write;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub(crate) use self::macos::copy_on_write;

#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) use self::linux::copy_on_write;

#[cfg(not(any(
    unix,
    any(target_os = "macos", target_os = "linux", target_os = "android")
)))]
mod other;
#[cfg(not(any(
    unix,
    any(target_os = "macos", target_os = "linux", target_os = "android")
)))]
pub(crate) use self::other::copy_on_write;
