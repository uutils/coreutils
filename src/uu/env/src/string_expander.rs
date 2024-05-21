// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::{
    ffi::{OsStr, OsString},
    mem,
    ops::Deref,
};

use crate::{
    native_int_str::{to_native_int_representation, NativeCharInt, NativeIntStr},
    string_parser::{Chunk, Error, StringParser},
};

/// This class makes parsing and word collection more convenient.
///
/// It manages an "output" buffer that is automatically filled.
/// It provides "skip_one" and "take_one" that focus on
/// working with ASCII separators. Thus they will skip or take
/// all consecutive non-ascii char sequences at once.
pub struct StringExpander<'a> {
    parser: StringParser<'a>,
    output: Vec<NativeCharInt>,
}

impl<'a> StringExpander<'a> {
    pub fn new(input: &'a NativeIntStr) -> Self {
        Self {
            parser: StringParser::new(input),
            output: Vec::default(),
        }
    }

    pub fn new_at(input: &'a NativeIntStr, pos: usize) -> Self {
        Self {
            parser: StringParser::new_at(input, pos),
            output: Vec::default(),
        }
    }

    pub fn get_parser(&self) -> &StringParser<'a> {
        &self.parser
    }

    pub fn get_parser_mut(&mut self) -> &mut StringParser<'a> {
        &mut self.parser
    }

    pub fn peek(&self) -> Result<char, Error> {
        self.parser.peek()
    }

    pub fn skip_one(&mut self) -> Result<(), Error> {
        self.get_parser_mut().consume_one_ascii_or_all_non_ascii()?;
        Ok(())
    }

    pub fn get_peek_position(&self) -> usize {
        self.get_parser().get_peek_position()
    }

    pub fn take_one(&mut self) -> Result<(), Error> {
        let chunks = self.parser.consume_one_ascii_or_all_non_ascii()?;
        for chunk in chunks {
            match chunk {
                Chunk::InvalidEncoding(invalid) => self.output.extend(invalid),
                Chunk::ValidSingleIntChar((_c, ni)) => self.output.push(ni),
            }
        }
        Ok(())
    }

    pub fn put_one_char(&mut self, c: char) {
        let os_str = OsString::from(c.to_string());
        self.put_string(os_str);
    }

    pub fn put_string<S: AsRef<OsStr>>(&mut self, os_str: S) {
        let native = to_native_int_representation(os_str.as_ref());
        self.output.extend(native.deref());
    }

    pub fn put_native_string(&mut self, n_str: &NativeIntStr) {
        self.output.extend(n_str);
    }

    pub fn take_collected_output(&mut self) -> Vec<NativeCharInt> {
        mem::take(&mut self.output)
    }
}
