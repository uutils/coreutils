/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Rolf Morel <rolfmorel@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[derive(Clone)]
pub struct Searcher<'a> {
    haystack: &'a [u8],
    needle: &'a [u8],
    position: usize 
}

impl<'a> Searcher<'a> {
    pub fn new(haystack: &'a [u8], needle: &'a [u8]) -> Searcher<'a> {
        Searcher {
            haystack: haystack,
            needle: needle,
            position: 0
        }
    }
}

impl<'a> Iterator for Searcher<'a> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<(usize, usize)> {
        if self.needle.len() == 1 {
            for offset in self.position..self.haystack.len() {
                if self.haystack[offset] == self.needle[0] {
                    self.position = offset + 1;
                    return Some((offset, offset + 1));
                }
            }

            self.position = self.haystack.len();
            return None;
        }

        while self.position + self.needle.len() <= self.haystack.len() {
            if &self.haystack[self.position..self.position + self.needle.len()] == self.needle {
                let match_pos = self.position;
                self.position += self.needle.len();
                return Some((match_pos, match_pos + self.needle.len()));
            } else {
                self.position += 1;
            }
        }
        None
    }
}
