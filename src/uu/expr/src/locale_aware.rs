// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

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
            left.iter_char_bytes()
                .position(|ch_h| right.iter_char_bytes().any(|ch_n| ch_n == ch_h))
                .map_or(0, |idx| idx + 1)
        }
        UEncoding::Ascii => {
            // In the default case, we just perform byte-wise comparison on the
            // arrays.
            left.iter()
                .position(|ch_h| right.iter().any(|ch_n| ch_n == ch_h))
                .map_or(0, |idx| idx + 1)
        }
    }
}

/// Perform an index search with an approach that differs with regard to the
/// current locale.
pub(crate) fn locale_aware_index(left: &MaybeNonUtf8Str, right: &MaybeNonUtf8Str) -> usize {
    index_with_locale(left, right, get_locale_encoding())
}
