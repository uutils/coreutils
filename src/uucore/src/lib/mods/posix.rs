// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (vars)
//! Iterate over lines, including the line ending character(s).
//!
//! This module provides the [`posix_version`] function, that returns
//! Some(usize) if the `_POSIX2_VERSION` environment variable is defined
//! and has value that can be parsed.
//! Otherwise returns None, so the calling utility would assume default behavior.
//!
//! NOTE: GNU (as of v9.4) recognizes three distinct values for POSIX version
//!
//! Utilities that rely on this module:
//! `sort` (TBD)
//! `tail` (TBD)
//! `touch` (TBD)
//! `uniq`
//!
use std::env;

/// '199209' for POSIX 1003.2-1992, which would define Obsolete mode
pub const OBSOLETE: usize = 199209;

/// '200112' for POSIX 1003.1-2001, which is the minimum version for Traditional mode
pub const TRADITIONAL: usize = 200112;

/// '200809' for POSIX 1003.1-2008, which is the minimum version for Modern mode
pub const MODERN: usize = 200809;

/// Returns the value of the `_POSIX2_VERSION` environment variable if it is defined
pub fn posix_version() -> Option<usize> {
    env::var("_POSIX2_VERSION")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
}

#[cfg(test)]
mod tests {
    use crate::posix::*;

    #[test]
    fn test_posix_version() {
        // default
        assert_eq!(posix_version(), None);
        // set specific version
        env::set_var("_POSIX2_VERSION", OBSOLETE.to_string());
        assert_eq!(posix_version(), Some(OBSOLETE));
        env::set_var("_POSIX2_VERSION", TRADITIONAL.to_string());
        assert_eq!(posix_version(), Some(TRADITIONAL));
        env::set_var("_POSIX2_VERSION", MODERN.to_string());
        assert_eq!(posix_version(), Some(MODERN));
    }
}
