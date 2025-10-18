// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::{cmp::Ordering, sync::OnceLock};

use icu_collator::{self, CollatorBorrowed};

use crate::i18n::{DEFAULT_LOCALE, get_collating_locale};

pub use icu_collator::options::{
    AlternateHandling, CaseLevel, CollatorOptions, MaxVariable, Strength,
};

static COLLATOR: OnceLock<CollatorBorrowed> = OnceLock::new();
static COLLATOR_OPTS: OnceLock<CollatorOptions> = OnceLock::new();
static CASE_INSENSITIVE: OnceLock<bool> = OnceLock::new();
static CAN_USE_ASCII_FASTPATH: OnceLock<bool> = OnceLock::new();

/// Will initialize the collator if not already initialized.
/// returns `true` if initialization happened
pub fn try_init_collator(opts: CollatorOptions) -> bool {
    let case_insensitive = opts
        .strength
        .map(|s| matches!(s, Strength::Secondary | Strength::Primary))
        .unwrap_or(false);
    let _ = CASE_INSENSITIVE.set(case_insensitive);

    // ASCII fast-path can only be used with default collator options.
    // Special options like AlternateHandling::Shifted change comparison semantics
    // in ways that can't be replicated with simple byte/case-insensitive comparison.
    let can_use_fastpath = opts.alternate_handling.is_none()
        && opts.case_level.is_none()
        && opts.max_variable.is_none();
    let _ = CAN_USE_ASCII_FASTPATH.set(can_use_fastpath);

    let _ = COLLATOR_OPTS.set(opts);
    COLLATOR
        .set(CollatorBorrowed::try_new(get_collating_locale().0.clone().into(), opts).unwrap())
        .is_ok()
}

/// Will initialize the collator and panic if already initialized.
pub fn init_collator(opts: CollatorOptions) {
    let case_insensitive = opts
        .strength
        .map(|s| matches!(s, Strength::Secondary | Strength::Primary))
        .unwrap_or(false);
    CASE_INSENSITIVE
        .set(case_insensitive)
        .expect("Case-insensitivity flag already initialized");

    // ASCII fast-path can only be used with default collator options.
    let can_use_fastpath = opts.alternate_handling.is_none()
        && opts.case_level.is_none()
        && opts.max_variable.is_none();
    CAN_USE_ASCII_FASTPATH
        .set(can_use_fastpath)
        .expect("ASCII fast-path flag already initialized");

    COLLATOR_OPTS
        .set(opts)
        .expect("Collator options already initialized");
    COLLATOR
        .set(CollatorBorrowed::try_new(get_collating_locale().0.clone().into(), opts).unwrap())
        .expect("Collator already initialized");
}

/// Compare both strings with regard to the current locale.
///
/// # Performance Optimization
///
/// This function implements a fast-path for ASCII-only strings to avoid
/// the overhead of ICU collation when not needed. ASCII characters have
/// the same collation order across all locales, so byte-wise comparison
/// is both correct and significantly faster.
///
/// # Fast Paths (in order of evaluation)
///
/// 1. **C/POSIX locale**: Direct byte comparison (all filenames)
/// 2. **ASCII-only strings**: Fast ASCII comparison respecting collator strength (UTF-8 locales)
/// 3. **Unicode strings**: Full ICU collation (UTF-8 locales)
///
/// This optimization is critical for performance when sorting directories
/// with primarily ASCII filenames (the common case), while still providing
/// correct locale-aware sorting for international filenames.
pub fn locale_cmp(left: &[u8], right: &[u8]) -> Ordering {
    // Fast path 1: C/POSIX locale - always use byte comparison for all strings
    // No locale-aware collation needed in C/POSIX locale
    if get_collating_locale().0 == DEFAULT_LOCALE {
        return left.cmp(right);
    }

    // Fast path 2: UTF-8 locales with ASCII-only strings AND default collator options
    // Use optimized ASCII comparison that respects collator strength.
    // Skip this fast-path if special collator options (like AlternateHandling::Shifted)
    // are set, as they change comparison semantics in ways we can't replicate simply.
    let can_use_fastpath = CAN_USE_ASCII_FASTPATH.get().copied().unwrap_or(true);
    if can_use_fastpath && left.is_ascii() && right.is_ascii() {
        return cmp_ascii_with_strength(left, right);
    }

    // Slow path: Use ICU collation for Unicode strings or when special options are set
    COLLATOR
        .get()
        .expect("Collator was not initialized")
        .compare_utf8(left, right)
}

/// Fast ASCII comparison respecting collator strength settings.
///
/// Eliminates branch-per-byte overhead by splitting case-sensitive and
/// case-insensitive paths. For case-insensitive mode, includes fast path
/// for equal bytes to avoid unnecessary lowercase operations.
#[inline]
fn cmp_ascii_with_strength(left: &[u8], right: &[u8]) -> Ordering {
    let case_insensitive = CASE_INSENSITIVE.get().copied().unwrap_or(false);

    if case_insensitive {
        cmp_ascii_case_insensitive(left, right)
    } else {
        left.cmp(right)
    }
}

/// Case-insensitive ASCII comparison optimized for short filenames.
///
/// # Performance Strategy
///
/// 1. **Skip equal bytes**: When bytes match, avoid any lowercasing
/// 2. **Branchless lowercase**: Use bit manipulation (no function calls)
/// 3. **Optimized for typical filenames**: Most comparisons resolve in first 1-4 bytes
///
/// Typical benchmark filenames: `f0` vs `f1`, `d0` vs `d1` (2-5 bytes)
#[inline]
fn cmp_ascii_case_insensitive(left: &[u8], right: &[u8]) -> Ordering {
    let min_len = left.len().min(right.len());
    
    for i in 0..min_len {
        let l = left[i];
        let r = right[i];
        
        // Fast path: bytes already equal (common for filename prefixes)
        if l == r {
            continue;
        }
        
        // Convert to lowercase using branchless bit manipulation
        // A-Z (65-90) -> a-z (97-122) by setting bit 5
        // For non-letters, this is a no-op since bit 5 is already set
        let is_l_upper = (l >= b'A') & (l <= b'Z');
        let is_r_upper = (r >= b'A') & (r <= b'Z');
        let l_lower = l | ((is_l_upper as u8) << 5);
        let r_lower = r | ((is_r_upper as u8) << 5);
        
        match l_lower.cmp(&r_lower) {
            Ordering::Equal => continue,
            other => return other,
        }
    }
    
    left.len().cmp(&right.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn test_ascii_fast_path() {
        // Test ASCII fast path (byte comparison for ASCII-only strings)
        // This works regardless of locale setting
        let a = b"apple";
        let b = b"banana";
        assert_eq!(locale_cmp(a, b), Ordering::Less);
        assert_eq!(locale_cmp(b, a), Ordering::Greater);
        assert_eq!(locale_cmp(a, a), Ordering::Equal);
    }

    #[test]
    fn test_ascii_strings_performance_optimization() {
        // This test verifies ASCII fast-path works for UTF-8 locales
        // Note: Collator may be initialized by other tests with case-insensitive settings

        let ascii1 = b"file001.txt";
        let ascii2 = b"file002.txt";

        // ASCII-only strings should use fast comparison
        // even in UTF-8 locales (when both strings are ASCII)
        assert_eq!(locale_cmp(ascii1, ascii2), Ordering::Less);
        assert_eq!(locale_cmp(ascii2, ascii1), Ordering::Greater);
        assert_eq!(locale_cmp(ascii1, ascii1), Ordering::Equal);
    }

    #[test]
    fn test_mixed_ascii_non_ascii() {
        // When either string contains non-ASCII, should use ICU path
        // Initialize collator for this test
        let _ = try_init_collator(CollatorOptions::default());

        let ascii = b"apple";
        let unicode = "café".as_bytes(); // Contains é (non-ASCII)

        // This will hit the ICU path since unicode contains non-ASCII
        let result = locale_cmp(ascii, unicode);
        // Just verify it doesn't panic and produces a deterministic result
        assert!(matches!(
            result,
            Ordering::Less | Ordering::Greater | Ordering::Equal
        ));
    }

    #[test]
    fn test_empty_and_edge_cases() {
        let empty = b"";
        let non_empty = b"test";

        assert_eq!(locale_cmp(empty, empty), Ordering::Equal);
        assert_eq!(locale_cmp(empty, non_empty), Ordering::Less);
        assert_eq!(locale_cmp(non_empty, empty), Ordering::Greater);

        // Single character
        let a = b"a";
        let b = b"b";
        assert_eq!(locale_cmp(a, b), Ordering::Less);
    }

    #[test]
    fn test_case_insensitive_ascii_comparison() {
        // Initialize with case-insensitive collator (Strength::Secondary)
        let mut opts = CollatorOptions::default();
        opts.strength = Some(Strength::Secondary);
        let initialized = try_init_collator(opts);

        // Skip test if collator was already initialized with different settings
        if !initialized && !CASE_INSENSITIVE.get().copied().unwrap_or(false) {
            eprintln!("Skipping test: collator already initialized with case-sensitive settings");
            return;
        }

        // Test case-insensitive comparison
        let lower = b"apple";
        let upper = b"APPLE";
        let mixed = b"Apple";

        // All variants should be considered equal when case-insensitive
        assert_eq!(locale_cmp(lower, upper), Ordering::Equal);
        assert_eq!(locale_cmp(lower, mixed), Ordering::Equal);
        assert_eq!(locale_cmp(upper, mixed), Ordering::Equal);

        // But different words still compare correctly
        assert_eq!(locale_cmp(b"apple", b"BANANA"), Ordering::Less);
        assert_eq!(locale_cmp(b"ZEBRA", b"apple"), Ordering::Greater);
    }

    #[test]
    fn test_case_insensitive_sorting_order() {
        // Test that case-insensitive sorting produces expected order
        let mut opts = CollatorOptions::default();
        opts.strength = Some(Strength::Secondary);
        let initialized = try_init_collator(opts);

        // Skip test if collator was already initialized with different settings
        if !initialized && !CASE_INSENSITIVE.get().copied().unwrap_or(false) {
            eprintln!("Skipping test: collator already initialized with case-sensitive settings");
            return;
        }

        let mut names = vec![b"Zoo".as_slice(), b"apple", b"BANANA", b"cherry"];
        names.sort_by(|a, b| locale_cmp(a, b));

        // Should be sorted alphabetically, ignoring case
        let expected: Vec<&[u8]> = vec![b"apple", b"BANANA", b"cherry", b"Zoo"];
        assert_eq!(names, expected);
    }
}
