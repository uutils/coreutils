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
    fn decode_with_standard(input: &[u8], output: &mut Vec<u8>) -> Result<(), ()> {
        match base64_simd::STANDARD.decode_to_vec(input) {
            Ok(decoded_bytes) => {
                output.extend_from_slice(&decoded_bytes);
                Ok(())
            }
            Err(_) => Err(()),
        }
    }

    fn decode_with_no_pad(input: &[u8], output: &mut Vec<u8>) -> Result<(), ()> {
        match base64_simd::STANDARD_NO_PAD.decode_to_vec(input) {
            Ok(decoded_bytes) => {
                output.extend_from_slice(&decoded_bytes);
                Ok(())
            }
            Err(_) => Err(()),
        }
    }

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
        let original_len = output.len();

        let decode_result = if self.use_padding {
            // GNU coreutils keeps decoding even when '=' appears before the true end
            // of the stream (e.g. concatenated padded chunks). Mirror that logic
            // by splitting at each '='-containing quantum, decoding those 4-byte
            // groups with the padded variant, then letting the remainder fall back
            // to whichever alphabet fits.
            let mut start = 0usize;
            while start < input.len() {
                let remaining = &input[start..];

                if remaining.is_empty() {
                    break;
                }

                if let Some(eq_rel_idx) = remaining.iter().position(|&b| b == b'=') {
                    let blocks = (eq_rel_idx / 4) + 1;
                    let segment_len = blocks * 4;

                    if segment_len > remaining.len() {
                        return Err(USimpleError::new(1, "error: invalid input".to_owned()));
                    }

                    if Self::decode_with_standard(&remaining[..segment_len], output).is_err() {
                        return Err(USimpleError::new(1, "error: invalid input".to_owned()));
                    }

                    start += segment_len;
                } else {
                    // If there are no more '=' bytes the tail might still be padded
                    // (len % 4 == 0) or purposely unpadded (GNU --ignore-garbage or
                    // concatenated streams), so select the matching alphabet.
                    let decoder = if remaining.len() % 4 == 0 {
                        Self::decode_with_standard
                    } else {
                        Self::decode_with_no_pad
                    };

                    if decoder(remaining, output).is_err() {
                        return Err(USimpleError::new(1, "error: invalid input".to_owned()));
                    }

                    break;
                }
            }

            Ok(())
        } else {
            Self::decode_with_no_pad(input, output)
                .map_err(|_| USimpleError::new(1, "error: invalid input".to_owned()))
        };

        if let Err(err) = decode_result {
            output.truncate(original_len);
            Err(err)
        } else {
            Ok(())
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

pub struct PadResult {
    pub chunk: Vec<u8>,
    pub had_invalid_tail: bool,
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

    /// Whether the decoder can flush partial chunks (multiples of `valid_decoding_multiple`)
    /// before seeing the full input. Defaults to `false` for encodings that must consume the
    /// entire input (e.g. base58).
    fn supports_partial_decode(&self) -> bool {
        false
    }

    /// Gives encoding-specific logic a chance to pad a trailing, non-empty remainder
    /// before the final decode attempt. The default implementation opts out.
    fn pad_remainder(&self, _remainder: &[u8]) -> Option<PadResult> {
        None
    }
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

        // Convert bytes to big integer (Vec<u32> in little-endian format)
        let mut num = Vec::with_capacity(input_trimmed.len().div_ceil(4) + 1);
        for &byte in input_trimmed {
            let mut carry = byte as u64;
            for n in &mut num {
                let tmp = (*n as u64) * 256 + carry;
                *n = tmp as u32;
                carry = tmp >> 32;
            }
            if carry > 0 {
                num.push(carry as u32);
            }
        }

        // Convert to base58
        let mut result = Vec::with_capacity((input_trimmed.len() * 138 / 100) + 1);
        let alphabet = self.alphabet();

        // Optimized check: stop when all elements are zero
        while !num.is_empty() {
            // Check if we're done (all zeros)
            let mut all_zero = true;
            let mut carry = 0u64;

            for n in num.iter_mut().rev() {
                let tmp = carry * (1u64 << 32) + *n as u64;
                *n = (tmp / 58) as u32;
                carry = tmp % 58;
                if *n != 0 {
                    all_zero = false;
                }
            }

            result.push(alphabet[carry as usize]);

            if all_zero {
                break;
            }

            // Trim trailing zeros less frequently
            if num.len() > 1 && result.len() % 8 == 0 {
                while num.last() == Some(&0) && num.len() > 1 {
                    num.pop();
                }
            }
        }

        // Add leading 1s for leading zeros in input
        for _ in 0..leading_zeros {
            output.push_back(b'1');
        }

        // Add result (reversed because we built it backwards)
        for &byte in result.iter().rev() {
            output.push_back(byte);
        }

        Ok(())
    }

    fn unpadded_multiple(&self) -> usize {
        // Base58 must encode the entire input as one big integer, not in chunks
        // Use a very large value to effectively disable chunking, but avoid overflow
        // when multiplied by ENCODE_IN_CHUNKS_OF_SIZE_MULTIPLE (1024) in base_common
        usize::MAX / 2048
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

pub struct Base32Wrapper {
    inner: EncodingWrapper,
}

impl Base32Wrapper {
    pub fn new(
        encoding: Encoding,
        valid_decoding_multiple: usize,
        unpadded_multiple: usize,
        alphabet: &'static [u8],
    ) -> Self {
        Self {
            inner: EncodingWrapper::new(
                encoding,
                valid_decoding_multiple,
                unpadded_multiple,
                alphabet,
            ),
        }
    }
}

impl SupportsFastDecodeAndEncode for Base32Wrapper {
    fn alphabet(&self) -> &'static [u8] {
        self.inner.alphabet()
    }

    fn decode_into_vec(&self, input: &[u8], output: &mut Vec<u8>) -> UResult<()> {
        self.inner.decode_into_vec(input, output)
    }

    fn encode_to_vec_deque(&self, input: &[u8], output: &mut VecDeque<u8>) -> UResult<()> {
        self.inner.encode_to_vec_deque(input, output)
    }

    fn unpadded_multiple(&self) -> usize {
        self.inner.unpadded_multiple()
    }

    fn valid_decoding_multiple(&self) -> usize {
        self.inner.valid_decoding_multiple()
    }

    fn pad_remainder(&self, remainder: &[u8]) -> Option<PadResult> {
        if remainder.is_empty() || remainder.contains(&b'=') {
            return None;
        }

        const VALID_REMAINDERS: [usize; 4] = [2, 4, 5, 7];

        let mut len = remainder.len();
        let mut trimmed = false;

        while len > 0 && !VALID_REMAINDERS.contains(&len) {
            len -= 1;
            trimmed = true;
        }

        if len == 0 {
            return None;
        }

        let mut padded = remainder[..len].to_vec();
        let missing = self.valid_decoding_multiple() - padded.len();
        padded.extend(std::iter::repeat_n(b'=', missing));

        Some(PadResult {
            chunk: padded,
            had_invalid_tail: trimmed,
        })
    }

    fn supports_partial_decode(&self) -> bool {
        true
    }
}
