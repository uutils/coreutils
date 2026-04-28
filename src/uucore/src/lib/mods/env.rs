// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Centralized wrappers around [`std::env::set_var`] and
//! [`std::env::remove_var`].
//!
//! Mutating the process environment was made `unsafe` in the Rust 2024
//! edition because it races with concurrent `getenv` calls on POSIX
//! systems. To keep the rest of the codebase free of `unsafe` blocks for
//! env mutations, every call goes through these wrappers — making this
//! module the single place where the `unsafe` lives. Callers must still
//! ensure the modification is sound in their context (e.g. only at
//! startup before threads spawn, or while holding an external mutex that
//! serializes every reader and writer of the variable).

use std::ffi::OsStr;

/// Set the environment variable `key` to `value`.
///
/// Wrapper around [`std::env::set_var`]. See the module documentation for
/// the safety considerations callers must uphold.
pub fn set_var<K: AsRef<OsStr>, V: AsRef<OsStr>>(key: K, value: V) {
    // SAFETY: env mutation races with concurrent getenv on POSIX. Callers
    // are expected to only invoke this at startup or under an external
    // mutex.
    unsafe { std::env::set_var(key, value) }
}

/// Remove the environment variable `key`.
///
/// Wrapper around [`std::env::remove_var`]. See the module documentation
/// for the safety considerations callers must uphold.
pub fn remove_var<K: AsRef<OsStr>>(key: K) {
    // SAFETY: see [`set_var`].
    unsafe { std::env::remove_var(key) }
}
