//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Michael Gehring <mg@ebfe.org>
//  * (c) kwantam <kwantam@gmail.com>
//  *     * 2015-04-28 ~ created `expand` module to eliminate most allocs during setup
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) allocs slen unesc

use std::char::from_u32;
use std::cmp::min;
use std::iter::Peekable;
use std::ops::RangeInclusive;

/// Parse a backslash escape sequence to the corresponding character. Assumes
/// the string starts from the character _after_ the `\` and is not empty.
///
/// Returns a tuple containing the character and the number of characters
/// consumed from the input. The alphabetic escape sequences consume 1
/// character; octal escape sequences consume 1 to 3 octal digits.
#[inline]
fn parse_sequence(s: &str) -> (char, usize) {
    let c = s.chars().next().expect("invalid escape: empty string");

    if ('0'..='7').contains(&c) {
        let mut v = c.to_digit(8).unwrap();
        let mut consumed = 1;
        let bits_per_digit = 3;

        for c in s.chars().skip(1).take(2) {
            match c.to_digit(8) {
                Some(c) => {
                    v = (v << bits_per_digit) | c;
                    consumed += 1;
                }
                None => break,
            }
        }

        (from_u32(v).expect("invalid octal escape"), consumed)
    } else {
        (
            match c {
                'a' => 0x07u8 as char,
                'b' => 0x08u8 as char,
                'f' => 0x0cu8 as char,
                'v' => 0x0bu8 as char,
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                c => c,
            },
            1,
        )
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
        if self.string.is_empty() {
            return None;
        }

        // is the next character an escape?
        let (ret, idx) = match self.string.chars().next().unwrap() {
            '\\' if self.string.len() > 1 => {
                // yes---it's \ and it's not the last char in a string
                // we know that \ is 1 byte long so we can index into the string safely
                let (c, consumed) = parse_sequence(&self.string[1..]);

                (Some(c), 1 + consumed)
            }
            c => (Some(c), c.len_utf8()), // not an escape char
        };

        self.string = &self.string[idx..]; // advance the pointer to the next char
        ret
    }
}

pub struct ExpandSet<'a> {
    range: RangeInclusive<u32>,
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
        for n in &mut self.range {
            if let Some(c) = from_u32(n) {
                return Some(c);
            }
        }

        if let Some(first) = self.unesc.next() {
            // peek ahead
            if self.unesc.peek() == Some(&'-') && self.unesc.size_hint().0 > 1 {
                self.unesc.next(); // this is the '-'
                let last = self.unesc.next().unwrap(); // this is the end of the range

                {
                    self.range = first as u32 + 1..=last as u32;
                }
            }

            return Some(first); // in any case, return the next char
        }

        None
    }
}

impl<'a> ExpandSet<'a> {
    #[inline]
    pub fn new(s: &'a str) -> ExpandSet<'a> {
        ExpandSet {
            range: 0..=0,
            unesc: Unescape { string: s }.peekable(),
        }
    }
}
