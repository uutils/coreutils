// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (strings) ABCDEFGHIJKLMNOPQRSTUVWXYZ ABCDEFGHIJKLMNOPQRSTUV
// spell-checker:ignore (encodings) lsbf msbf

use data_encoding::{Encoding, BASE64};
use data_encoding_macro::new_encoding;
use std::{
    error::Error,
    io::{self, Read, Write},
};

// Re-export for the faster encoding logic
pub mod for_fast_encode {
    pub use data_encoding::*;
}

#[derive(Debug)]
pub enum EncodeError {
    Z85InputLenNotMultipleOf4,
    InvalidInput,
}

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

pub fn encode_base_six_four(input: &[u8]) -> String {
    BASE64.encode(input)
}

pub fn decode_z_eight_five<R: Read>(
    mut input: R,
    ignore_garbage: bool,
) -> Result<Vec<u8>, Box<dyn Error>> {
    const Z_EIGHT_FIVE_ALPHABET: &[u8; 85_usize] =
        b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.-:+=^!/*?&<>()[]{}@%$#";

    let mut buf = Vec::<u8>::new();

    input.read_to_end(&mut buf)?;

    if ignore_garbage {
        let table = alphabet_to_table(Z_EIGHT_FIVE_ALPHABET);

        buf.retain(|&ue| table[usize::from(ue)]);
    } else {
        buf.retain(|&ue| ue != b'\n' && ue != b'\r');
    };

    // The z85 crate implements a padded encoding by using a leading '#' which is otherwise not allowed.
    // We manually check for a leading '#' and return an error ourselves.
    let vec = if buf.starts_with(b"#") {
        return Err(Box::from("'#' character at index 0 is invalid".to_owned()));
    } else {
        z85::decode(buf)?
    };

    Ok(vec)
}

pub fn encode_z_eight_five<R: Read>(mut input: R) -> Result<String, EncodeError> {
    let mut buf = Vec::<u8>::new();

    match input.read_to_end(&mut buf) {
        Ok(_) => {
            let buf_slice = buf.as_slice();

            // According to the spec we should not accept inputs whose len is not a multiple of 4.
            // However, the z85 crate implements a padded encoding and accepts such inputs. We have to manually check for them.
            if buf_slice.len() % 4_usize == 0_usize {
                Ok(z85::encode(buf_slice))
            } else {
                Err(EncodeError::Z85InputLenNotMultipleOf4)
            }
        }
        Err(_) => Err(EncodeError::InvalidInput),
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

pub fn alphabet_to_table(alphabet: &[u8]) -> [bool; 256_usize] {
    let mut table = [false; 256_usize];

    for ue in alphabet {
        let us = usize::from(*ue);

        // Should not have been set yet
        assert!(!table[us]);

        table[us] = true;
    }

    table
}
