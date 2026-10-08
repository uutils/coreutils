// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore clen

use memchr::{memchr, memchr2};
use uucore::i18n::charmap::{Encoding, locale_encoding};

// Find the next matching byte sequence positions
// Return (first, last) where haystack[first..last] corresponds to the matched pattern
pub trait Matcher {
    fn next_match(&self, haystack: &[u8]) -> Option<(usize, usize)>;
}

// Matches for the exact byte sequence pattern
pub struct ExactMatcher<'a> {
    needle: &'a [u8],
}

impl<'a> ExactMatcher<'a> {
    pub fn new(needle: &'a [u8]) -> Self {
        assert!(!needle.is_empty());
        Self { needle }
    }
}

impl Matcher for ExactMatcher<'_> {
    fn next_match(&self, haystack: &[u8]) -> Option<(usize, usize)> {
        let mut pos = 0usize;
        loop {
            let match_idx = memchr(self.needle[0], &haystack[pos..])?;
            let match_idx = match_idx + pos; // account for starting from pos

            if self.needle.len() == 1 || haystack[match_idx + 1..].starts_with(&self.needle[1..]) {
                return Some((match_idx, match_idx + self.needle.len()));
            }

            pos = match_idx + 1;
        }
    }
}

// Matches the delimiter as a whole character, never inside a multi-byte
// character. Used for delimiters that could be a continuation byte (e.g. a
// lone `0xa9`) or a multi-byte character; ASCII delimiters use `ExactMatcher`.
pub struct MbExactMatcher<'a> {
    needle: &'a [u8],
}

impl<'a> MbExactMatcher<'a> {
    pub fn new(needle: &'a [u8]) -> Self {
        assert!(!needle.is_empty());
        Self { needle }
    }
}

impl Matcher for MbExactMatcher<'_> {
    fn next_match(&self, haystack: &[u8]) -> Option<(usize, usize)> {
        // Resolved once per line: character boundaries have to be walked from
        // the start, so there is no way to skip ahead with `memchr` here.
        let encoding = locale_encoding();
        let mut pos = 0;
        while pos < haystack.len() {
            let clen = encoding.char_len(&haystack[pos..]);
            if clen == self.needle.len() && &haystack[pos..pos + clen] == self.needle {
                return Some((pos, pos + clen));
            }
            pos += clen;
        }
        None
    }
}

// Matches any number of whitespace characters. ASCII space and tab are always
// recognized; in a UTF-8 locale Unicode space separators are too.
pub struct WhitespaceMatcher {}

impl Matcher for WhitespaceMatcher {
    fn next_match(&self, haystack: &[u8]) -> Option<(usize, usize)> {
        // In a single-byte locale SPACE and TAB are the only blanks, so the run
        // can be found with a SIMD scan instead of decoding every character.
        let encoding = locale_encoding();
        if encoding == Encoding::SingleByte {
            let start = memchr2(b' ', b'\t', haystack)?;
            let mut end = start + 1;
            while end < haystack.len() && matches!(haystack[end], b' ' | b'\t') {
                end += 1;
            }
            return Some((start, end));
        }

        let mut pos = 0;
        while pos < haystack.len() {
            if let Some(blank_len) = encoding.blank_len(&haystack[pos..]) {
                let start = pos;
                pos += blank_len;
                while pos < haystack.len() {
                    match encoding.blank_len(&haystack[pos..]) {
                        Some(len) => pos += len,
                        None => break,
                    }
                }
                return Some((start, pos));
            }
            pos += encoding.char_len(&haystack[pos..]);
        }
        None
    }
}

#[cfg(test)]
mod matcher_tests {

    use super::*;

    #[test]
    fn test_exact_matcher_single_byte() {
        let matcher = ExactMatcher::new(":".as_bytes());
        // spell-checker:disable
        assert_eq!(matcher.next_match("".as_bytes()), None);
        assert_eq!(matcher.next_match(":".as_bytes()), Some((0, 1)));
        assert_eq!(matcher.next_match(":abcxyz".as_bytes()), Some((0, 1)));
        assert_eq!(matcher.next_match("abc:xyz".as_bytes()), Some((3, 4)));
        assert_eq!(matcher.next_match("abcxyz:".as_bytes()), Some((6, 7)));
        assert_eq!(matcher.next_match("abcxyz".as_bytes()), None);
        // spell-checker:enable
    }

    #[test]
    fn test_exact_matcher_multi_bytes() {
        let matcher = ExactMatcher::new("<>".as_bytes());
        // spell-checker:disable
        assert_eq!(matcher.next_match("".as_bytes()), None);
        assert_eq!(matcher.next_match("<>".as_bytes()), Some((0, 2)));
        assert_eq!(matcher.next_match("<>abcxyz".as_bytes()), Some((0, 2)));
        assert_eq!(matcher.next_match("abc<>xyz".as_bytes()), Some((3, 5)));
        assert_eq!(matcher.next_match("abcxyz<>".as_bytes()), Some((6, 8)));
        assert_eq!(matcher.next_match("abcxyz".as_bytes()), None);
        // spell-checker:enable
    }

    #[test]
    fn test_whitespace_matcher_single_space() {
        let matcher = WhitespaceMatcher {};
        // spell-checker:disable
        assert_eq!(matcher.next_match("".as_bytes()), None);
        assert_eq!(matcher.next_match(" ".as_bytes()), Some((0, 1)));
        assert_eq!(matcher.next_match("\tabcxyz".as_bytes()), Some((0, 1)));
        assert_eq!(matcher.next_match("abc\txyz".as_bytes()), Some((3, 4)));
        assert_eq!(matcher.next_match("abcxyz ".as_bytes()), Some((6, 7)));
        assert_eq!(matcher.next_match("abcxyz".as_bytes()), None);
        // spell-checker:enable
    }

    #[test]
    fn test_whitespace_matcher_multi_spaces() {
        let matcher = WhitespaceMatcher {};
        // spell-checker:disable
        assert_eq!(matcher.next_match("".as_bytes()), None);
        assert_eq!(matcher.next_match(" \t ".as_bytes()), Some((0, 3)));
        assert_eq!(matcher.next_match("\t\tabcxyz".as_bytes()), Some((0, 2)));
        assert_eq!(matcher.next_match("abc \txyz".as_bytes()), Some((3, 5)));
        assert_eq!(matcher.next_match("abcxyz  ".as_bytes()), Some((6, 8)));
        assert_eq!(matcher.next_match("abcxyz".as_bytes()), None);
        // spell-checker:enable
    }
}
