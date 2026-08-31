// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! The part of the caret diagnostics that is real even without them.
//!
//! Both [`crate::diagnostics`] and its no-op stand-in re-export these, and both
//! do so for the same reason: a caller locates what a caret would point at —
//! flooring an offset, walking a list, keeping a value next to the option it
//! came from — before it knows whether anything will be drawn, so that much has
//! to work even when the rendering is compiled out. Keeping it here means the
//! two cannot drift.

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

/// The value of an option, and the option it was given to.
///
/// An option's value can be spelled many ways — `-S 1Q`, `-S1Q`,
/// `--buffer-size=1Q` — so a caret pointing inside one has to know which option
/// carried it before it can know which argument to draw under. A utility that
/// may want a caret keeps the value and the two names together from the moment
/// the parse fails until the report is drawn.
#[derive(Debug)]
pub struct OptionValue {
    /// The value as typed.
    pub value: String,
    /// The option's short name, if it has one.
    pub short: Option<char>,
    /// The option's long name, if it has one.
    pub long: Option<&'static str>,
}

impl OptionValue {
    /// The value of an option answering to both a short and a long name.
    pub fn new(value: impl Into<String>, short: char, long: &'static str) -> Self {
        Self::with_names(value, Some(short), Some(long))
    }

    /// The value of an option that is missing one of the two names, or whose
    /// names are only known once the parse has failed — `stat` blames `-c` or
    /// `--printf` depending on which one it was given.
    pub fn with_names(
        value: impl Into<String>,
        short: Option<char>,
        long: Option<&'static str>,
    ) -> Self {
        Self {
            value: value.into(),
            short,
            long,
        }
    }
}

/// The options whose value is written as a separate argument.
///
/// An operand cannot be counted off by position alone once a value can sit
/// between the operands: in `csplit -n 3 file 5` the `3` is a value, not the
/// first operand. Only the spellings that put the value in the *next*
/// argument matter; `--name=value` and `-dq` are self-contained.
pub struct ValueOptions<'a> {
    /// The short names, without the `-`.
    pub shorts: &'a [char],
    /// The long names, without the `--`. An unambiguous abbreviation counts
    /// too, since `infer_long_args` accepts one; an ambiguous one never gets
    /// this far, clap having refused it first.
    pub longs: &'a [&'a str],
}

impl ValueOptions<'_> {
    /// For a utility none of whose options takes a separate value, where an
    /// operand is simply an argument that does not look like an option.
    pub const NONE: Self = Self {
        shorts: &[],
        longs: &[],
    };

    /// Whether `arg` names one of these options in the spelling that takes the
    /// argument after it as its value.
    ///
    /// # Arguments
    ///
    /// * `arg` - An argument already known to start with `-` and not to be a
    ///   lone `-` or a bare `--`.
    pub fn takes_next(&self, arg: &str) -> bool {
        if let Some(long) = arg.strip_prefix("--") {
            // `--name=value` carries its own value.
            return !long.contains('=') && self.longs.iter().any(|name| name.starts_with(long));
        }
        // In a run of short options, only the last one can carry the value.
        arg.strip_prefix('-')
            .and_then(|cluster| cluster.chars().next_back())
            .is_some_and(|last| self.shorts.contains(&last))
    }
}

/// The items of a separated list, each with its byte range inside `list`.
///
/// A caret pointing at one item of a list — a `dd` conversion flag, a `join`
/// output field — needs to know where that item was written. The list is walked
/// rather than searched for the item's text, which would also match inside an
/// earlier item the wanted one is a prefix of: the `noc` of `nocache,noc`.
///
/// # Arguments
///
/// * `list` - The list as typed.
/// * `separators` - The characters it is split on, of any width.
pub fn list_items<'a>(
    list: &'a str,
    separators: &'a [char],
) -> impl DoubleEndedIterator<Item = (&'a str, Range<usize>)> {
    let base = list.as_ptr() as usize;
    list.split(separators).map(move |item| {
        // Every item is a slice of `list`, so its address gives away where it
        // was written: no running count to keep, which would have tied the
        // spans to walking the list once, in order, past separators of a width
        // the count assumed.
        let start = item.as_ptr() as usize - base;
        (item, start..start + item.len())
    })
}

#[cfg(test)]
mod tests {
    use super::{char_span, floor_boundary, list_items};

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

    #[test]
    fn spans_every_item_of_a_list() {
        let items: Vec<_> = list_items("ab,,cde", &[',']).collect();
        assert_eq!(items, vec![("ab", 0..2), ("", 3..3), ("cde", 4..7)]);
    }

    /// The whole point of walking: "sy" also occurs at the start of "sync".
    #[test]
    fn spans_an_item_an_earlier_one_starts_with() {
        let items: Vec<_> = list_items("sync sy", &[' ']).collect();
        assert_eq!(items[1], ("sy", 5..7));
    }

    /// The spans are a property of the list, not of the walk: taking the items
    /// out of order, or splitting on a separator that is more than one byte
    /// wide, points at the same text.
    #[test]
    fn spans_an_item_wherever_it_is_reached() {
        let items: Vec<_> = list_items("aé§bb§c", &['\u{a7}']).rev().collect();
        assert_eq!(items, vec![("c", 9..10), ("bb", 5..7), ("aé", 0..3)]);
    }

    #[test]
    fn spans_a_list_of_one() {
        let items: Vec<_> = list_items("solo", &[',', ' ']).collect();
        assert_eq!(items, vec![("solo", 0..4)]);
    }
}
