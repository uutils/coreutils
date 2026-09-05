// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[macro_use]
pub mod macros;
pub mod random;
#[cfg(not(any(target_os = "aix", target_os = "horizon")))]
pub mod util;
