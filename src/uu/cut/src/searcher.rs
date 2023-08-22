// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore multispace

use super::matcher::Matcher;

// Generic searcher that relies on a specific matcher
pub struct Searcher<'a, 'b, M: Matcher> {
    matcher: &'a M,
    haystack: &'b [u8],
    position: usize,
}

impl<'a, 'b, M: Matcher> Searcher<'a, 'b, M> {
    pub fn new(matcher: &'a M, haystack: &'b [u8]) -> Self {
        Self {
            matcher,
            haystack,
            position: 0,
        }
    }
}

// Iterate over field delimiters
// Returns (first, last) positions of each sequence, where `haystack[first..last]`
// corresponds to the delimiter.
impl<'a, 'b, M: Matcher> Iterator for Searcher<'a, 'b, M> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        match self.matcher.next_match(&self.haystack[self.position..]) {
            Some((first, last)) => {
                let result = (first + self.position, last + self.position);
                self.position += last;
                Some(result)
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod exact_searcher_tests {

    use super::super::matcher::ExactMatcher;
    use super::*;

    #[test]
    fn test_normal() {
        let matcher = ExactMatcher::new("a".as_bytes());
        let iter = Searcher::new(&matcher, "a.a.a".as_bytes());
        let items: Vec<(usize, usize)> = iter.collect();
        assert_eq!(vec![(0, 1), (2, 3), (4, 5)], items);
    }

    #[test]
    fn test_empty() {
        let matcher = ExactMatcher::new("a".as_bytes());
        let iter = Searcher::new(&matcher, "".as_bytes());
        let items: Vec<(usize, usize)> = iter.collect();
        assert_eq!(vec![] as Vec<(usize, usize)>, items);
    }

    fn test_multibyte(line: &[u8], expected: &[(usize, usize)]) {
        let matcher = ExactMatcher::new("ab".as_bytes());
        let iter = Searcher::new(&matcher, line);
        let items: Vec<(usize, usize)> = iter.collect();
        assert_eq!(expected, items);
    }

    #[test]
    fn test_multibyte_normal() {
        test_multibyte("...ab...ab...".as_bytes(), &[(3, 5), (8, 10)]);
    }

    #[test]
    fn test_multibyte_needle_head_at_end() {
        test_multibyte("a".as_bytes(), &[]);
    }

    #[test]
    fn test_multibyte_starting_needle() {
        test_multibyte("ab...ab...".as_bytes(), &[(0, 2), (5, 7)]);
    }

    #[test]
    fn test_multibyte_trailing_needle() {
        test_multibyte("...ab...ab".as_bytes(), &[(3, 5), (8, 10)]);
    }

    #[test]
    fn test_multibyte_first_byte_false_match() {
        test_multibyte("aA..aCaC..ab..aD".as_bytes(), &[(10, 12)]);
    }

    #[test]
    fn test_searcher_with_exact_matcher() {
        let matcher = ExactMatcher::new("<>".as_bytes());
        let haystack = "<><>a<>b<><>cd<><>".as_bytes();
        let mut searcher = Searcher::new(&matcher, haystack);
        assert_eq!(searcher.next(), Some((0, 2)));
        assert_eq!(searcher.next(), Some((2, 4)));
        assert_eq!(searcher.next(), Some((5, 7)));
        assert_eq!(searcher.next(), Some((8, 10)));
        assert_eq!(searcher.next(), Some((10, 12)));
        assert_eq!(searcher.next(), Some((14, 16)));
        assert_eq!(searcher.next(), Some((16, 18)));
        assert_eq!(searcher.next(), None);
        assert_eq!(searcher.next(), None);
    }
}

#[cfg(test)]
mod whitespace_searcher_tests {

    use super::super::matcher::WhitespaceMatcher;
    use super::*;

    #[test]
    fn test_space() {
        let matcher = WhitespaceMatcher {};
        let iter = Searcher::new(&matcher, " . . ".as_bytes());
        let items: Vec<(usize, usize)> = iter.collect();
        assert_eq!(vec![(0, 1), (2, 3), (4, 5)], items);
    }

    #[test]
    fn test_tab() {
        let matcher = WhitespaceMatcher {};
        let iter = Searcher::new(&matcher, "\t.\t.\t".as_bytes());
        let items: Vec<(usize, usize)> = iter.collect();
        assert_eq!(vec![(0, 1), (2, 3), (4, 5)], items);
    }

    #[test]
    fn test_empty() {
        let matcher = WhitespaceMatcher {};
        let iter = Searcher::new(&matcher, "".as_bytes());
        let items: Vec<(usize, usize)> = iter.collect();
        assert_eq!(vec![] as Vec<(usize, usize)>, items);
    }

    fn test_multispace(line: &[u8], expected: &[(usize, usize)]) {
        let matcher = WhitespaceMatcher {};
        let iter = Searcher::new(&matcher, line);
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

    #[test]
    fn test_searcher_with_whitespace_matcher() {
        let matcher = WhitespaceMatcher {};
        let haystack = "\t a b \t cd\t\t".as_bytes();
        let mut searcher = Searcher::new(&matcher, haystack);
        assert_eq!(searcher.next(), Some((0, 2)));
        assert_eq!(searcher.next(), Some((3, 4)));
        assert_eq!(searcher.next(), Some((5, 8)));
        assert_eq!(searcher.next(), Some((10, 12)));
        assert_eq!(searcher.next(), None);
        assert_eq!(searcher.next(), None);
    }
}
