use uucore::{
    IntoCharByteIterator,
    i18n::{UEncoding, get_locale_encoding},
};

use crate::syntax_tree::{MaybeNonUtf8Str, MaybeNonUtf8String};

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
