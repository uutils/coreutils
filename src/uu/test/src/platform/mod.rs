// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[cfg(target_os = "wasi")]
pub use self::wasi::{path, same_file};
#[cfg(windows)]
pub use self::windows::{
    fd_is_terminal, is_executable, is_readable, is_writable, owned_by_current_token, same_file,
};

#[cfg(target_os = "wasi")]
mod wasi;
#[cfg(windows)]
mod windows;
