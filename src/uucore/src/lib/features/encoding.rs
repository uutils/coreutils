// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (strings) ABCDEFGHIJKLMNOPQRSTUVWXYZ ABCDEFGHIJKLMNOPQRSTUV
// spell-checker:ignore (encodings) lsbf msbf hexupper

use self::Format::*;
use data_encoding::{Encoding, BASE32, BASE32HEX, BASE64, BASE64URL, HEXUPPER};
use data_encoding_macro::new_encoding;
use std::io::{self, Read, Write};

#[cfg(feature = "thiserror")]
use thiserror::Error;

// Re-export for the faster encoding logic
pub mod for_fast_encode {
    pub use data_encoding::*;
}

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("{}", _0)]
    Decode(#[from] data_encoding::DecodeError),
    #[error("{}", _0)]
    DecodeZ85(#[from] z85::DecodeError),
    #[error("{}", _0)]
    Io(#[from] io::Error),
}

#[derive(Debug)]
pub enum EncodeError {
    Z85InputLenNotMultipleOf4,
    InvalidInput,
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

pub const BASE2LSBF: Encoding = new_encoding! {
    symbols: "01",
    bit_order: LeastSignificantFirst,
};
pub const BASE2MSBF: Encoding = new_encoding! {
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
            if input.len() % 4 == 0 {
                z85::encode(input)
            } else {
                return Err(EncodeError::Z85InputLenNotMultipleOf4);
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
            if input.starts_with(b"#") {
                return Err(z85::DecodeError::InvalidByte(0, b'#').into());
            } else {
                z85::decode(input)?
            }
        }
    })
}

pub struct Data<R: Read> {
    input: R,
    format: Format,
    alphabet: &'static [u8],
}

impl<R: Read> Data<R> {
    pub fn new(input: R, format: Format) -> Self {
        Self {
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

    pub fn decode(&mut self, ignore_garbage: bool) -> DecodeResult {
        let mut buf = vec![];
        self.input.read_to_end(&mut buf)?;
        if ignore_garbage {
            buf.retain(|c| self.alphabet.contains(c));
        } else {
            buf.retain(|&c| c != b'\r' && c != b'\n');
        };
        decode(self.format, &buf)
    }

    pub fn encode(&mut self) -> Result<String, EncodeError> {
        let mut buf: Vec<u8> = vec![];
        match self.input.read_to_end(&mut buf) {
            Ok(_) => encode(self.format, buf.as_slice()),
            Err(_) => Err(EncodeError::InvalidInput),
        }
    }
}

pub fn wrap_print(res: &str, line_wrap: usize) -> io::Result<()> {
    let stdout = io::stdout();

    let mut stdout_lock = stdout.lock();

    if line_wrap == 0 {
        stdout_lock.write_all(res.as_bytes())?;
    } else {
        let res_len = res.len();

        let mut start = 0;

        while start < res_len {
            let start_plus_line_wrap = start + line_wrap;

            let end = start_plus_line_wrap.min(res_len);

            writeln!(stdout_lock, "{}", &res[start..end])?;

            start = end;
        }
    }

    Ok(())
}
