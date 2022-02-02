// This file is part of the uutils coreutils package.
//
// (c) Rolf Morel <rolfmorel@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use memchr::memchr;

pub struct Searcher<'a> {
    haystack: &'a [u8],
    needle: &'a [u8],
    position: usize,
}

impl<'a> Searcher<'a> {
    pub fn new(haystack: &'a [u8], needle: &'a [u8]) -> Searcher<'a> {
        assert!(!needle.is_empty());
        Searcher {
            haystack,
            needle,
            position: 0,
        }
    }
}

impl<'a> Iterator for Searcher<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(match_idx) = memchr(self.needle[0], self.haystack) {
                if self.needle.len() == 1
                    || self.haystack[match_idx + 1..].starts_with(&self.needle[1..])
                {
                    let match_pos = self.position + match_idx;
                    let skip = match_idx + self.needle.len();
                    self.haystack = &self.haystack[skip..];
                    self.position += skip;
                    return Some(match_pos);
                } else {
                    let skip = match_idx + 1;
                    self.haystack = &self.haystack[skip..];
                    self.position += skip;
                    // continue
                }
            } else {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    const NEEDLE: &[u8] = "ab".as_bytes();

    #[test]
    fn test_normal() {
        let iter = Searcher::new("a.a.a".as_bytes(), "a".as_bytes());
        let items: Vec<usize> = iter.collect();
        assert_eq!(vec![0, 2, 4], items);
    }

    #[test]
    fn test_empty() {
        let iter = Searcher::new("".as_bytes(), "a".as_bytes());
        let items: Vec<usize> = iter.collect();
        assert_eq!(vec![] as Vec<usize>, items);
    }

    fn test_multibyte(line: &[u8], expected: &[usize]) {
        let iter = Searcher::new(line, NEEDLE);
        let items: Vec<usize> = iter.collect();
        assert_eq!(expected, items);
    }

    #[test]
    fn test_multibyte_normal() {
        test_multibyte("...ab...ab...".as_bytes(), &[3, 8]);
    }

    #[test]
    fn test_multibyte_needle_head_at_end() {
        test_multibyte("a".as_bytes(), &[]);
    }

    #[test]
    fn test_multibyte_starting_needle() {
        test_multibyte("ab...ab...".as_bytes(), &[0, 5]);
    }

    #[test]
    fn test_multibyte_trailing_needle() {
        test_multibyte("...ab...ab".as_bytes(), &[3, 8]);
    }

    #[test]
    fn test_multibyte_first_byte_false_match() {
        test_multibyte("aA..aCaC..ab..aD".as_bytes(), &[10]);
    }
}
