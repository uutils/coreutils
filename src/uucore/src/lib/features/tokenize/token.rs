// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! Traits and enums dealing with Tokenization of printf Format String
use std::io::Write;
use std::iter::Peekable;
use std::slice::Iter;

use crate::features::tokenize::sub::Sub;
use crate::features::tokenize::unescaped_text::UnescapedText;

// A token object is an object that can print the expected output
// of a contiguous segment of the format string, and
// requires at most 1 argument
pub enum Token {
    Sub(Sub),
    UnescapedText(UnescapedText),
}

impl Token {
    pub(crate) fn write<W>(&self, writer: &mut W, args: &mut Peekable<Iter<String>>)
    where
        W: Write,
    {
        match self {
            Self::Sub(sub) => sub.write(writer, args),
            Self::UnescapedText(unescaped_text) => unescaped_text.write(writer),
        }
    }
}

// A tokenizer object is an object that takes an iterator
// at a position in a format string, and sees whether
// it can return a token of a type it knows how to produce
// if so, return the token, move the iterator past the
// format string text the token represents, and if an
// argument is used move the argument iter forward one

// creating token of a format string segment should also cause
// printing of that token's value. Essentially tokenizing
// a whole format string will print the format string and consume
// a number of arguments equal to the number of argument-using tokens
