// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use memchr::{memchr, memchr2};

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

impl<'a> Matcher for ExactMatcher<'a> {
    fn next_match(&self, haystack: &[u8]) -> Option<(usize, usize)> {
        let mut pos = 0usize;
        loop {
            match memchr(self.needle[0], &haystack[pos..]) {
                Some(match_idx) => {
                    let match_idx = match_idx + pos; // account for starting from pos
                    if self.needle.len() == 1
                        || haystack[match_idx + 1..].starts_with(&self.needle[1..])
                    {
                        return Some((match_idx, match_idx + self.needle.len()));
                    } else {
                        pos = match_idx + 1;
                    }
                }
                None => {
                    return None;
                }
            }
        }
    }
}

// Matches for any number of SPACE or TAB
pub struct WhitespaceMatcher {}

impl Matcher for WhitespaceMatcher {
    fn next_match(&self, haystack: &[u8]) -> Option<(usize, usize)> {
        match memchr2(b' ', b'\t', haystack) {
            Some(match_idx) => {
                let mut skip = match_idx + 1;
                while skip < haystack.len() {
                    match haystack[skip] {
                        b' ' | b'\t' => skip += 1,
                        _ => break,
                    }
                }
                Some((match_idx, skip))
            }
            None => None,
        }
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
