// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Shared block size resolution logic for utilities like ls, du, and df.
//!
//! GNU coreutils utilities that display file sizes follow a common pattern
//! for resolving block sizes from environment variables and defaults.
//! This module centralizes that logic.

use std::sync::LazyLock;

use super::parse_size::parse_size_non_zero_u64;

/// Result of looking up a block size from environment variables.
///
/// Distinguishes between "no env var was set" and "an env var was set but
/// its value was invalid (or zero)". This matters for utilities like `ls`
/// that have different fallback behavior depending on which case occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockSizeEnv {
    /// A valid, non-zero block size was found.
    Found(u64),
    /// An env var was set but its value was invalid or zero.
    /// GNU coreutils treats this as "stop lookup, use default".
    SetButInvalid,
    /// No relevant env var was set at all.
    NotSet,
}

impl BlockSizeEnv {
    /// Returns the block size if valid, `None` otherwise.
    ///
    /// Convenience method for callers (like `du`, `df`) that don't need to
    /// distinguish `SetButInvalid` from `NotSet`.
    pub fn found(self) -> Option<u64> {
        match self {
            Self::Found(n) => Some(n),
            _ => None,
        }
    }
}

/// Look up a block size from the given environment variable names, in order.
///
/// The first *set* variable wins, even if its value is invalid — GNU coreutils
/// ignores lower-priority variables once a higher-priority one is set.
///
/// Typical usage:
/// - `du`/`df`: `block_size_from_env(&["DU_BLOCK_SIZE", "BLOCK_SIZE", "BLOCKSIZE"])`
/// - `ls` (file size): `block_size_from_env(&["LS_BLOCK_SIZE", "BLOCK_SIZE"])`
///   (`BLOCKSIZE` excluded because it only affects allocation display)
/// - `ls` (allocation): `block_size_from_env(&["BLOCKSIZE"])`
pub fn block_size_from_env(vars: &[&str]) -> BlockSizeEnv {
    for var in vars {
        if let Ok(s) = std::env::var(var) {
            return match parse_size_non_zero_u64(&s) {
                Ok(n) => BlockSizeEnv::Found(n),
                Err(_) => BlockSizeEnv::SetButInvalid,
            };
        }
    }
    BlockSizeEnv::NotSet
}

/// Default block size when no env var or flag is set.
///
/// Returns 512 if `POSIXLY_CORRECT` is set, 1024 otherwise.
pub fn default_block_size() -> u64 {
    if *IS_POSIXLY_CORRECT { 512 } else { 1024 }
}

static IS_POSIXLY_CORRECT: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("POSIXLY_CORRECT").is_some());

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Mutex;

    // These tests modify shared environment variables (BLOCK_SIZE, BLOCKSIZE,
    // POSIXLY_CORRECT), so they must run sequentially. The mutex ensures that.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    // SAFETY: set_var/remove_var are unsafe since Rust 2024 edition because env
    // vars are process-global. We serialize access via ENV_LOCK above.
    fn clear_env_vars(vars: &[&str]) {
        for var in vars {
            unsafe {
                std::env::remove_var(var);
            }
        }
    }

    fn set_env_var(key: &str, value: &str) {
        unsafe {
            std::env::set_var(key, value);
        }
    }

    #[test]
    fn test_block_size_from_env_program_var_priority() {
        let _guard = ENV_LOCK.lock().unwrap();
        let vars = ["TEST_DU_BLOCK_SIZE", "BLOCK_SIZE", "BLOCKSIZE"];
        clear_env_vars(&vars);

        set_env_var("TEST_DU_BLOCK_SIZE", "2048");
        set_env_var("BLOCK_SIZE", "4096");
        set_env_var("BLOCKSIZE", "8192");

        assert_eq!(
            block_size_from_env(&["TEST_DU_BLOCK_SIZE", "BLOCK_SIZE", "BLOCKSIZE"]),
            BlockSizeEnv::Found(2048),
            "program-specific var should have highest priority"
        );

        clear_env_vars(&vars);
    }

    #[test]
    fn test_block_size_from_env_block_size_fallback() {
        let _guard = ENV_LOCK.lock().unwrap();
        let vars = ["TEST_PROG_BS", "BLOCK_SIZE", "BLOCKSIZE"];
        clear_env_vars(&vars);

        set_env_var("BLOCK_SIZE", "4096");
        set_env_var("BLOCKSIZE", "8192");

        assert_eq!(
            block_size_from_env(&["TEST_PROG_BS", "BLOCK_SIZE", "BLOCKSIZE"]),
            BlockSizeEnv::Found(4096),
            "BLOCK_SIZE should be checked when program var is unset"
        );

        clear_env_vars(&vars);
    }

    #[test]
    fn test_block_size_from_env_blocksize_fallback() {
        let _guard = ENV_LOCK.lock().unwrap();
        let vars = ["TEST_PROG_BS2", "BLOCK_SIZE", "BLOCKSIZE"];
        clear_env_vars(&vars);

        set_env_var("BLOCKSIZE", "8192");

        assert_eq!(
            block_size_from_env(&["TEST_PROG_BS2", "BLOCK_SIZE", "BLOCKSIZE"]),
            BlockSizeEnv::Found(8192),
            "BLOCKSIZE should be checked when others are unset"
        );

        clear_env_vars(&vars);
    }

    #[test]
    fn test_block_size_from_env_none_set() {
        let _guard = ENV_LOCK.lock().unwrap();
        let vars = ["TEST_PROG_BS3", "BLOCK_SIZE", "BLOCKSIZE"];
        clear_env_vars(&vars);

        assert_eq!(
            block_size_from_env(&["TEST_PROG_BS3", "BLOCK_SIZE", "BLOCKSIZE"]),
            BlockSizeEnv::NotSet,
            "should return NotSet when no env vars are set"
        );
    }

    #[test]
    fn test_block_size_from_env_invalid_stops_lookup() {
        let _guard = ENV_LOCK.lock().unwrap();
        let vars = ["TEST_PROG_BS4", "BLOCK_SIZE", "BLOCKSIZE"];
        clear_env_vars(&vars);

        set_env_var("TEST_PROG_BS4", "invalid");
        set_env_var("BLOCK_SIZE", "4096");

        assert_eq!(
            block_size_from_env(&["TEST_PROG_BS4", "BLOCK_SIZE", "BLOCKSIZE"]),
            BlockSizeEnv::SetButInvalid,
            "first set var with invalid value should return SetButInvalid, not fall through"
        );

        clear_env_vars(&vars);
    }

    #[test]
    fn test_block_size_from_env_zero_stops_lookup() {
        let _guard = ENV_LOCK.lock().unwrap();
        let vars = ["TEST_PROG_BS5", "BLOCK_SIZE", "BLOCKSIZE"];
        clear_env_vars(&vars);

        set_env_var("TEST_PROG_BS5", "0");
        set_env_var("BLOCK_SIZE", "2048");

        assert_eq!(
            block_size_from_env(&["TEST_PROG_BS5", "BLOCK_SIZE", "BLOCKSIZE"]),
            BlockSizeEnv::SetButInvalid,
            "first set var with zero value should return SetButInvalid, not fall through"
        );

        clear_env_vars(&vars);
    }

    #[test]
    fn test_block_size_from_env_all_invalid() {
        let _guard = ENV_LOCK.lock().unwrap();
        let vars = ["TEST_PROG_BS6", "BLOCK_SIZE", "BLOCKSIZE"];
        clear_env_vars(&vars);

        set_env_var("TEST_PROG_BS6", "invalid");
        set_env_var("BLOCK_SIZE", "bad");
        set_env_var("BLOCKSIZE", "nope");

        assert_eq!(
            block_size_from_env(&["TEST_PROG_BS6", "BLOCK_SIZE", "BLOCKSIZE"]),
            BlockSizeEnv::SetButInvalid,
            "should return SetButInvalid when first set var is invalid"
        );

        clear_env_vars(&vars);
    }

    #[test]
    fn test_block_size_from_env_empty_stops_lookup() {
        let _guard = ENV_LOCK.lock().unwrap();
        let vars = ["TEST_PROG_BS8", "BLOCK_SIZE", "BLOCKSIZE"];
        clear_env_vars(&vars);

        // BLOCK_SIZE is set to empty string — first set var wins, even if invalid
        set_env_var("BLOCK_SIZE", "");
        set_env_var("BLOCKSIZE", "512");

        assert_eq!(
            block_size_from_env(&["TEST_PROG_BS8", "BLOCK_SIZE", "BLOCKSIZE"]),
            BlockSizeEnv::SetButInvalid,
            "empty BLOCK_SIZE is set but invalid, should return SetButInvalid (not fall through)"
        );

        clear_env_vars(&vars);
    }

    #[test]
    fn test_block_size_from_env_unset_var_skipped() {
        let _guard = ENV_LOCK.lock().unwrap();
        let vars = ["TEST_PROG_BS9", "BLOCK_SIZE", "BLOCKSIZE"];
        clear_env_vars(&vars);

        // Program var is not set, BLOCK_SIZE is not set, only BLOCKSIZE is set
        set_env_var("BLOCKSIZE", "512");

        assert_eq!(
            block_size_from_env(&["TEST_PROG_BS9", "BLOCK_SIZE", "BLOCKSIZE"]),
            BlockSizeEnv::Found(512),
            "unset vars should be skipped, falling through to BLOCKSIZE"
        );

        clear_env_vars(&vars);
    }

    #[test]
    fn test_block_size_from_env_with_suffix() {
        let _guard = ENV_LOCK.lock().unwrap();
        let vars = ["TEST_PROG_BS7", "BLOCK_SIZE", "BLOCKSIZE"];
        clear_env_vars(&vars);

        set_env_var("TEST_PROG_BS7", "1K");

        assert_eq!(
            block_size_from_env(&["TEST_PROG_BS7", "BLOCK_SIZE", "BLOCKSIZE"]),
            BlockSizeEnv::Found(1024),
            "should parse size suffixes like 1K"
        );

        clear_env_vars(&vars);
    }

    #[test]
    fn test_block_size_from_env_no_blocksize_excludes_blocksize() {
        let _guard = ENV_LOCK.lock().unwrap();
        let vars = ["TEST_PROG_NB", "BLOCK_SIZE", "BLOCKSIZE"];
        clear_env_vars(&vars);

        set_env_var("BLOCKSIZE", "8192");

        assert_eq!(
            block_size_from_env(&["TEST_PROG_NB", "BLOCK_SIZE"]),
            BlockSizeEnv::NotSet,
            "should not check BLOCKSIZE"
        );

        clear_env_vars(&vars);
    }

    #[test]
    fn test_block_size_from_env_no_blocksize_checks_block_size() {
        let _guard = ENV_LOCK.lock().unwrap();
        let vars = ["TEST_PROG_NB2", "BLOCK_SIZE", "BLOCKSIZE"];
        clear_env_vars(&vars);

        set_env_var("BLOCK_SIZE", "4096");
        set_env_var("BLOCKSIZE", "8192");

        assert_eq!(
            block_size_from_env(&["TEST_PROG_NB2", "BLOCK_SIZE"]),
            BlockSizeEnv::Found(4096),
            "should check BLOCK_SIZE but not BLOCKSIZE"
        );

        clear_env_vars(&vars);
    }

    #[test]
    fn test_blocksize_from_env_set() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_env_vars(&["BLOCKSIZE"]);

        set_env_var("BLOCKSIZE", "2048");
        assert_eq!(
            block_size_from_env(&["BLOCKSIZE"]),
            BlockSizeEnv::Found(2048)
        );

        clear_env_vars(&["BLOCKSIZE"]);
    }

    #[test]
    fn test_blocksize_from_env_unset() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_env_vars(&["BLOCKSIZE"]);
        assert_eq!(block_size_from_env(&["BLOCKSIZE"]), BlockSizeEnv::NotSet);
    }

    #[test]
    fn test_blocksize_from_env_invalid() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_env_vars(&["BLOCKSIZE"]);

        set_env_var("BLOCKSIZE", "invalid");
        assert_eq!(
            block_size_from_env(&["BLOCKSIZE"]),
            BlockSizeEnv::SetButInvalid
        );

        clear_env_vars(&["BLOCKSIZE"]);
    }
}
