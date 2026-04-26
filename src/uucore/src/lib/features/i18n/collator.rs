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
    use crate::i18n::UEncoding;

    // Check if we need locale-aware collation. Collation is governed by
    // LC_COLLATE, not LC_CTYPE, so read the encoding off the collating locale
    // directly instead of going through get_locale_encoding().
    if get_collating_locale().1 != UEncoding::Utf8 {
        // C/POSIX locale - no collator needed
        return false;
    }

    // UTF-8 locale - initialize collator with Shifted mode to match GNU behavior
    let mut opts = CollatorOptions::default();
    opts.alternate_handling = Some(AlternateHandling::Shifted);

    try_init_collator(opts)
}

/// Compute the ICU collation sort key for the given input bytes and append it to `buf`.
/// This allows pre-computing sort keys once per line, then comparing them with simple
/// byte comparison during sorting (much faster than calling `compare_utf8` per comparison).
pub fn compute_sort_key_utf8(input: &[u8], buf: &mut Vec<u8>) {
    let c = COLLATOR
        .get()
        .expect("compute_sort_key_utf8 called before collator initialization");
    c.write_sort_key_utf8_to(input, buf)
        .expect("ICU write_sort_key_utf8_to failed");
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
