use std::cmp::Ordering;

use uucore::{
    IntoCharByteIterator,
    i18n::{
        UEncoding,
        collator::{AlternateHandling, CollatorOptions, locale_cmp, try_init_collator},
        get_locale_encoding,
    },
};

use crate::syntax_tree::{MaybeNonUtf8Str, MaybeNonUtf8String};

/// Perform a locale-aware string comparison using the current locale's
/// collator.
pub(crate) fn locale_comparison(a: &MaybeNonUtf8Str, b: &MaybeNonUtf8Str) -> Ordering {
    // Initialize the collator
    let mut opts = CollatorOptions::default();
    opts.alternate_handling = Some(AlternateHandling::Shifted); // This is black magic
    let _ = try_init_collator(opts);

    locale_cmp(a, b)
}

/// Perform an index search with an approach that differs with regard to the
/// given locale.
fn index_with_locale(
    left: &MaybeNonUtf8Str,
    right: &MaybeNonUtf8Str,
    encoding: UEncoding,
) -> usize {
    match encoding {
        UEncoding::Utf8 => {
            // In the UTF-8 case, we try to decode the strings on the fly. We
            // compare UTf-8 characters as long as the stream is valid, and
            // switch to byte comparison when the byte is an invalid sequence.
            for (current_idx, ch_h) in left.iter_char_bytes().enumerate() {
                for ch_n in right.iter_char_bytes() {
                    if ch_n == ch_h {
                        return current_idx + 1;
                    }
                }
            }
            0
        }
        UEncoding::Ascii => {
            // In the default case, we just perform byte-wise comparison on the
            // arrays.
            for (current_idx, ch_h) in left.iter().enumerate() {
                for ch_n in right {
                    if ch_n == ch_h {
                        return current_idx + 1;
                    }
                }
            }
            0
        }
    }
}

/// Perform an index search with an approach that differs with regard to the
/// current locale.
pub(crate) fn locale_aware_index(left: &MaybeNonUtf8Str, right: &MaybeNonUtf8Str) -> usize {
    index_with_locale(left, right, get_locale_encoding())
}
