// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// Platform-specific implementations for the rm utility

#[cfg(all(
    unix,
    not(any(target_os = "aix", target_os = "hurd", target_os = "redox"))
))]
pub mod unix;

#[cfg(all(
    unix,
    not(any(target_os = "aix", target_os = "hurd", target_os = "redox"))
))]
pub use unix::*;
