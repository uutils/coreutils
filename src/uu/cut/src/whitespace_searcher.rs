// This file is part of the uutils coreutils package.
//
// (c) Rolf Morel <rolfmorel@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// cSpell:ignore multispace

use memchr::memchr2;

pub struct WhitespaceSearcher<'a> {
    haystack: &'a [u8],
    position: usize,
}

impl<'a> WhitespaceSearcher<'a> {
    pub fn new(haystack: &'a [u8]) -> WhitespaceSearcher<'a> {
        WhitespaceSearcher {
            haystack,
            position: 0,
        }
    }
}

impl<'a> Iterator for WhitespaceSearcher<'a> {
    type Item = (usize, usize);

    // Iterate over sequences of consecutive whitespace (space and/or tab) characters.
    // Returns (first, last) positions of each sequence, where `haystack[first..last]`
    // corresponds to the delimiter.
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(match_idx) = memchr2(b' ', b'\t', self.haystack) {
            let mut skip = match_idx + 1;
            while skip < self.haystack.len()
                && (self.haystack[skip] == b' ' || self.haystack[skip] == b'\t')
            {
                skip += 1;
            }
            let match_pos = self.position + match_idx;
            self.haystack = &self.haystack[skip..];
            self.position += skip;
            Some((match_pos, self.position))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_space() {
        let iter = WhitespaceSearcher::new(" . . ".as_bytes());
        let items: Vec<(usize, usize)> = iter.collect();
        assert_eq!(vec![(0, 1), (2, 3), (4, 5)], items);
    }

    #[test]
    fn test_tab() {
        let iter = WhitespaceSearcher::new("\t.\t.\t".as_bytes());
        let items: Vec<(usize, usize)> = iter.collect();
        assert_eq!(vec![(0, 1), (2, 3), (4, 5)], items);
    }

    #[test]
    fn test_empty() {
        let iter = WhitespaceSearcher::new("".as_bytes());
        let items: Vec<(usize, usize)> = iter.collect();
        assert_eq!(vec![] as Vec<(usize, usize)>, items);
    }

    fn test_multispace(line: &[u8], expected: &[(usize, usize)]) {
        let iter = WhitespaceSearcher::new(line);
        let items: Vec<(usize, usize)> = iter.collect();
        assert_eq!(expected, items);
    }

    #[test]
    fn test_multispace_normal() {
        test_multispace(
            "...  ... \t...\t ... \t ...".as_bytes(),
            &[(3, 5), (8, 10), (13, 15), (18, 21)],
        );
    }

    #[test]
    fn test_multispace_begin() {
        test_multispace(" \t\t...".as_bytes(), &[(0, 3)]);
    }

    #[test]
    fn test_multispace_end() {
        test_multispace("...\t  ".as_bytes(), &[(3, 6)]);
    }
}
