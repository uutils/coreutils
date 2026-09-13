// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore nbytes

//! Display width and byte length of UTF-8 characters within a byte buffer.

use std::str::from_utf8;
use unicode_width::UnicodeWidthChar;

/// Decode the (possibly multibyte) UTF-8 character beginning at `buf[pos]`,
/// returning the character, its display width, and its byte length.
///
/// The sequence length is taken from the leading byte's bit pattern rather than
/// its value, so 3- and 4-byte characters are measured correctly. A truncated
/// or otherwise invalid sequence resolves to `(None, 1, 1)`, which lets callers
/// resync one byte at a time.
///
/// # Panics
///
/// Panics if `pos` is out of bounds for `buf`.
#[must_use]
pub fn char_info_at(buf: &[u8], pos: usize) -> (Option<char>, usize, usize) {
    let nbytes = match buf[pos] {
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 1,
    };
    buf.get(pos..pos + nbytes)
        .and_then(|s| from_utf8(s).ok())
        .and_then(|s| s.chars().next())
        .map_or((None, 1, 1), |c| {
            (Some(c), UnicodeWidthChar::width(c).unwrap_or(0), nbytes)
        })
}

/// Return the display width and byte length of the character at `buf[pos]`.
///
/// A truncated or otherwise invalid sequence resolves to a single byte of width
/// one. See [`char_info_at`] for the decoding details.
///
/// # Panics
///
/// Panics if `pos` is out of bounds for `buf`.
#[must_use]
pub fn char_width_at(buf: &[u8], pos: usize) -> (usize, usize) {
    let (_, width, nbytes) = char_info_at(buf, pos);
    (width, nbytes)
}

#[cfg(test)]
mod tests {
    use super::{char_info_at, char_width_at};

    #[test]
    fn width_and_length() {
        assert_eq!(char_width_at(b"a", 0), (1, 1)); // ASCII
        assert_eq!(char_width_at("é".as_bytes(), 0), (1, 2)); // U+00E9, 2-byte width 1
        assert_eq!(char_width_at("\u{0301}".as_bytes(), 0), (0, 2)); // combining mark, width 0
        assert_eq!(char_width_at("\u{3000}".as_bytes(), 0), (2, 3)); // 3-byte width 2
        assert_eq!(char_width_at("\u{1F600}".as_bytes(), 0), (2, 4)); // 4-byte width 2
        assert_eq!(char_width_at(&[0xFF], 0), (1, 1)); // invalid leading byte resyncs
        assert_eq!(char_width_at(&[0xE3], 0), (1, 1)); // truncated sequence resyncs
    }

    #[test]
    fn info_decodes_char() {
        assert_eq!(
            char_info_at("\u{3000}".as_bytes(), 0),
            (Some('\u{3000}'), 2, 3)
        );
        assert_eq!(char_info_at(b"a", 0), (Some('a'), 1, 1));
        assert_eq!(char_info_at(&[0xFF], 0), (None, 1, 1)); // invalid resyncs
    }
}
