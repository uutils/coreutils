/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[cfg(unix)]
pub use self::unix::getusername;

#[cfg(windows)]
pub use self::windows::getusername;

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;
