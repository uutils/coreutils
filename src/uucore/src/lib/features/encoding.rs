// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (encodings) lsbf msbf
// spell-checker:ignore unpadded
// spell-checker:ignore ABCDEFGHJKLMNPQRSTUVWXY Zabcdefghijkmnopqrstuvwxyz

use crate::error::{UResult, USimpleError};
use base64_simd;
use data_encoding::Encoding;
use data_encoding_macro::new_encoding;
use std::collections::VecDeque;

// SIMD base64 wrapper
pub struct Base64SimdWrapper {
    pub alphabet: &'static [u8],
    pub use_padding: bool,
    pub unpadded_multiple: usize,
    pub valid_decoding_multiple: usize,
}

impl Base64SimdWrapper {
    pub fn new(
        use_padding: bool,
        valid_decoding_multiple: usize,
        unpadded_multiple: usize,
        alphabet: &'static [u8],
    ) -> Self {
        assert!(valid_decoding_multiple > 0);
        assert!(unpadded_multiple > 0);
        assert!(!alphabet.is_empty());

        Self {
            alphabet,
            use_padding,
            unpadded_multiple,
            valid_decoding_multiple,
        }
    }
}

impl SupportsFastDecodeAndEncode for Base64SimdWrapper {
    fn alphabet(&self) -> &'static [u8] {
        self.alphabet
    }

    fn decode_into_vec(&self, input: &[u8], output: &mut Vec<u8>) -> UResult<()> {
        let decoded = if self.use_padding {
            base64_simd::STANDARD.decode_to_vec(input)
        } else {
            base64_simd::STANDARD_NO_PAD.decode_to_vec(input)
        };

        match decoded {
            Ok(decoded_bytes) => {
                output.extend_from_slice(&decoded_bytes);
                Ok(())
            }
            Err(_) => {
                // Restore original length on error
                output.truncate(output.len());
                Err(USimpleError::new(1, "error: invalid input".to_owned()))
            }
        }
    }

    fn encode_to_vec_deque(&self, input: &[u8], output: &mut VecDeque<u8>) -> UResult<()> {
        let encoded = if self.use_padding {
            base64_simd::STANDARD.encode_to_string(input)
        } else {
            base64_simd::STANDARD_NO_PAD.encode_to_string(input)
        };

        output.extend(encoded.as_bytes());

        Ok(())
    }

    fn unpadded_multiple(&self) -> usize {
        self.unpadded_multiple
    }

    fn valid_decoding_multiple(&self) -> usize {
        self.valid_decoding_multiple
    }
}

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
    Base58,
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

pub struct Base58Wrapper {}

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

impl SupportsFastDecodeAndEncode for Base58Wrapper {
    fn alphabet(&self) -> &'static [u8] {
        // Base58 alphabet
        b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
    }

    fn decode_into_vec(&self, input: &[u8], output: &mut Vec<u8>) -> UResult<()> {
        if input.is_empty() {
            return Ok(());
        }

        // Count leading zeros (will become leading 1s in base58)
        let leading_ones = input.iter().take_while(|&&b| b == b'1').count();

        // Skip leading 1s for conversion
        let input_trimmed = &input[leading_ones..];
        if input_trimmed.is_empty() {
            output.resize(output.len() + leading_ones, 0);
            return Ok(());
        }

        // Convert base58 to big integer
        let mut num: Vec<u32> = vec![0];
        let alphabet = self.alphabet();

        for &byte in input_trimmed {
            // Find position in alphabet
            let digit = alphabet
                .iter()
                .position(|&b| b == byte)
                .ok_or_else(|| USimpleError::new(1, "error: invalid input".to_owned()))?;

            // Multiply by 58 and add digit
            let mut carry = digit as u32;
            for n in &mut num {
                let tmp = (*n as u64) * 58 + carry as u64;
                *n = tmp as u32;
                carry = (tmp >> 32) as u32;
            }
            if carry > 0 {
                num.push(carry);
            }
        }

        // Convert to bytes (little endian, then reverse)
        let mut result = Vec::new();
        for &n in &num {
            result.extend_from_slice(&n.to_le_bytes());
        }

        // Remove trailing zeros and reverse to get big endian
        while result.last() == Some(&0) && result.len() > 1 {
            result.pop();
        }
        result.reverse();

        // Add leading zeros for leading 1s in input
        let mut final_result = vec![0; leading_ones];
        final_result.extend_from_slice(&result);

        output.extend_from_slice(&final_result);
        Ok(())
    }

    fn encode_to_vec_deque(&self, input: &[u8], output: &mut VecDeque<u8>) -> UResult<()> {
        if input.is_empty() {
            return Ok(());
        }

        // Count leading zeros
        let leading_zeros = input.iter().take_while(|&&b| b == 0).count();

        // Skip leading zeros
        let input_trimmed = &input[leading_zeros..];
        if input_trimmed.is_empty() {
            for _ in 0..leading_zeros {
                output.push_back(b'1');
            }
            return Ok(());
        }

        // Convert bytes to big integer
        let mut num: Vec<u32> = Vec::new();
        for &byte in input_trimmed {
            let mut carry = byte as u32;
            for n in &mut num {
                let tmp = (*n as u64) * 256 + carry as u64;
                *n = tmp as u32;
                carry = (tmp >> 32) as u32;
            }
            if carry > 0 {
                num.push(carry);
            }
        }

        // Convert to base58
        let mut result = Vec::new();
        let alphabet = self.alphabet();

        while !num.is_empty() && num.iter().any(|&n| n != 0) {
            let mut carry = 0u64;
            for n in num.iter_mut().rev() {
                let tmp = carry * (1u64 << 32) + *n as u64;
                *n = (tmp / 58) as u32;
                carry = tmp % 58;
            }
            result.push(alphabet[carry as usize]);

            // Remove leading zeros
            while num.last() == Some(&0) && num.len() > 1 {
                num.pop();
            }
        }

        // Add leading 1s for leading zeros in input
        for _ in 0..leading_zeros {
            output.push_back(b'1');
        }

        // Add result (reversed because we built it backwards)
        for byte in result.into_iter().rev() {
            output.push_back(byte);
        }

        Ok(())
    }

    fn unpadded_multiple(&self) -> usize {
        1 // Base58 doesn't use padding
    }

    fn valid_decoding_multiple(&self) -> usize {
        1 // Any length is valid for Base58
    }
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
