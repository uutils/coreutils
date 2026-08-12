// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! The character-boundary arithmetic behind the caret diagnostics.
//!
//! Both [`crate::diagnostics`] and its no-op stand-in re-export these, and both
//! do so for the same reason: a caller may floor an offset before it knows
//! whether anything will be drawn, so the arithmetic has to be real even when
//! the rendering is compiled out. Keeping it here means the two cannot drift.

use std::ops::Range;

/// `offset`, moved back to the nearest character boundary of `text`.
///
/// Offsets handed to the diagnostics are counted in bytes by someone else's
/// parser, which may have been walking an `OsStr` rather than text; one can
/// land inside a multi-byte character. Clamped to the end of `text`.
///
/// # Arguments
///
/// * `text` - The string the offset counts into.
/// * `offset` - A byte offset, trusted only as far as `text` agrees with it.
pub fn floor_boundary(text: &str, offset: usize) -> usize {
    let offset = offset.min(text.len());
    (0..=offset)
        .rev()
        .find(|&i| text.is_char_boundary(i))
        .unwrap_or(0)
}

/// The range covering the character `offset` falls in.
///
/// Useful for the errors that blame a single character, so that a caret marks
/// the whole of it rather than its first byte.
///
/// # Arguments
///
/// * `text` - The string the offset counts into.
/// * `offset` - A byte offset, trusted only as far as `text` agrees with it.
///
/// # Returns
///
/// An empty range at the end of `text`, which the renderer reads as "nothing
/// left to point at".
pub fn char_span(text: &str, offset: usize) -> Range<usize> {
    let start = floor_boundary(text, offset);
    match text[start..].chars().next() {
        Some(c) => start..start + c.len_utf8(),
        None => start..start,
    }
}

#[cfg(test)]
mod tests {
    use super::{char_span, floor_boundary};

    #[test]
    fn floors_into_a_multibyte_character() {
        // "é" is two bytes, so offset 1 is inside it.
        assert_eq!(floor_boundary("é", 1), 0);
        assert_eq!(floor_boundary("aé", 2), 1);
    }

    #[test]
    fn clamps_past_the_end() {
        assert_eq!(floor_boundary("ab", 9), 2);
    }

    #[test]
    fn spans_the_whole_character() {
        assert_eq!(char_span("aé", 1), 1..3);
        assert_eq!(char_span("aé", 2), 1..3);
    }

    #[test]
    fn spans_nothing_at_the_end() {
        assert_eq!(char_span("ab", 2), 2..2);
    }
}
