/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

// spell-checker:ignore (ToDO) getusername

#[cfg(unix)]
pub use self::unix::get_username;

#[cfg(windows)]
pub use self::windows::get_username;

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;
