// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::cmp::Ordering;

use uucore::{
    CharByte, IntoCharByteIterator,
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

/// Perform a string length calculation depending on the current locale. In
/// UTF-8 locale, it will count valid UTF-8 chars, and fallback to counting
/// bytes otherwise. In Non UTF-8 locale, directly return input byte length.
pub(crate) fn locale_aware_length(input: &MaybeNonUtf8Str) -> usize {
    match get_locale_encoding() {
        UEncoding::Utf8 => std::str::from_utf8(input).map_or(input.len(), |s| s.chars().count()),
        UEncoding::Ascii => input.len(),
    }
}

fn substr_with_locale(
    s: MaybeNonUtf8String,
    pos: usize,
    len: usize,
    encoding: UEncoding,
) -> MaybeNonUtf8String {
    match encoding {
        UEncoding::Utf8 => {
            // Create a buffer with the heuristic that all the chars are ASCII
            // and are 1-byte long.
            let mut string = MaybeNonUtf8String::with_capacity(len);
            let mut buf = [0; 4];

            // Iterate on char-bytes, and skip them accordingly.
            // For each character (or byte) in the right range,
            // push it to the string.
            for cb in s.iter_char_bytes().skip(pos).take(len) {
                match cb {
                    CharByte::Char(c) => {
                        let len = c.encode_utf8(&mut buf).len();
                        string.extend(&buf[..len]);
                    }
                    CharByte::Byte(b) => string.push(b),
                }
            }
            string
        }
        UEncoding::Ascii => s.into_iter().skip(pos).take(len).collect(),
    }
}

/// Given a byte sequence, a position and a length, return the corresponding
/// substring depending on the current locale.
pub(crate) fn locale_aware_substr(
    s: MaybeNonUtf8String,
    pos: usize,
    len: usize,
) -> MaybeNonUtf8String {
    substr_with_locale(s, pos, len, get_locale_encoding())
}
