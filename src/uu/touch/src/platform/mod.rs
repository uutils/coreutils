// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[cfg(target_os = "wasi")]
pub use self::wasi::{pathbuf_from_stdout, set_file_times, set_symlink_file_times};

#[cfg(windows)]
pub use self::windows::pathbuf_from_stdout;

#[cfg(target_os = "wasi")]
mod wasi;

#[cfg(windows)]
mod windows;
