//! Traits and enums dealing with Tokenization of printf Format String
use itertools::PutBackN;
use std::iter::Peekable;
use std::slice::Iter;
use std::str::Chars;

use crate::error::UResult;

// A token object is an object that can print the expected output
// of a contiguous segment of the format string, and
// requires at most 1 argument
pub trait Token {
    fn print(&self, args: &mut Peekable<Iter<String>>);
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

pub trait Tokenizer {
    fn from_it(
        it: &mut PutBackN<Chars>,
        args: &mut Peekable<Iter<String>>,
    ) -> UResult<Option<Box<dyn Token>>>;
}
