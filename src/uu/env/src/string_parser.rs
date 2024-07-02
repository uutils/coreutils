// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//
// spell-checker:ignore (words) FFFD
#![forbid(unsafe_code)]

use std::{borrow::Cow, ffi::OsStr};

use crate::native_int_str::{
    from_native_int_representation, get_char_from_native_int, get_single_native_int_value,
    NativeCharInt, NativeIntStr,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Error {
    pub peek_position: usize,
    pub err_type: ErrorType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorType {
    EndOfInput,
    InternalError,
}

/// Provides a valid char or a invalid sequence of bytes.
///
/// Invalid byte sequences can't be split in any meaningful way.
/// Thus, they need to be consumed as one piece.
pub enum Chunk<'a> {
    InvalidEncoding(&'a NativeIntStr),
    ValidSingleIntChar((char, NativeCharInt)),
}

/// This class makes parsing a OsString char by char more convenient.
///
/// It also allows to capturing of intermediate positions for later splitting.
pub struct StringParser<'a> {
    input: &'a NativeIntStr,
    pointer: usize,
    remaining: &'a NativeIntStr,
}

impl<'a> StringParser<'a> {
    pub fn new(input: &'a NativeIntStr) -> Self {
        let mut instance = Self {
            input,
            pointer: 0,
            remaining: input,
        };
        instance.set_pointer(0);
        instance
    }

    pub fn new_at(input: &'a NativeIntStr, pos: usize) -> Self {
        let mut instance = Self::new(input);
        instance.set_pointer(pos);
        instance
    }

    pub fn get_input(&self) -> &'a NativeIntStr {
        self.input
    }

    pub fn get_peek_position(&self) -> usize {
        self.pointer
    }

    pub fn peek(&self) -> Result<char, Error> {
        self.peek_char_at_pointer(self.pointer)
    }

    fn make_err(&self, err_type: ErrorType) -> Error {
        Error {
            peek_position: self.get_peek_position(),
            err_type,
        }
    }

    pub fn peek_char_at_pointer(&self, at_pointer: usize) -> Result<char, Error> {
        let split = self.input.split_at(at_pointer).1;
        if split.is_empty() {
            return Err(self.make_err(ErrorType::EndOfInput));
        }
        if let Some((c, _ni)) = get_char_from_native_int(split[0]) {
            Ok(c)
        } else {
            Ok('\u{FFFD}')
        }
    }

    fn get_chunk_with_length_at(&self, pointer: usize) -> Result<(Chunk<'a>, usize), Error> {
        let (_before, after) = self.input.split_at(pointer);
        if after.is_empty() {
            return Err(self.make_err(ErrorType::EndOfInput));
        }

        if let Some(c_ni) = get_char_from_native_int(after[0]) {
            Ok((Chunk::ValidSingleIntChar(c_ni), 1))
        } else {
            let mut i = 1;
            while i < after.len() {
                if let Some(_c) = get_char_from_native_int(after[i]) {
                    break;
                }
                i += 1;
            }

            let chunk = &after[0..i];
            Ok((Chunk::InvalidEncoding(chunk), chunk.len()))
        }
    }

    pub fn peek_chunk(&self) -> Option<Chunk<'a>> {
        return self
            .get_chunk_with_length_at(self.pointer)
            .ok()
            .map(|(chunk, _)| chunk);
    }

    pub fn consume_chunk(&mut self) -> Result<Chunk<'a>, Error> {
        let (chunk, len) = self.get_chunk_with_length_at(self.pointer)?;
        self.set_pointer(self.pointer + len);
        Ok(chunk)
    }

    pub fn consume_one_ascii_or_all_non_ascii(&mut self) -> Result<Vec<Chunk<'a>>, Error> {
        let mut result = Vec::<Chunk<'a>>::new();
        loop {
            let data = self.consume_chunk()?;
            let was_ascii = if let Chunk::ValidSingleIntChar((c, _ni)) = &data {
                c.is_ascii()
            } else {
                false
            };
            result.push(data);
            if was_ascii {
                return Ok(result);
            }

            match self.peek_chunk() {
                Some(Chunk::ValidSingleIntChar((c, _ni))) if c.is_ascii() => return Ok(result),
                None => return Ok(result),
                _ => {}
            }
        }
    }

    pub fn skip_multiple(&mut self, skip_byte_count: usize) {
        let end_ptr = self.pointer + skip_byte_count;
        self.set_pointer(end_ptr);
    }

    pub fn skip_until_char_or_end(&mut self, c: char) {
        let native_rep = get_single_native_int_value(&c).unwrap();
        let pos = self.remaining.iter().position(|x| *x == native_rep);

        if let Some(pos) = pos {
            self.set_pointer(self.pointer + pos);
        } else {
            self.set_pointer(self.input.len());
        }
    }

    pub fn substring(&self, range: &std::ops::Range<usize>) -> &'a NativeIntStr {
        let (_before1, after1) = self.input.split_at(range.start);
        let (middle, _after2) = after1.split_at(range.end - range.start);
        middle
    }

    pub fn peek_remaining(&self) -> Cow<'a, OsStr> {
        from_native_int_representation(Cow::Borrowed(self.remaining))
    }

    pub fn set_pointer(&mut self, new_pointer: usize) {
        self.pointer = new_pointer;
        let (_before, after) = self.input.split_at(self.pointer);
        self.remaining = after;
    }
}
