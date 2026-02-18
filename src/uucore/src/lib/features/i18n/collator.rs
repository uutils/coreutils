// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::{cell::RefCell, cmp::Ordering, collections::HashMap, sync::OnceLock};

use icu_collator::{self, CollatorBorrowed};

use crate::i18n::{DEFAULT_LOCALE, get_collating_locale};

pub use icu_collator::options::{
    AlternateHandling, CaseLevel, CollatorOptions, MaxVariable, Strength,
};

static COLLATOR: OnceLock<CollatorBorrowed> = OnceLock::new();

// Simple comparison cache for repeated field values
type ComparisonKey = (Vec<u8>, Vec<u8>);
type ComparisonCache = RefCell<HashMap<ComparisonKey, Ordering>>;

thread_local! {
    static COMPARISON_CACHE: ComparisonCache = RefCell::new(HashMap::new());
}

const CACHE_SIZE_LIMIT: usize = 1000;

/// Will initialize the collator if not already initialized.
/// returns `true` if initialization happened
pub fn try_init_collator(opts: CollatorOptions) -> bool {
    COLLATOR
        .set(CollatorBorrowed::try_new(get_collating_locale().0.clone().into(), opts).unwrap())
        .is_ok()
}

/// Will initialize the collator and panic if already initialized.
pub fn init_collator(opts: CollatorOptions) {
    COLLATOR
        .set(CollatorBorrowed::try_new(get_collating_locale().0.clone().into(), opts).unwrap())
        .expect("Collator already initialized");
}

/// Check if locale collation should be used.
pub fn should_use_locale_collation() -> bool {
    get_collating_locale().0 != DEFAULT_LOCALE
}

/// Initialize the collator for locale-aware string comparison if needed.
///
/// This function checks if the current locale requires locale-aware collation
/// (UTF-8 encoding) and initializes the ICU collator with appropriate settings
/// if necessary. For C/POSIX locales, no initialization is needed as byte
/// comparison is sufficient.
///
/// # Returns
///
/// `true` if the collator was initialized for a UTF-8 locale, `false` if
/// using C/POSIX locale (no initialization needed).
///
/// # Example
///
/// ```
/// use uucore::i18n::collator::init_locale_collation;
///
/// if init_locale_collation() {
///     // Using locale-aware collation
/// } else {
///     // Using byte comparison (C/POSIX locale)
/// }
/// ```
pub fn init_locale_collation() -> bool {
    use crate::i18n::{UEncoding, get_locale_encoding};

    // Check if we need locale-aware collation
    if get_locale_encoding() != UEncoding::Utf8 {
        // C/POSIX locale - no collator needed
        return false;
    }

    // UTF-8 locale - initialize collator with Shifted mode to match GNU behavior
    let mut opts = CollatorOptions::default();
    opts.alternate_handling = Some(AlternateHandling::Shifted);

    try_init_collator(opts)
}

/// Compare both strings with regard to the current locale.
pub fn locale_cmp(left: &[u8], right: &[u8]) -> Ordering {
    // If the detected locale is 'C', just do byte-wise comparison
    if get_collating_locale().0 == DEFAULT_LOCALE {
        left.cmp(right)
    } else {
        // Fall back to byte comparison if collator is not available
        COLLATOR
            .get()
            .map_or_else(|| left.cmp(right), |c| c.compare_utf8(left, right))
    }
}

/// Get a reference to the initialized collator for performance-critical paths
#[inline]
pub fn get_collator() -> &'static CollatorBorrowed<'static> {
    COLLATOR.get().expect("Collator was not initialized")
}

/// Hybrid comparison: byte-first with caching and locale fallback
#[inline]
pub fn locale_cmp_unchecked(left: &[u8], right: &[u8]) -> Ordering {
    // Fast path: try byte comparison first
    let byte_cmp = left.cmp(right);

    // If strings are identical by bytes, they're identical by locale too
    if byte_cmp == Ordering::Equal {
        return Ordering::Equal;
    }

    // If both are pure ASCII, byte comparison is sufficient for most locales
    if left.is_ascii() && right.is_ascii() {
        // For ASCII in en_US and similar locales, byte order equals collation order
        // This covers the vast majority of cases
        return byte_cmp;
    }

    // Check cache for repeated comparisons (common in join operations)
    let cache_key = if left.len() + right.len() < 64 {
        // Only cache small strings
        Some((left.to_vec(), right.to_vec()))
    } else {
        None
    };

    if let Some(ref key) = cache_key {
        if let Ok(Some(cached_result)) = COMPARISON_CACHE.try_with(|c| c.borrow().get(key).copied())
        {
            return cached_result;
        }
    }

    // Compute result using ICU for non-ASCII data
    let result = match (std::str::from_utf8(left), std::str::from_utf8(right)) {
        (Ok(l), Ok(r)) => {
            let l_ascii = l.is_ascii();
            let r_ascii = r.is_ascii();

            // If one is ASCII and other isn't, use ICU
            if l_ascii != r_ascii {
                get_collator().compare(l, r)
            } else if !l_ascii {
                // Both non-ASCII, use ICU
                get_collator().compare(l, r)
            } else {
                // Both ASCII - byte comparison should be correct
                byte_cmp
            }
        }
        _ => byte_cmp, // Invalid UTF-8, use byte comparison
    };

    // Cache the result for future lookups
    if let Some(key) = cache_key {
        let _ = COMPARISON_CACHE.try_with(|c| {
            let mut cache = c.borrow_mut();
            if cache.len() >= CACHE_SIZE_LIMIT {
                cache.clear(); // Simple eviction policy
            }
            cache.insert(key, result);
        });
    }

    result
}
