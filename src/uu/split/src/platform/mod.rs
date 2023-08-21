// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
#[cfg(unix)]
pub use self::unix::instantiate_current_writer;
#[cfg(unix)]
pub use self::unix::paths_refer_to_same_file;

#[cfg(windows)]
pub use self::windows::instantiate_current_writer;
#[cfg(windows)]
pub use self::windows::paths_refer_to_same_file;

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;
