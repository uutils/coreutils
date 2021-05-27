//! Iterating over lines of a file in reverse order.
//!
//! Use [`rlines_trailing_separator`] to iterate over lines of a file if
//! the line separator is at the end of each line. For example, if the
//! file is
//!
//! ```text
//! abc
//! def
//! ghi
//! ```
//!
//! then the lines in reverse order would be "ghi\n", "def\n", and
//! "abc\n".
//!
//! Use [`rlines_leading_separator`] to iterator over lines of a file if
//! the line separator is at the *beginning* of each line. For example,
//! if the file is
//!
//! ```text
//! /abc/def/ghi
//! ```
//!
//! and the line separator is assumed to be the slash character ("/"),
//! then the lines in reverse order would be "/ghi", "/def", "/abc".
use std::io::Read;
use std::io::Seek;
use uucore::chunks::rchunks;

/// The chunk size to use when reading bytes from a file.
///
/// The file will be read from the end to the beginning in chunks of
/// this many bytes. A larger number will use more memory but will
/// require less seeks and reads from the underlying file.
const CHUNK_SIZE: usize = 1 << 16;

/// A partially-read line, and whether it includes the line separator.
///
/// When reading a file in chunks, sometimes a line extends across the
/// boundaries of a chunk. For example, if the line of text is "abc\n",
/// but the first two characters are in one chunk and the second two
/// characters are in the next, we can use the [`PartialLine`] to
/// represent that each pair of characters form part of a line of text
/// that should be joined later. In this example, we could use the enum
/// like this:
///
/// ```rust,ignore
/// let part1 = PartialLine::WithoutSeparator("ab");
/// let part2 = PartialLine::WithSeparator("c\n");
/// ```
enum PartialLine<T> {
    WithoutSeparator(T),
    WithSeparator(T),
}

/// An iterator over the lines of a slice of bytes, in reverse.
///
/// This iterator yields lines where the line separator is at the
/// *beginning* of the line.
///
/// This struct is generally created by calling
/// [`rlines_leading_separator`] on a file. Please see the documentation
/// of [`rlines_leading_separator`] for more details.
struct ReverseLinesLeadingSeparator {
    /// The bytes to scan for lines from right to left.
    bytes: Vec<u8>,

    /// The index of the last byte read during iteration.
    ///
    /// This index starts at the end of the slice and decreases (that
    /// is, moves toward the beginning of the slice) each time the
    /// iterator yields.
    last_index: usize,

    /// The byte representing the character to use as a line separator.
    ///
    /// A common setting for this field is `b'\n'`.
    separator: u8,
}

impl ReverseLinesLeadingSeparator {
    fn new(bytes: Vec<u8>, separator: u8) -> ReverseLinesLeadingSeparator {
        let last_index = bytes.len();
        ReverseLinesLeadingSeparator {
            bytes,
            last_index,
            separator,
        }
    }
}

impl Iterator for ReverseLinesLeadingSeparator {
    type Item = PartialLine<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        // If we have read past the first byte, then the iterator is done.
        if self.last_index == 0 {
            return None;
        }

        // Scan from right to left, starting at the index of the
        // last-read byte, searching for the line separator
        // character. When we find it, yield the slice that starts just
        // before the line separator character and ends just before the
        // last index read in the previous call to `next()`. For
        // example, if `self.bytes` is "\nabc\ndef" and
        // `self.last_index` is 4, then this will scan from index 3 to
        // index 0, find the line separator "\n" at index 0, and return
        // the slice `self.bytes[0..4]`, which is "\nabc".
        for i in (0..self.last_index).rev() {
            if self.bytes[i] == self.separator {
                let line = self.bytes[i..self.last_index].to_vec();
                self.last_index = i;
                return Some(PartialLine::WithSeparator(line));
            }
        }

        // If we got all the way to the left-most byte and found no line
        // separators, then just yield everything up to the last index.
        let line = self.bytes[0..self.last_index].to_vec();
        self.last_index = 0;
        Some(PartialLine::WithoutSeparator(line))
    }
}

/// Returns an iterator over the lines of the file, in reverse.
///
/// The iterator returned from this function will yield [`Vec`]<[`u8`]>
/// instances. Each `Vec` represents a line of the file `f` and will
/// include the line separator (contrast this with [`BufRead::lines`])).
/// The line separator is assumed to be at the *beginning* of the
/// line. Lines are yielded in reverse order, from end of the file to
/// the beginning.
///
/// See [`rlines_trailing_separator`] for a similar iterator that
/// assumes line separators are at the end of the line.
///
/// # Examples
///
/// ```rust,ignore
/// let f = Cursor::new("\nabc\ndef");
/// let sep = b'\n';
/// let iter = rlines_leading_separator(&mut f, sep);
/// assert_eq!(iter.next(), Some(vec![b'\n', b'd', b'e', b'f']));
/// assert_eq!(iter.next(), Some(vec![b'\n', b'a', b'b', b'c']));
/// assert_eq!(iter.next(), None);
/// ```
pub fn rlines_leading_separator<T>(f: &mut T, separator: u8) -> impl Iterator<Item = Vec<u8>> + '_
where
    T: Seek + Read,
{
    let mut accumulated_partial_line = Vec::new();
    let accumulate_partial_lines = move |line: PartialLine<Vec<u8>>| match line {
        PartialLine::WithoutSeparator(mut l) => {
            l.append(&mut accumulated_partial_line);
            accumulated_partial_line = l;
            None
        }
        PartialLine::WithSeparator(mut l) => {
            l.append(&mut accumulated_partial_line);
            if l.is_empty() {
                None
            } else {
                Some(l)
            }
        }
    };
    let sentinel = std::iter::once(PartialLine::WithSeparator(b"".to_vec()));
    rchunks(f, CHUNK_SIZE)
        .flat_map(move |c| ReverseLinesLeadingSeparator::new(c, separator))
        .chain(sentinel)
        .filter_map(accumulate_partial_lines)
}

/// An iterator over the lines of a slice of bytes, in reverse.
///
/// This iterator yields lines where the line separator is at the end of
/// the line.
///
/// This struct is generally created by calling
/// [`rlines_trailing_separator`] on a file. Please see the documentation
/// of [`rlines_trailing_separator`] for more details.
struct ReverseLinesTrailingSeparator {
    /// The bytes to scan for lines from right to left.
    bytes: Vec<u8>,

    /// The index of the last byte read during iteration.
    ///
    /// This index starts at the end of the slice and decreases (that
    /// is, moves toward the beginning of the slice) each time the
    /// iterator yields.
    last_index: usize,

    /// The total number of bytes being scanned.
    n: usize,

    /// The byte representing the character to use as a line separator.
    ///
    /// A common setting for this field is `b'\n'`.
    separator: u8,

    /// Whether the iterator is exhausted.
    is_done: bool,
}

impl ReverseLinesTrailingSeparator {
    fn new(bytes: Vec<u8>, separator: u8) -> ReverseLinesTrailingSeparator {
        let n = bytes.len();
        let last_index = n;
        let is_done = false;
        ReverseLinesTrailingSeparator {
            bytes,
            last_index,
            n,
            separator,
            is_done,
        }
    }
}

impl Iterator for ReverseLinesTrailingSeparator {
    type Item = PartialLine<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        // If we have read from right to left to the first byte,
        // determine whether the first byte is a line separator, in
        // which case it needs to be yielded as its own line.
        if self.last_index == 0 {
            if self.n == 1 || self.is_done {
                return None;
            }
            self.is_done = true;
            if self.bytes[0] != self.separator {
                return None;
            } else {
                return Some(PartialLine::WithSeparator(vec![self.bytes[0]]));
            }
        }

        // Scan from right to left, starting at the index of the
        // last-read byte, searching for the line separator
        // character. When we find it, yield the slice that starts just
        // *after* the line separator character and ends just *after*
        // the last index read in the previous call to `next()`. For
        // example, if `self.bytes` is "abc\ndef\n" and
        // `self.last_index` is 8, then this will scan from index 7 to
        // index 3, find the line separator "\n" at index 3, and return
        // the slice `self.bytes[4..8]`, which is "def\n".
        for i in (0..self.last_index).rev() {
            // The `i < self.n - 1` prevents us from yielding the last
            // line separator, if the last byte is a line separator, as
            // if it were on its own line.
            if self.bytes[i] == self.separator && i < self.n - 1 {
                let start = i + 1;
                let end = (self.last_index + 1).min(self.n);
                let line = self.bytes[start..end].to_vec();
                self.last_index = i;
                match line.last() {
                    Some(&c) if c == self.separator => {
                        return Some(PartialLine::WithSeparator(line));
                    }
                    Some(_) | None => {
                        return Some(PartialLine::WithoutSeparator(line));
                    }
                }
            }
        }

        // If we got all the way to the left-most byte and found no line
        // separators, then just yield everything up to the last index.
        let start = 0;
        let end = (self.last_index + 1).min(self.n);
        let line = self.bytes[start..end].to_vec();
        self.last_index = 0;
        match line.last() {
            Some(&c) if c == self.separator => Some(PartialLine::WithSeparator(line)),
            Some(_) | None => Some(PartialLine::WithoutSeparator(line)),
        }
    }
}

/// Returns an iterator over the lines of the file, in reverse.
///
/// The iterator returned from this function will yield [`Vec`]<[`u8`]>
/// instances. Each `Vec` represents a line of the file `f` and will
/// include the line separator (contrast this with [`BufRead::lines`])).
/// The line separator is assumed to be at the *end* of the line. Lines
/// are yielded in reverse order, from end of the file to the beginning.
///
/// See [`rlines_leading_separator`] for a similar iterator that assumes
/// line separators are at the *beginning* of the line.
///
/// # Examples
///
/// ```rust,ignore
/// let f = Cursor::new("abc\ndef\n");
/// let sep = b'\n';
/// let iter = rlines_leading_separator(&mut f, sep);
/// assert_eq!(iter.next(), Some(vec![b'd', b'e', b'f', b'\n']));
/// assert_eq!(iter.next(), Some(vec![b'a', b'b', b'c', b'\n']));
/// assert_eq!(iter.next(), None);
/// ```
pub fn rlines_trailing_separator<T>(f: &mut T, separator: u8) -> impl Iterator<Item = Vec<u8>> + '_
where
    T: Seek + Read,
{
    let mut accumulated_partial_line = Vec::new();
    let accumulate_partial_lines = move |line: PartialLine<Vec<u8>>| match line {
        PartialLine::WithSeparator(l) => {
            let result = accumulated_partial_line.clone();
            accumulated_partial_line = l;
            (!result.is_empty()).then(|| result)
        }
        PartialLine::WithoutSeparator(mut l) => {
            l.append(&mut accumulated_partial_line);
            accumulated_partial_line = l;
            None
        }
    };
    let sentinel = std::iter::once(PartialLine::WithSeparator(b"".to_vec()));
    rchunks(f, CHUNK_SIZE)
        .flat_map(move |c| ReverseLinesTrailingSeparator::new(c, separator))
        .chain(sentinel)
        .filter_map(accumulate_partial_lines)
}

#[cfg(test)]
mod tests {

    mod rlines {
        use crate::lines::rlines_leading_separator;
        use std::io::Cursor;

        #[test]
        fn test_empty_file() {
            let actual: Vec<Vec<u8>> =
                rlines_leading_separator(&mut Cursor::new(""), b'\n').collect();
            let expected = Vec::<Vec<u8>>::new();
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_only_separator() {
            let actual: Vec<Vec<u8>> =
                rlines_leading_separator(&mut Cursor::new("\n"), b'\n').collect();
            let mut expected = Vec::<Vec<u8>>::new();
            expected.push("\n".bytes().collect());
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_no_line_endings() {
            let actual: Vec<Vec<u8>> =
                rlines_leading_separator(&mut Cursor::new("abc"), b'\n').collect();
            let mut expected = Vec::<Vec<u8>>::new();
            expected.push("abc".bytes().collect());
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_trailing_line_ending() {
            let actual: Vec<Vec<u8>> =
                rlines_leading_separator(&mut Cursor::new("abc\n"), b'\n').collect();
            let mut expected = Vec::<Vec<u8>>::new();
            expected.push("\n".bytes().collect());
            expected.push("abc".bytes().collect());
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_leading_line_ending() {
            let actual: Vec<Vec<u8>> =
                rlines_leading_separator(&mut Cursor::new("\nabc"), b'\n').collect();
            let mut expected = Vec::<Vec<u8>>::new();
            expected.push("\nabc".bytes().collect());
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_multiple_lines() {
            let actual: Vec<Vec<u8>> =
                rlines_leading_separator(&mut Cursor::new("\nabc\ndef"), b'\n').collect();
            let mut expected = Vec::<Vec<u8>>::new();
            expected.push("\ndef".bytes().collect());
            expected.push("\nabc".bytes().collect());
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_separator() {
            let actual: Vec<Vec<u8>> =
                rlines_leading_separator(&mut Cursor::new(":abc:def"), b':').collect();
            let mut expected = Vec::<Vec<u8>>::new();
            expected.push(":def".bytes().collect());
            expected.push(":abc".bytes().collect());
            assert_eq!(actual, expected);
        }
    }

    mod rlines_trailing_separator {
        use crate::lines::rlines_trailing_separator;
        use std::io::Cursor;

        #[test]
        fn test_empty_file() {
            let actual: Vec<Vec<u8>> =
                rlines_trailing_separator(&mut Cursor::new(""), b'\n').collect();
            let expected = Vec::<Vec<u8>>::new();
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_only_separator() {
            let actual: Vec<Vec<u8>> =
                rlines_trailing_separator(&mut Cursor::new("\n"), b'\n').collect();
            let mut expected = Vec::<Vec<u8>>::new();
            expected.push("\n".bytes().collect());
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_no_line_endings() {
            let actual: Vec<Vec<u8>> =
                rlines_trailing_separator(&mut Cursor::new("abc"), b'\n').collect();
            let mut expected = Vec::<Vec<u8>>::new();
            expected.push("abc".bytes().collect());
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_trailing_line_ending() {
            let actual: Vec<Vec<u8>> =
                rlines_trailing_separator(&mut Cursor::new("abc\n"), b'\n').collect();
            let mut expected = Vec::<Vec<u8>>::new();
            expected.push("abc\n".bytes().collect());
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_leading_line_ending() {
            let actual: Vec<Vec<u8>> =
                rlines_trailing_separator(&mut Cursor::new("\nabc"), b'\n').collect();
            let mut expected = Vec::<Vec<u8>>::new();
            expected.push("abc".bytes().collect());
            expected.push("\n".bytes().collect());
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_multiple_lines() {
            let actual: Vec<Vec<u8>> =
                rlines_trailing_separator(&mut Cursor::new("abc\ndef\n"), b'\n').collect();
            let mut expected = Vec::<Vec<u8>>::new();
            expected.push("def\n".bytes().collect());
            expected.push("abc\n".bytes().collect());
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_separator() {
            let actual: Vec<Vec<u8>> =
                rlines_trailing_separator(&mut Cursor::new("abc:def:"), b':').collect();
            let mut expected = Vec::<Vec<u8>>::new();
            expected.push("def:".bytes().collect());
            expected.push("abc:".bytes().collect());
            assert_eq!(actual, expected);
        }
    }
}
