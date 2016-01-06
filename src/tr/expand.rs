/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Michael Gehring <mg@ebfe.org>
 * (c) kwantam <kwantam@gmail.com>
 *     20150428 created `expand` module to eliminate most allocs during setup
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

use std::char::from_u32;
use std::cmp::min;
use std::iter::Peekable;
use std::ops::Range;

#[inline]
fn unescape_char(c: char) -> char {
    match c {
        'a' => 0x07u8 as char,
        'b' => 0x08u8 as char,
        'f' => 0x0cu8 as char,
        'v' => 0x0bu8 as char,
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        _ => c,
    }
}

struct Unescape<'a> {
    string: &'a str,
}

impl<'a> Iterator for Unescape<'a> {
    type Item = char;

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let slen = self.string.len();
        (min(slen, 1), None)
    }

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.string.len() == 0 {
            return None;
        }

        // is the next character an escape?
        let (ret, idx) = match self.string.chars().next().unwrap() {
            '\\' if self.string.len() > 1 => {
                // yes---it's \ and it's not the last char in a string
                // we know that \ is 1 byte long so we can index into the string safely
                let c = self.string[1..].chars().next().unwrap();
                (Some(unescape_char(c)), 1 + c.len_utf8())
            },
            c => (Some(c), c.len_utf8()),   // not an escape char
        };

        self.string = &self.string[idx..];              // advance the pointer to the next char
        ret
    }
}

pub struct ExpandSet<'a> {
    range: Range<u32>,
    unesc: Peekable<Unescape<'a>>,
}

impl<'a> Iterator for ExpandSet<'a> {
    type Item = char;

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.unesc.size_hint()
    }

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // while the Range has elements, try to return chars from it
        // but make sure that they actually turn out to be Chars!
        while let Some(n) = self.range.next() {
            if let Some(c) = from_u32(n) {
                return Some(c);
            }
        }

        if let Some(first) = self.unesc.next() {
            // peek ahead
            if self.unesc.peek() == Some(&'-') && match self.unesc.size_hint() {
                (x, _) if x > 1 => true,    // there's a range here; record it in our internal Range struct
                _ => false,
            } {
                self.unesc.next();                      // this is the '-'
                let last = self.unesc.next().unwrap();  // this is the end of the range

                self.range = first as u32 + 1 .. last as u32 + 1;
            }

            return Some(first);     // in any case, return the next char
        }

        None
    }
}

impl<'a> ExpandSet<'a> {
    #[inline]
    pub fn new(s: &'a str) -> ExpandSet<'a> {
        ExpandSet {
            range: 0 .. 0,
            unesc: Unescape { string: s }.peekable(),
        }
    }
}
