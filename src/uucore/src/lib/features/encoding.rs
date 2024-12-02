// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (encodings) lsbf msbf
// spell-checker:ignore unpadded

use crate::error::{UResult, USimpleError};
use data_encoding::Encoding;
use data_encoding_macro::new_encoding;
use std::collections::VecDeque;

// Re-export for the faster decoding/encoding logic
pub mod for_base_common {
    pub use data_encoding::*;
}

pub mod for_cksum {
    pub use data_encoding::BASE64;
}

#[derive(Clone, Copy, Debug)]
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

pub struct Z85Wrapper {}

pub struct EncodingWrapper {
    pub alphabet: &'static [u8],
    pub encoding: Encoding,
    pub unpadded_multiple: usize,
    pub valid_decoding_multiple: usize,
}

impl EncodingWrapper {
    pub fn new(
        encoding: Encoding,
        valid_decoding_multiple: usize,
        unpadded_multiple: usize,
        alphabet: &'static [u8],
    ) -> Self {
        assert!(valid_decoding_multiple > 0);

        assert!(unpadded_multiple > 0);

        assert!(!alphabet.is_empty());

        Self {
            alphabet,
            encoding,
            unpadded_multiple,
            valid_decoding_multiple,
        }
    }
}

pub trait SupportsFastDecodeAndEncode {
    /// Returns the list of characters used by this encoding
    fn alphabet(&self) -> &'static [u8];

    fn decode_into_vec(&self, input: &[u8], output: &mut Vec<u8>) -> UResult<()>;

    fn encode_to_vec_deque(&self, input: &[u8], output: &mut VecDeque<u8>) -> UResult<()>;

    /// Inputs with a length that is a multiple of this number do not have padding when encoded. For instance:
    ///
    /// "The quick brown"
    ///
    /// is 15 characters (divisible by 3), so it is encoded in Base64 without padding:
    ///
    /// "VGhlIHF1aWNrIGJyb3du"
    ///
    /// While:
    ///
    /// "The quick brown fox"
    ///
    /// is 19 characters, which is not divisible by 3, so its Base64 representation has padding:
    ///
    /// "VGhlIHF1aWNrIGJyb3duIGZveA=="
    ///
    /// The encoding performed by `fast_encode` depends on this number being correct.
    fn unpadded_multiple(&self) -> usize;

    /// Data to decode must be a length that is multiple of this number
    ///
    /// The decoding performed by `fast_decode` depends on this number being correct.
    fn valid_decoding_multiple(&self) -> usize;
}

impl SupportsFastDecodeAndEncode for Z85Wrapper {
    fn alphabet(&self) -> &'static [u8] {
        // Z85 alphabet
        b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.-:+=^!/*?&<>()[]{}@%$#"
    }

    fn decode_into_vec(&self, input: &[u8], output: &mut Vec<u8>) -> UResult<()> {
        if input.first() == Some(&b'#') {
            return Err(USimpleError::new(1, "error: invalid input".to_owned()));
        }

        let decode_result = match z85::decode(input) {
            Ok(ve) => ve,
            Err(_de) => {
                return Err(USimpleError::new(1, "error: invalid input".to_owned()));
            }
        };

        output.extend_from_slice(&decode_result);

        Ok(())
    }

    fn valid_decoding_multiple(&self) -> usize {
        5
    }

    fn encode_to_vec_deque(&self, input: &[u8], output: &mut VecDeque<u8>) -> UResult<()> {
        // According to the spec we should not accept inputs whose len is not a multiple of 4.
        // However, the z85 crate implements a padded encoding and accepts such inputs. We have to manually check for them.
        if input.len() % 4 != 0 {
            return Err(USimpleError::new(
                1,
                "error: invalid input (length must be multiple of 4 characters)".to_owned(),
            ));
        }

        let string = z85::encode(input);

        output.extend(string.as_bytes());

        Ok(())
    }

    fn unpadded_multiple(&self) -> usize {
        4
    }
}

impl SupportsFastDecodeAndEncode for EncodingWrapper {
    fn alphabet(&self) -> &'static [u8] {
        self.alphabet
    }

    // Adapted from `decode` in the "data-encoding" crate
    fn decode_into_vec(&self, input: &[u8], output: &mut Vec<u8>) -> UResult<()> {
        let decode_len_result = match self.encoding.decode_len(input.len()) {
            Ok(us) => us,
            Err(_de) => {
                return Err(USimpleError::new(1, "error: invalid input".to_owned()));
            }
        };

        let output_len = output.len();

        output.resize(output_len + decode_len_result, 0);

        match self.encoding.decode_mut(input, &mut (output[output_len..])) {
            Ok(us) => {
                // See:
                // https://docs.rs/data-encoding/latest/data_encoding/struct.Encoding.html#method.decode_mut
                // "Returns the length of the decoded output. This length may be smaller than the output length if the input contained padding or ignored characters. The output bytes after the returned length are not initialized and should not be read."
                output.truncate(output_len + us);
            }
            Err(_de) => {
                return Err(USimpleError::new(1, "error: invalid input".to_owned()));
            }
        }

        Ok(())
    }

    fn valid_decoding_multiple(&self) -> usize {
        self.valid_decoding_multiple
    }

    // Adapted from `encode_append` in the "data-encoding" crate
    fn encode_to_vec_deque(&self, input: &[u8], output: &mut VecDeque<u8>) -> UResult<()> {
        let output_len = output.len();

        output.resize(output_len + self.encoding.encode_len(input.len()), 0);

        let make_contiguous_result = output.make_contiguous();

        self.encoding
            .encode_mut(input, &mut (make_contiguous_result[output_len..]));

        Ok(())
    }

    fn unpadded_multiple(&self) -> usize {
        self.unpadded_multiple
    }
}
