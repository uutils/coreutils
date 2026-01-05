// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Safe wrappers for environment variable manipulation.
//!
//! This module provides safe abstractions over the inherently unsafe
//! `std::env::set_var` and `std::env::remove_var` functions.
//!
//! # Safety
//!
//! These functions are safe to use when:
//! 1. Called from single-threaded contexts (like utility main functions)
//! 2. Called before spawning child processes
//! 3. No other threads are concurrently accessing environment variables
//!
//! The primary use case is the `env` utility and test code that needs to
//! manipulate environment variables before executing child processes.

use std::env;
use std::ffi::OsStr;

/// Set an environment variable that will be inherited by child processes.
///
/// This is a safe wrapper around `std::env::set_var` that should only be
/// called in single-threaded contexts or before spawning child processes.
///
/// # Examples
///
/// ```no_run
/// use uucore::env_manipulation::set_var;
/// use std::ffi::OsStr;
///
/// set_var(OsStr::new("MY_VAR"), OsStr::new("value"));
/// ```
///
/// # Safety
///
/// This modifies process-wide state. Safe to call in single-threaded
/// contexts or when the caller ensures no other threads are accessing
/// environment variables.
pub fn set_var(key: &OsStr, value: &OsStr) {
    // SAFETY: This is safe when called from single-threaded contexts
    // or before spawning children, which is enforced by the caller
    unsafe {
        env::set_var(key, value);
    }
}

/// Remove an environment variable.
///
/// This is a safe wrapper around `std::env::remove_var` that should only be
/// called in single-threaded contexts or before spawning child processes.
///
/// # Examples
///
/// ```no_run
/// use uucore::env_manipulation::remove_var;
/// use std::ffi::OsStr;
///
/// remove_var(OsStr::new("MY_VAR"));
/// ```
///
/// # Safety
///
/// This modifies process-wide state. Safe to call in single-threaded
/// contexts or when the caller ensures no other threads are accessing
/// environment variables.
pub fn remove_var(key: &OsStr) {
    // SAFETY: This is safe when called from single-threaded contexts
    // or before spawning children, which is enforced by the caller
    unsafe {
        env::remove_var(key);
    }
}

/// Clear all environment variables.
///
/// This is useful when implementing the `env -i` flag behavior.
///
/// # Examples
///
/// ```no_run
/// use uucore::env_manipulation::clear_all;
///
/// clear_all();
/// ```
///
/// # Safety
///
/// This modifies process-wide state by removing all environment variables.
/// Safe to call in single-threaded contexts or when the caller ensures no
/// other threads are accessing environment variables.
pub fn clear_all() {
    // SAFETY: This is safe when called from single-threaded contexts
    // or before spawning children, which is enforced by the caller
    for (name, _) in env::vars_os() {
        unsafe {
            env::remove_var(name);
        }
    }
}

/// A RAII guard that temporarily sets an environment variable and restores
/// the original value when dropped.
///
/// This is useful for test code that needs to temporarily modify environment
/// variables without affecting other tests.
///
/// # Examples
///
/// ```no_run
/// use uucore::env_manipulation::TempEnvVar;
/// use std::ffi::OsStr;
///
/// {
///     let _guard = TempEnvVar::new(OsStr::new("TEST_VAR"), OsStr::new("test_value"));
///     // TEST_VAR is set to "test_value"
/// } // TEST_VAR is restored to its original value (or removed if it didn't exist)
/// ```
#[cfg(test)]
pub struct TempEnvVar {
    key: Box<OsStr>,
    original_value: Option<Box<OsStr>>,
}

#[cfg(test)]
impl TempEnvVar {
    /// Create a new temporary environment variable.
    ///
    /// The variable will be set immediately and restored when the guard is dropped.
    pub fn new(key: &OsStr, value: &OsStr) -> Self {
        let original_value = env::var_os(key).map(|s| s.into_boxed_os_str());
        set_var(key, value);
        Self {
            key: key.to_os_string().into_boxed_os_str(),
            original_value,
        }
    }
}

#[cfg(test)]
impl Drop for TempEnvVar {
    fn drop(&mut self) {
        if let Some(ref value) = self.original_value {
            set_var(&self.key, value);
        } else {
            remove_var(&self.key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn test_set_and_get_var() {
        let key = OsStr::new("UUCORE_TEST_VAR_SET");
        let value = OsStr::new("test_value");

        // Clean up in case it exists
        remove_var(key);

        // Set the variable
        set_var(key, value);

        // Verify it was set
        assert_eq!(env::var_os(key), Some(OsString::from(value)));

        // Clean up
        remove_var(key);
    }

    #[test]
    fn test_remove_var() {
        let key = OsStr::new("UUCORE_TEST_VAR_REMOVE");
        let value = OsStr::new("test_value");

        // Set the variable
        set_var(key, value);
        assert!(env::var_os(key).is_some());

        // Remove it
        remove_var(key);
        assert!(env::var_os(key).is_none());
    }

    #[test]
    fn test_remove_nonexistent_var() {
        let key = OsStr::new("UUCORE_TEST_VAR_NONEXISTENT");

        // Ensure it doesn't exist
        remove_var(key);

        // Removing again should not panic
        remove_var(key);
        assert!(env::var_os(key).is_none());
    }

    #[test]
    fn test_clear_all() {
        // Save current environment
        let original_env: Vec<(OsString, OsString)> = env::vars_os().collect();

        // Set some test variables
        set_var(OsStr::new("UUCORE_TEST_CLEAR_1"), OsStr::new("value1"));
        set_var(OsStr::new("UUCORE_TEST_CLEAR_2"), OsStr::new("value2"));

        // Clear all
        clear_all();

        // Verify environment is empty
        assert_eq!(env::vars_os().count(), 0);

        // Restore original environment
        for (key, value) in original_env {
            set_var(&key, &value);
        }
    }

    #[test]
    fn test_temp_env_var_new() {
        let key = OsStr::new("UUCORE_TEST_TEMP_NEW");
        let value = OsStr::new("temp_value");

        // Ensure it doesn't exist initially
        remove_var(key);
        assert!(env::var_os(key).is_none());

        {
            let _guard = TempEnvVar::new(key, value);
            // Variable should be set
            assert_eq!(env::var_os(key), Some(OsString::from(value)));
        }

        // Variable should be removed after drop
        assert!(env::var_os(key).is_none());
    }

    #[test]
    fn test_temp_env_var_restore() {
        let key = OsStr::new("UUCORE_TEST_TEMP_RESTORE");
        let original_value = OsStr::new("original");
        let temp_value = OsStr::new("temporary");

        // Set original value
        set_var(key, original_value);

        {
            let _guard = TempEnvVar::new(key, temp_value);
            // Variable should be changed
            assert_eq!(env::var_os(key), Some(OsString::from(temp_value)));
        }

        // Variable should be restored
        assert_eq!(env::var_os(key), Some(OsString::from(original_value)));

        // Clean up
        remove_var(key);
    }

    #[test]
    fn test_temp_env_var_nested() {
        let key = OsStr::new("UUCORE_TEST_TEMP_NESTED");
        let value1 = OsStr::new("value1");
        let value2 = OsStr::new("value2");

        remove_var(key);

        {
            let _guard1 = TempEnvVar::new(key, value1);
            assert_eq!(env::var_os(key), Some(OsString::from(value1)));

            {
                let _guard2 = TempEnvVar::new(key, value2);
                assert_eq!(env::var_os(key), Some(OsString::from(value2)));
            }

            // Should restore to value1
            assert_eq!(env::var_os(key), Some(OsString::from(value1)));
        }

        // Should be removed
        assert!(env::var_os(key).is_none());
    }

    #[test]
    fn test_set_var_with_special_chars() {
        let key = OsStr::new("UUCORE_TEST_SPECIAL");
        let value = OsStr::new("value with spaces and 特殊字符");

        set_var(key, value);
        assert_eq!(env::var_os(key), Some(OsString::from(value)));

        remove_var(key);
    }

    #[test]
    fn test_set_var_empty_value() {
        let key = OsStr::new("UUCORE_TEST_EMPTY");
        let value = OsStr::new("");

        set_var(key, value);
        assert_eq!(env::var_os(key), Some(OsString::from(value)));

        remove_var(key);
    }
}
