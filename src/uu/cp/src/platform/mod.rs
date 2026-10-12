// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[cfg(all(
    unix,
    not(any(target_vendor = "apple", target_os = "linux", target_os = "android"))
))]
mod other_unix;
#[cfg(all(
    unix,
    not(any(target_vendor = "apple", target_os = "linux", target_os = "android"))
))]
pub(crate) use self::other_unix::copy_on_write;

#[cfg(target_vendor = "apple")]
mod macos;
#[cfg(target_vendor = "apple")]
pub(crate) use self::macos::copy_on_write;

#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) use self::linux::copy_on_write;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use self::windows::copy_on_write;

#[cfg(not(any(unix, windows)))]
mod other;
#[cfg(not(any(unix, windows)))]
pub(crate) use self::other::copy_on_write;

#[cfg(target_os = "wasi")]
mod wasi;
#[cfg(target_os = "wasi")]
pub(crate) use self::wasi::create_symlink;
