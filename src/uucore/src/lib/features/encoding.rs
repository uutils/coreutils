// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (encodings) lsbf msbf

use crate::error::{UResult, USimpleError};
use data_encoding::Encoding;
use data_encoding_macro::new_encoding;
use std::collections::VecDeque;

// Re-export for the faster encoding logic
pub mod for_fast_encode {
    pub use data_encoding::*;
}

pub mod for_cksum {
    pub use data_encoding::BASE64;
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

pub struct ZEightFiveWrapper {}

pub trait SupportsFastEncode {
    fn encode_to_vec_deque(&self, input: &[u8], output: &mut VecDeque<u8>) -> UResult<()>;
}

impl SupportsFastEncode for ZEightFiveWrapper {
    fn encode_to_vec_deque(&self, input: &[u8], output: &mut VecDeque<u8>) -> UResult<()> {
        // According to the spec we should not accept inputs whose len is not a multiple of 4.
        // However, the z85 crate implements a padded encoding and accepts such inputs. We have to manually check for them.
        if input.len() % 4_usize != 0_usize {
            return Err(USimpleError::new(
                1_i32,
                "error: invalid input (length must be multiple of 4 characters)".to_owned(),
            ));
        }

        let string = z85::encode(input);

        output.extend(string.as_bytes());

        Ok(())
    }
}

impl SupportsFastEncode for Encoding {
    // Adapted from `encode_append` in the "data-encoding" crate
    fn encode_to_vec_deque(&self, input: &[u8], output: &mut VecDeque<u8>) -> UResult<()> {
        let output_len = output.len();

        output.resize(output_len + self.encode_len(input.len()), 0_u8);

        let make_contiguous_result = output.make_contiguous();

        self.encode_mut(input, &mut (make_contiguous_result[output_len..]));

        Ok(())
    }
}

pub trait SupportsFastDecode {
    fn decode_into_vec(&self, input: &[u8], output: &mut Vec<u8>) -> UResult<()>;
}

impl SupportsFastDecode for ZEightFiveWrapper {
    fn decode_into_vec(&self, input: &[u8], output: &mut Vec<u8>) -> UResult<()> {
        if input.first() == Some(&b'#') {
            return Err(USimpleError::new(1_i32, "error: invalid input".to_owned()));
        }

        // According to the spec we should not accept inputs whose len is not a multiple of 4.
        // However, the z85 crate implements a padded encoding and accepts such inputs. We have to manually check for them.
        if input.len() % 4_usize != 0_usize {
            return Err(USimpleError::new(
                1_i32,
                "error: invalid input (length must be multiple of 4 characters)".to_owned(),
            ));
        };

        let decode_result = match z85::decode(input) {
            Ok(ve) => ve,
            Err(_de) => {
                return Err(USimpleError::new(1_i32, "error: invalid input".to_owned()));
            }
        };

        output.extend_from_slice(&decode_result);

        Ok(())
    }
}

impl SupportsFastDecode for Encoding {
    // Adapted from `decode` in the "data-encoding" crate
    fn decode_into_vec(&self, input: &[u8], output: &mut Vec<u8>) -> UResult<()> {
        let decode_len_result = match self.decode_len(input.len()) {
            Ok(us) => us,
            Err(_de) => {
                return Err(USimpleError::new(1_i32, "error: invalid input".to_owned()));
            }
        };

        let output_len = output.len();

        output.resize(output_len + decode_len_result, 0_u8);

        match self.decode_mut(input, &mut (output[output_len..])) {
            Ok(us) => {
                // See:
                // https://docs.rs/data-encoding/latest/data_encoding/struct.Encoding.html#method.decode_mut
                // "Returns the length of the decoded output. This length may be smaller than the output length if the input contained padding or ignored characters. The output bytes after the returned length are not initialized and should not be read."
                output.truncate(output_len + us);
            }
            Err(_de) => {
                return Err(USimpleError::new(1_i32, "error: invalid input".to_owned()));
            }
        }

        Ok(())
    }
}
