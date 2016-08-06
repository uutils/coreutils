// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//

extern crate data_encoding;
use self::data_encoding::{base64, base32, decode};
use std::io::Read;

pub type DecodeResult = Result<Vec<u8>, decode::Error>;

#[derive(Clone, Copy)]
pub enum Format {
    Base32,
    Base64,
}
use self::Format::*;

pub fn encode(f: Format, input: &[u8]) -> String {
    match f {
        Base32 => base32::encode(input),
        Base64 => base64::encode(input),
    }
}

pub fn decode(f: Format, input: &[u8]) -> DecodeResult {
    match f {
        Base32 => base32::decode(input),
        Base64 => base64::decode(input),
    }
}

pub struct Data<R: Read> {
    line_wrap: usize,
    ignore_garbage: bool,
    input: R,
    format: Format,
    alphabet: &'static str,
}

impl<R: Read> Data<R> {
    pub fn new(input: R, format: Format) -> Self {
        Data {
            line_wrap: 76,
            ignore_garbage: false,
            input: input,
            format: format,
            alphabet: match format {
                Base32 => "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567=",
                Base64 => "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789=+/",
            },
        }
    }

    pub fn line_wrap(mut self, wrap: usize) -> Self {
        self.line_wrap = wrap;
        self
    }

    pub fn ignore_garbage(mut self, ignore: bool) -> Self {
        self.ignore_garbage = ignore;
        self
    }

    pub fn decode(&mut self) -> DecodeResult {
        let mut buf = String::new();
        self.input.read_to_string(&mut buf).unwrap();
        let clean = if self.ignore_garbage {
            buf.chars()
               .filter(|&c| self.alphabet.contains(c))
               .collect::<String>()
        } else {
            buf.chars()
               .filter(|&c| c != '\r' && c != '\n')
               .collect::<String>()
        };
        decode(self.format, clean.as_bytes())
    }

    pub fn encode(&mut self) -> String {
        let mut buf: Vec<u8> = vec![];
        self.input.read_to_end(&mut buf).unwrap();
        encode(self.format, buf.as_slice())
    }
}

pub fn wrap_print(line_wrap: usize, res: String) {
    if line_wrap == 0 {
        return print!("{}", res);
    }
    use std::cmp::min;
    let mut start = 0;
    while start < res.len() {
        let end = min(start + line_wrap, res.len());
        println!("{}", &res[start..end]);
        start = end;
    }
}
