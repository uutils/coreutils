// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[cfg(windows)]
pub use self::windows::owned_by_current_token;

#[cfg(windows)]
mod windows;
