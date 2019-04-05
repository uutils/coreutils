// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//

extern crate data_encoding;
use self::data_encoding::{DecodeError, BASE32, BASE64};
use std::io::{self, Read, Write};

#[derive(Fail, Debug)]
pub enum EncodingError {
    #[fail(display = "{}", _0)]
    Decode(#[cause] DecodeError),
    #[fail(display = "{}", _0)]
    Io(#[cause] io::Error),
}

impl From<io::Error> for EncodingError {
    fn from(err: io::Error) -> EncodingError {
        EncodingError::Io(err)
    }
}

impl From<DecodeError> for EncodingError {
    fn from(err: DecodeError) -> EncodingError {
        EncodingError::Decode(err)
    }
}

pub type DecodeResult = Result<Vec<u8>, EncodingError>;

#[derive(Clone, Copy)]
pub enum Format {
    Base32,
    Base64,
}
use self::Format::*;

pub fn encode(f: Format, input: &[u8]) -> String {
    match f {
        Base32 => BASE32.encode(input),
        Base64 => BASE64.encode(input),
    }
}

pub fn decode(f: Format, input: &[u8]) -> DecodeResult {
    Ok(match f {
        Base32 => BASE32.decode(input)?,
        Base64 => BASE64.decode(input)?,
    })
}

pub struct Data<R: Read> {
    line_wrap: usize,
    ignore_garbage: bool,
    input: R,
    format: Format,
    alphabet: &'static [u8],
}

impl<R: Read> Data<R> {
    pub fn new(input: R, format: Format) -> Self {
        Data {
            line_wrap: 76,
            ignore_garbage: false,
            input,
            format,
            alphabet: match format {
                Base32 => b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567=",
                Base64 => b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789=+/",
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
        let mut buf = vec![];
        self.input.read_to_end(&mut buf)?;
        if self.ignore_garbage {
            buf.retain(|c| self.alphabet.contains(c));
        } else {
            buf.retain(|&c| c != b'\r' && c != b'\n');
        };
        decode(self.format, &buf)
    }

    pub fn encode(&mut self) -> String {
        let mut buf: Vec<u8> = vec![];
        self.input.read_to_end(&mut buf).unwrap();
        encode(self.format, buf.as_slice())
    }
}

// NOTE: this will likely be phased out at some point
pub fn wrap_print<R: Read>(data: &Data<R>, res: String) {
    let stdout = io::stdout();
    wrap_write(stdout.lock(), data.line_wrap, res).unwrap();
}

pub fn wrap_write<W: Write>(mut writer: W, line_wrap: usize, res: String) -> io::Result<()> {
    use std::cmp::min;

    if line_wrap == 0 {
        return write!(writer, "{}", res);
    }

    let mut start = 0;
    while start < res.len() {
        let end = min(start + line_wrap, res.len());
        writeln!(writer, "{}", &res[start..end])?;
        start = end;
    }

    Ok(())
}
