// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (encodings) lsbf msbf
// spell-checker:ignore unpadded
// spell-checker:ignore ABCDEFGHJKLMNPQRSTUVWXY Zabcdefghijkmnopqrstuvwxyz

use crate::error::{UResult, USimpleError};
use crate::translate;
use base64_simd;
use data_encoding::Encoding;
use data_encoding_macro::new_encoding;
use num_bigint::BigUint;
use std::collections::VecDeque;

// SIMD base64 wrapper
pub struct Base64SimdWrapper {
    pub alphabet: &'static [u8],
    pub use_padding: bool,
    pub unpadded_multiple: usize,
    pub valid_decoding_multiple: usize,
}

impl Base64SimdWrapper {
    fn engine(&self, padded: bool) -> base64_simd::Base64 {
        // The URL safe alphabet is the only one with '-' in it.
        let url_safe = self.alphabet.contains(&b'-');
        match (url_safe, padded) {
            (false, true) => base64_simd::STANDARD,
            (false, false) => base64_simd::STANDARD_NO_PAD,
            (true, true) => base64_simd::URL_SAFE,
            (true, false) => base64_simd::URL_SAFE_NO_PAD,
        }
    }

    fn decode_padded(&self, input: &[u8], output: &mut Vec<u8>) -> Result<(), ()> {
        let decoded_bytes = self.engine(true).decode_to_vec(input).map_err(|_| ())?;
        output.extend_from_slice(&decoded_bytes);
        Ok(())
    }

    fn decode_unpadded(&self, input: &[u8], output: &mut Vec<u8>) -> Result<(), ()> {
        let decoded_bytes = self.engine(false).decode_to_vec(input).map_err(|_| ())?;
        output.extend_from_slice(&decoded_bytes);
        Ok(())
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
                        return Err(USimpleError::new(
                            1,
                            translate!("encoding-error-invalid-input"),
                        ));
                    }

                    self.decode_padded(&remaining[..segment_len], output)
                        .map_err(|_| {
                            USimpleError::new(1, translate!("encoding-error-invalid-input"))
                        })?;

                    start += segment_len;
                } else {
                    // If there are no more '=' bytes the tail might still be padded
                    // (len % 4 == 0) or purposely unpadded (GNU --ignore-garbage or
                    // concatenated streams), so select the matching alphabet.
                    let decoded = if remaining.len().is_multiple_of(4) {
                        self.decode_padded(remaining, output)
                    } else {
                        self.decode_unpadded(remaining, output)
                    };

                    decoded.map_err(|_| {
                        USimpleError::new(1, translate!("encoding-error-invalid-input"))
                    })?;

                    break;
                }
            }

            Ok(())
        } else {
            self.decode_unpadded(input, output)
                .map_err(|_| USimpleError::new(1, translate!("encoding-error-invalid-input")))
        };

        if let Err(err) = decode_result {
            output.truncate(original_len);
            Err(err)
        } else {
            Ok(())
        }
    }

    fn encode_to_vec_deque(&self, input: &[u8], output: &mut VecDeque<u8>) -> UResult<()> {
        let encoded = self.engine(self.use_padding).encode_to_string(input);

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
    /// `"VGhlIHF1aWNrIGJyb3du"`
    ///
    /// While:
    ///
    /// "The quick brown fox"
    ///
    /// is 19 characters, which is not divisible by 3, so its Base64 representation has padding:
    ///
    /// `"VGhlIHF1aWNrIGJyb3duIGZveA=="`
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

/// Turn big-endian base58 digit values into a number.
///
/// Converting digit by digit costs O(n²). Splitting the digits in two halves,
/// `high * 58^len(low) + low`, lets the big-number multiplication do the heavy
/// lifting, which is much faster on large inputs.
fn base58_digits_to_number(digits: &[u8]) -> BigUint {
    const SPLIT_THRESHOLD: usize = 256;
    if digits.len() <= SPLIT_THRESHOLD {
        // Every digit is below 58, so this cannot fail.
        return BigUint::from_radix_be(digits, 58).unwrap_or_default();
    }
    let (high, low) = digits.split_at(digits.len() / 2);
    // A chunk long enough to overflow u32 could never fit in memory as a BigUint.
    let low_len = low.len() as u32;
    base58_digits_to_number(high) * BigUint::from(58u32).pow(low_len) + base58_digits_to_number(low)
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

        let alphabet = self.alphabet();
        let digits = input_trimmed
            .iter()
            .map(|byte| {
                alphabet
                    .iter()
                    .position(|b| b == byte)
                    .map(|digit| digit as u8)
                    .ok_or_else(|| USimpleError::new(1, translate!("encoding-error-invalid-input")))
            })
            .collect::<UResult<Vec<u8>>>()?;

        output.resize(output.len() + leading_ones, 0);
        output.extend_from_slice(&base58_digits_to_number(&digits).to_bytes_be());
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

        let alphabet = self.alphabet();
        let digits = BigUint::from_bytes_be(input_trimmed).to_radix_be(58);

        // Add leading 1s for leading zeros in input
        for _ in 0..leading_zeros {
            output.push_back(b'1');
        }
        output.extend(digits.into_iter().map(|digit| alphabet[usize::from(digit)]));

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
            return Err(USimpleError::new(
                1,
                translate!("encoding-error-invalid-input"),
            ));
        }

        let decode_result = z85::decode(input)
            .map_err(|_de| USimpleError::new(1, translate!("encoding-error-invalid-input")))?;
        output.extend_from_slice(&decode_result);

        Ok(())
    }

    fn valid_decoding_multiple(&self) -> usize {
        5
    }

    fn encode_to_vec_deque(&self, input: &[u8], output: &mut VecDeque<u8>) -> UResult<()> {
        // According to the spec we should not accept inputs whose len is not a multiple of 4.
        // However, the z85 crate implements a padded encoding and accepts such inputs. We have to manually check for them.
        if !input.len().is_multiple_of(4) {
            return Err(USimpleError::new(
                1,
                translate!("encoding-error-invalid-input-z85-length"),
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
        let decode_len_result = self
            .encoding
            .decode_len(input.len())
            .map_err(|_de| USimpleError::new(1, translate!("encoding-error-invalid-input")))?;

        let output_len = output.len();

        output.resize(output_len + decode_len_result, 0);

        let us = self
            .encoding
            .decode_mut(input, &mut (output[output_len..]))
            .map_err(|_de| USimpleError::new(1, translate!("encoding-error-invalid-input")))?;
        // See:
        // https://docs.rs/data-encoding/latest/data_encoding/struct.Encoding.html#method.decode_mut
        // "Returns the length of the decoded output. This length may be smaller than the output length if the input contained padding or ignored characters. The output bytes after the returned length are not initialized and should not be read."
        output.truncate(output_len + us);

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
        const VALID_REMAINDERS: [usize; 4] = [2, 4, 5, 7];

        if remainder.is_empty() || remainder.contains(&b'=') {
            return None;
        }

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

#[cfg(test)]
mod tests {
    use super::base58_digits_to_number;
    use num_bigint::BigUint;

    #[test]
    fn base58_split_conversion_matches_direct_conversion() {
        // Runs of zero digits make some halves start with zeros, which the
        // split must still weigh correctly.
        let digits: Vec<u8> = (0..3001u32)
            .map(|i| {
                if i % 700 < 300 {
                    0
                } else {
                    ((i * 29 + 5) % 58) as u8
                }
            })
            .collect();
        for len in [1, 256, 257, 1000, 3001] {
            let slice = &digits[digits.len() - len..];
            assert_eq!(
                base58_digits_to_number(slice),
                BigUint::from_radix_be(slice, 58).unwrap(),
                "length {len}"
            );
        }
    }
}
