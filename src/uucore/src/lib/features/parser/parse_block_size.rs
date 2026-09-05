// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Shared block size resolution logic for utilities like ls, du, and df.
//!
//! GNU coreutils utilities that display file sizes follow a common pattern
//! for resolving block sizes from environment variables and defaults.
//! This module centralizes that logic.

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

/// Returns the GNU display unit of a suffix-only block-size spec, `None` if
/// the spec starts with a digit or is a plain `B`.
///
/// ```
/// # use uucore::parser::parse_block_size::suffix_from_parsed_block_size;
/// assert_eq!(suffix_from_parsed_block_size("k"), Some("K".into()));
/// assert_eq!(suffix_from_parsed_block_size("1K"), None);
/// ```
pub fn suffix_from_parsed_block_size(s: &str) -> Option<String> {
    let mut chars = s.chars();
    let unit = chars.next()?.to_ascii_uppercase();
    if !unit.is_ascii_alphabetic() || unit == 'B' {
        return None;
    }

    Some(match chars.as_str() {
        "" | "D" => unit.to_string(),
        "B" if unit == 'K' => "kB".to_string(),
        suffix => format!("{unit}{suffix}"),
    })
}

/// Like [`block_size_from_env`], but also returns the
/// [`suffix_from_parsed_block_size`] of the variable that won.
pub fn block_size_from_env_with_suffix(vars: &[&str]) -> (BlockSizeEnv, Option<String>) {
    let result = block_size_from_env(vars);
    let suffix = if matches!(result, BlockSizeEnv::Found(_)) {
        vars.iter()
            .find_map(|var| std::env::var(var).ok())
            .and_then(|value| suffix_from_parsed_block_size(&value))
    } else {
        None
    };
    (result, suffix)
}

/// Default block size when no env var or flag is set.
///
/// Returns 512 if `POSIXLY_CORRECT` is set, 1024 otherwise.
pub fn default_block_size() -> u64 {
    if std::env::var("POSIXLY_CORRECT").is_ok() {
        512
    } else {
        1024
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use temp_env::{with_var, with_var_unset, with_vars, with_vars_unset};

    #[test]
    fn test_suffix_from_parsed_block_size_bare_units() {
        // A bare unit is echoed back, canonicalized to the form the unit is
        // conventionally spelled with.
        for (spec, expected) in [
            ("K", "K"),
            ("k", "K"),
            ("KB", "kB"),
            ("kB", "kB"),
            ("KiB", "KiB"),
            ("kiB", "KiB"),
            ("M", "M"),
            ("m", "M"),
            ("MB", "MB"),
            ("MiB", "MiB"),
            ("G", "G"),
            ("T", "T"),
            ("P", "P"),
            ("E", "E"),
        ] {
            assert_eq!(
                suffix_from_parsed_block_size(spec).as_deref(),
                Some(expected),
                "spec {spec:?}"
            );
        }
    }

    #[test]
    fn test_suffix_from_parsed_block_size_decimal_marker() {
        // `KD` selects a decimal multiplier but still displays as `K`.
        assert_eq!(
            suffix_from_parsed_block_size("KD").as_deref(),
            Some("K"),
            "the trailing D is a multiplier marker, not part of the unit"
        );
        assert_eq!(suffix_from_parsed_block_size("MD").as_deref(), Some("M"));
    }

    #[test]
    fn test_suffix_from_parsed_block_size_numeric_specs_have_none() {
        for spec in ["1K", "1024", "2K", "1MB", "0", ""] {
            assert_eq!(
                suffix_from_parsed_block_size(spec),
                None,
                "a spec with a leading number echoes no unit: {spec:?}"
            );
        }
    }

    #[test]
    fn test_suffix_from_parsed_block_size_plain_bytes_have_none() {
        assert_eq!(suffix_from_parsed_block_size("B"), None);
    }

    #[test]
    fn test_block_size_from_env_with_suffix_reports_unit() {
        let vars = ["TEST_PROG_SFX", "BLOCK_SIZE", "BLOCKSIZE"];
        for (value, expected) in [
            ("K", (BlockSizeEnv::Found(1024), Some("K".to_string()))),
            // a numeric spec carries no unit to echo
            ("1K", (BlockSizeEnv::Found(1024), None)),
            ("bogus", (BlockSizeEnv::SetButInvalid, None)),
        ] {
            with_vars(
                [(vars[0], Some(value)), (vars[1], None), (vars[2], None)],
                || {
                    assert_eq!(
                        block_size_from_env_with_suffix(&vars),
                        expected,
                        "value {value:?}"
                    );
                },
            );
        }
    }

    #[test]
    fn test_block_size_from_env_with_suffix_uses_winning_var() {
        let vars = ["TEST_PROG_SFX2", "BLOCK_SIZE", "BLOCKSIZE"];
        // The first *set* variable wins, and the unit comes from that one.
        with_vars(
            [(vars[0], None), (vars[1], Some("M")), (vars[2], Some("K"))],
            || {
                assert_eq!(
                    block_size_from_env_with_suffix(&vars),
                    (BlockSizeEnv::Found(1024 * 1024), Some("M".to_string()))
                );
            },
        );
    }

    #[test]
    fn test_block_size_from_env_program_var_priority() {
        let vars = ["TEST_DU_BLOCK_SIZE", "BLOCK_SIZE", "BLOCKSIZE"];
        with_vars(
            [
                (vars[0], Some("2048")),
                (vars[1], Some("4096")),
                (vars[2], Some("8192")),
            ],
            || {
                assert_eq!(
                    block_size_from_env(&vars),
                    BlockSizeEnv::Found(2048),
                    "program-specific var should have highest priority"
                );
            },
        );
    }

    #[test]
    fn test_block_size_from_env_block_size_fallback() {
        let vars = ["TEST_PROG_BS", "BLOCK_SIZE", "BLOCKSIZE"];
        with_vars(
            [
                (vars[0], None),
                (vars[1], Some("4096")),
                (vars[2], Some("8192")),
            ],
            || {
                assert_eq!(
                    block_size_from_env(&vars),
                    BlockSizeEnv::Found(4096),
                    "BLOCK_SIZE should be checked when program var is unset"
                );
            },
        );
    }

    #[test]
    fn test_block_size_from_env_blocksize_fallback() {
        let vars = ["TEST_PROG_BS2", "BLOCK_SIZE", "BLOCKSIZE"];
        with_vars(
            [(vars[0], None), (vars[1], None), (vars[2], Some("8192"))],
            || {
                assert_eq!(
                    block_size_from_env(&vars),
                    BlockSizeEnv::Found(8192),
                    "BLOCKSIZE should be checked when others are unset"
                );
            },
        );
    }

    #[test]
    fn test_block_size_from_env_none_set() {
        let vars = ["TEST_PROG_BS3", "BLOCK_SIZE", "BLOCKSIZE"];
        with_vars_unset(vars, || {
            assert_eq!(
                block_size_from_env(&vars),
                BlockSizeEnv::NotSet,
                "should return NotSet when no env vars are set"
            );
        });
    }

    #[test]
    fn test_block_size_from_env_invalid_stops_lookup() {
        let vars = ["TEST_PROG_BS4", "BLOCK_SIZE", "BLOCKSIZE"];
        with_vars(
            [
                (vars[0], Some("invalid")),
                (vars[1], Some("4096")),
                (vars[2], None),
            ],
            || {
                assert_eq!(
                    block_size_from_env(&vars),
                    BlockSizeEnv::SetButInvalid,
                    "first set var with invalid value should return SetButInvalid, not fall through"
                );
            },
        );
    }

    #[test]
    fn test_block_size_from_env_zero_stops_lookup() {
        let vars = ["TEST_PROG_BS5", "BLOCK_SIZE", "BLOCKSIZE"];
        with_vars(
            [
                (vars[0], Some("0")),
                (vars[1], Some("2048")),
                (vars[2], None),
            ],
            || {
                assert_eq!(
                    block_size_from_env(&vars),
                    BlockSizeEnv::SetButInvalid,
                    "first set var with zero value should return SetButInvalid, not fall through"
                );
            },
        );
    }

    #[test]
    fn test_block_size_from_env_all_invalid() {
        let vars = ["TEST_PROG_BS6", "BLOCK_SIZE", "BLOCKSIZE"];
        with_vars(
            [
                (vars[0], Some("invalid")),
                (vars[1], Some("bad")),
                (vars[2], Some("nope")),
            ],
            || {
                assert_eq!(
                    block_size_from_env(&vars),
                    BlockSizeEnv::SetButInvalid,
                    "should return SetButInvalid when first set var is invalid"
                );
            },
        );
    }

    #[test]
    fn test_block_size_from_env_empty_stops_lookup() {
        let vars = ["TEST_PROG_BS8", "BLOCK_SIZE", "BLOCKSIZE"];
        // BLOCK_SIZE is set to empty string — first set var wins, even if invalid
        with_vars(
            [(vars[0], None), (vars[1], Some("")), (vars[2], Some("512"))],
            || {
                assert_eq!(
                    block_size_from_env(&vars),
                    BlockSizeEnv::SetButInvalid,
                    "empty BLOCK_SIZE is set but invalid, should return SetButInvalid (not fall through)"
                );
            },
        );
    }

    #[test]
    fn test_block_size_from_env_unset_var_skipped() {
        let vars = ["TEST_PROG_BS9", "BLOCK_SIZE", "BLOCKSIZE"];
        // Program var is not set, BLOCK_SIZE is not set, only BLOCKSIZE is set
        with_vars(
            [(vars[0], None), (vars[1], None), (vars[2], Some("512"))],
            || {
                assert_eq!(
                    block_size_from_env(&vars),
                    BlockSizeEnv::Found(512),
                    "unset vars should be skipped, falling through to BLOCKSIZE"
                );
            },
        );
    }

    #[test]
    fn test_block_size_from_env_with_suffix() {
        let vars = ["TEST_PROG_BS7", "BLOCK_SIZE", "BLOCKSIZE"];
        with_vars(
            [(vars[0], Some("1K")), (vars[1], None), (vars[2], None)],
            || {
                assert_eq!(
                    block_size_from_env(&vars),
                    BlockSizeEnv::Found(1024),
                    "should parse size suffixes like 1K"
                );
            },
        );
    }

    #[test]
    fn test_block_size_from_env_no_blocksize_excludes_blocksize() {
        with_vars(
            [
                ("TEST_PROG_NB", None),
                ("BLOCK_SIZE", None),
                ("BLOCKSIZE", Some("8192")),
            ],
            || {
                assert_eq!(
                    block_size_from_env(&["TEST_PROG_NB", "BLOCK_SIZE"]),
                    BlockSizeEnv::NotSet,
                    "should not check BLOCKSIZE"
                );
            },
        );
    }

    #[test]
    fn test_block_size_from_env_no_blocksize_checks_block_size() {
        with_vars(
            [
                ("TEST_PROG_NB2", None),
                ("BLOCK_SIZE", Some("4096")),
                ("BLOCKSIZE", Some("8192")),
            ],
            || {
                assert_eq!(
                    block_size_from_env(&["TEST_PROG_NB2", "BLOCK_SIZE"]),
                    BlockSizeEnv::Found(4096),
                    "should check BLOCK_SIZE but not BLOCKSIZE"
                );
            },
        );
    }

    #[test]
    fn test_blocksize_from_env_set() {
        with_var("BLOCKSIZE", Some("2048"), || {
            assert_eq!(
                block_size_from_env(&["BLOCKSIZE"]),
                BlockSizeEnv::Found(2048)
            );
        });
    }

    #[test]
    fn test_blocksize_from_env_unset() {
        with_var_unset("BLOCKSIZE", || {
            assert_eq!(block_size_from_env(&["BLOCKSIZE"]), BlockSizeEnv::NotSet);
        });
    }

    #[test]
    fn test_blocksize_from_env_invalid() {
        with_var("BLOCKSIZE", Some("invalid"), || {
            assert_eq!(
                block_size_from_env(&["BLOCKSIZE"]),
                BlockSizeEnv::SetButInvalid
            );
        });
    }

    #[test]
    fn test_default_block_size_without_posixly_correct() {
        with_var_unset("POSIXLY_CORRECT", || {
            assert_eq!(default_block_size(), 1024);
        });
    }

    #[test]
    fn test_default_block_size_with_posixly_correct() {
        with_var("POSIXLY_CORRECT", Some("1"), || {
            assert_eq!(default_block_size(), 512);
        });
    }

    #[test]
    fn test_default_block_size_with_posixly_correct_empty() {
        // Even an empty value means POSIXLY_CORRECT is set
        with_var("POSIXLY_CORRECT", Some(""), || {
            assert_eq!(default_block_size(), 512);
        });
    }
}
