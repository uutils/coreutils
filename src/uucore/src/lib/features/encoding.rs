// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (strings) ABCDEFGHIJKLMNOPQRSTUVWXYZ ABCDEFGHIJKLMNOPQRSTUV
// spell-checker:ignore (encodings) lsbf msbf hexupper

use data_encoding::{self, BASE32, BASE64};

use std::io::{self, Read, Write};

use data_encoding::{Encoding, BASE32HEX, BASE64URL, HEXUPPER};
use data_encoding_macro::new_encoding;
#[cfg(feature = "thiserror")]
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("{}", _0)]
    Decode(#[from] data_encoding::DecodeError),
    #[error("{}", _0)]
    DecodeZ85(#[from] z85::DecodeError),
    #[error("{}", _0)]
    Io(#[from] io::Error),
}

pub enum EncodeError {
    Z85InputLenNotMultipleOf4,
}

pub type DecodeResult = Result<Vec<u8>, DecodeError>;

#[derive(Clone, Copy)]
pub enum Format {
    Base64,
    Base64Url,
    Base32,
    Base32Hex,
    Base16,
    Base2Lsbf,
    Base2Msbf,
    Z85,
}
use self::Format::*;

const BASE2LSBF: Encoding = new_encoding! {
    symbols: "01",
    bit_order: LeastSignificantFirst,
};
const BASE2MSBF: Encoding = new_encoding! {
    symbols: "01",
    bit_order: MostSignificantFirst,
};

pub fn encode(f: Format, input: &[u8]) -> Result<String, EncodeError> {
    Ok(match f {
        Base32 => BASE32.encode(input),
        Base64 => BASE64.encode(input),
        Base64Url => BASE64URL.encode(input),
        Base32Hex => BASE32HEX.encode(input),
        Base16 => HEXUPPER.encode(input),
        Base2Lsbf => BASE2LSBF.encode(input),
        Base2Msbf => BASE2MSBF.encode(input),
        Z85 => {
            // According to the spec we should not accept inputs whose len is not a multiple of 4.
            // However, the z85 crate implements a padded encoding and accepts such inputs. We have to manually check for them.
            if input.len() % 4 != 0 {
                return Err(EncodeError::Z85InputLenNotMultipleOf4);
            } else {
                z85::encode(input)
            }
        }
    })
}

pub fn decode(f: Format, input: &[u8]) -> DecodeResult {
    Ok(match f {
        Base32 => BASE32.decode(input)?,
        Base64 => BASE64.decode(input)?,
        Base64Url => BASE64URL.decode(input)?,
        Base32Hex => BASE32HEX.decode(input)?,
        Base16 => HEXUPPER.decode(input)?,
        Base2Lsbf => BASE2LSBF.decode(input)?,
        Base2Msbf => BASE2MSBF.decode(input)?,
        Z85 => {
            // The z85 crate implements a padded encoding by using a leading '#' which is otherwise not allowed.
            // We manually check for a leading '#' and return an error ourselves.
            if input.starts_with(&[b'#']) {
                return Err(z85::DecodeError::InvalidByte(0, b'#').into());
            } else {
                z85::decode(input)?
            }
        }
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
        Self {
            line_wrap: 76,
            ignore_garbage: false,
            input,
            format,
            alphabet: match format {
                Base32 => b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567=",
                Base64 => b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789=+/",
                Base64Url => b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789=_-",
                Base32Hex => b"0123456789ABCDEFGHIJKLMNOPQRSTUV=",
                Base16 => b"0123456789ABCDEF",
                Base2Lsbf => b"01",
                Base2Msbf => b"01",
                Z85 => b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.-:+=^!/*?&<>()[]{}@%$#",
            },
        }
    }

    #[must_use]
    pub fn line_wrap(mut self, wrap: usize) -> Self {
        self.line_wrap = wrap;
        self
    }

    #[must_use]
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

    pub fn encode(&mut self) -> Result<String, EncodeError> {
        let mut buf: Vec<u8> = vec![];
        self.input.read_to_end(&mut buf).unwrap();
        encode(self.format, buf.as_slice())
    }
}

// NOTE: this will likely be phased out at some point
pub fn wrap_print<R: Read>(data: &Data<R>, res: &str) {
    let stdout = io::stdout();
    wrap_write(stdout.lock(), data.line_wrap, res).unwrap();
}

pub fn wrap_write<W: Write>(mut writer: W, line_wrap: usize, res: &str) -> io::Result<()> {
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
