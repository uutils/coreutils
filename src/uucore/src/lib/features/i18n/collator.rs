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

/// Cap on input bytes used to compute a sort key. Callers must fall back to
/// `locale_cmp` when prefix keys tie. 8 KiB bounds key cost on multi-MB lines
/// without hitting the fallback for realistic inputs — see issue #12138
/// (unbounded path was ~40× slower than GNU `sort`).
const SORT_KEY_PREFIX_LIMIT: usize = 8 * 1024;

/// Append the ICU collation sort key for `input` to `buf`, using at most
/// `SORT_KEY_PREFIX_LIMIT` bytes. Returns `true` if the input was truncated;
/// the caller must then fall back to `locale_cmp` on tie.
pub fn compute_sort_key_utf8(input: &[u8], buf: &mut Vec<u8>) -> bool {
    let c = COLLATOR
        .get()
        .expect("compute_sort_key_utf8 called before collator initialization");
    let truncated = input.len() > SORT_KEY_PREFIX_LIMIT;
    let effective_input = if truncated {
        let mut end = SORT_KEY_PREFIX_LIMIT;
        while end > 0 && !is_utf8_char_boundary(input[end]) {
            end -= 1;
        }
        &input[..end]
    } else {
        input
    };
    c.write_sort_key_utf8_to(effective_input, buf)
        .expect("ICU write_sort_key_utf8_to failed");
    truncated
}

#[inline]
fn is_utf8_char_boundary(b: u8) -> bool {
    // ASCII (0xxxxxxx) or UTF-8 leading byte (11xxxxxx).
    (b as i8) >= -0x40
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
