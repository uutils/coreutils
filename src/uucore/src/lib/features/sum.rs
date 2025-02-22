// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore memmem algo

//! Implementations of digest functions, like md5 and sha1.
//!
//! The [`Digest`] trait represents the interface for providing inputs
//! to these digest functions and accessing the resulting hash. The
//! [`DigestWriter`] struct provides a wrapper around [`Digest`] that
//! implements the [`Write`] trait, for use in situations where calling
//! [`write`] would be useful.
use std::io::Write;

use hex::encode;
#[cfg(windows)]
use memchr::memmem;

pub trait Digest {
    fn new() -> Self
    where
        Self: Sized;
    fn hash_update(&mut self, input: &[u8]);
    fn hash_finalize(&mut self, out: &mut [u8]);
    fn reset(&mut self);
    fn output_bits(&self) -> usize;
    fn output_bytes(&self) -> usize {
        self.output_bits().div_ceil(8)
    }
    fn result_str(&mut self) -> String {
        let mut buf: Vec<u8> = vec![0; self.output_bytes()];
        self.hash_finalize(&mut buf);
        encode(buf)
    }
}

/// first element of the tuple is the blake2b state
/// second is the number of output bits
pub struct Blake2b(blake2b_simd::State, usize);

impl Blake2b {
    /// Return a new Blake2b instance with a custom output bytes length
    pub fn with_output_bytes(output_bytes: usize) -> Self {
        let mut params = blake2b_simd::Params::new();
        params.hash_length(output_bytes);

        let state = params.to_state();
        Self(state, output_bytes * 8)
    }
}

impl Digest for Blake2b {
    fn new() -> Self {
        // by default, Blake2b output is 512 bits long (= 64B)
        Self::with_output_bytes(64)
    }

    fn hash_update(&mut self, input: &[u8]) {
        self.0.update(input);
    }

    fn hash_finalize(&mut self, out: &mut [u8]) {
        let hash_result = &self.0.finalize();
        out.copy_from_slice(hash_result.as_bytes());
    }

    fn reset(&mut self) {
        *self = Self::with_output_bytes(self.output_bytes());
    }

    fn output_bits(&self) -> usize {
        self.1
    }
}

pub struct Blake3(blake3::Hasher);
impl Digest for Blake3 {
    fn new() -> Self {
        Self(blake3::Hasher::new())
    }

    fn hash_update(&mut self, input: &[u8]) {
        self.0.update(input);
    }

    fn hash_finalize(&mut self, out: &mut [u8]) {
        let hash_result = &self.0.finalize();
        out.copy_from_slice(hash_result.as_bytes());
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn output_bits(&self) -> usize {
        256
    }
}

pub struct Sm3(sm3::Sm3);
impl Digest for Sm3 {
    fn new() -> Self {
        Self(<sm3::Sm3 as sm3::Digest>::new())
    }

    fn hash_update(&mut self, input: &[u8]) {
        <sm3::Sm3 as sm3::Digest>::update(&mut self.0, input);
    }

    fn hash_finalize(&mut self, out: &mut [u8]) {
        out.copy_from_slice(&<sm3::Sm3 as sm3::Digest>::finalize(self.0.clone()));
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn output_bits(&self) -> usize {
        256
    }
}

// NOTE: CRC_TABLE_LEN *must* be <= 256 as we cast 0..CRC_TABLE_LEN to u8
const CRC_TABLE_LEN: usize = 256;

pub struct CRC {
    state: u32,
    size: usize,
    crc_table: [u32; CRC_TABLE_LEN],
}
impl CRC {
    fn generate_crc_table() -> [u32; CRC_TABLE_LEN] {
        let mut table = [0; CRC_TABLE_LEN];

        for (i, elt) in table.iter_mut().enumerate().take(CRC_TABLE_LEN) {
            *elt = Self::crc_entry(i as u8);
        }

        table
    }
    fn crc_entry(input: u8) -> u32 {
        let mut crc = (input as u32) << 24;

        let mut i = 0;
        while i < 8 {
            let if_condition = crc & 0x8000_0000;
            let if_body = (crc << 1) ^ 0x04c1_1db7;
            let else_body = crc << 1;

            // NOTE: i feel like this is easier to understand than emulating an if statement in bitwise
            //       ops
            let condition_table = [else_body, if_body];

            crc = condition_table[(if_condition != 0) as usize];
            i += 1;
        }

        crc
    }

    fn update(&mut self, input: u8) {
        self.state = (self.state << 8)
            ^ self.crc_table[((self.state >> 24) as usize ^ input as usize) & 0xFF];
    }
}

impl Digest for CRC {
    fn new() -> Self {
        Self {
            state: 0,
            size: 0,
            crc_table: Self::generate_crc_table(),
        }
    }

    fn hash_update(&mut self, input: &[u8]) {
        for &elt in input {
            self.update(elt);
        }
        self.size += input.len();
    }

    fn hash_finalize(&mut self, out: &mut [u8]) {
        let mut sz = self.size;
        while sz != 0 {
            self.update(sz as u8);
            sz >>= 8;
        }
        self.state = !self.state;
        out.copy_from_slice(&self.state.to_ne_bytes());
    }

    fn result_str(&mut self) -> String {
        let mut _out: Vec<u8> = vec![0; 4];
        self.hash_finalize(&mut _out);
        format!("{}", self.state)
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn output_bits(&self) -> usize {
        256
    }
}

pub struct CRC32B(crc32fast::Hasher);
impl Digest for CRC32B {
    fn new() -> Self {
        Self(crc32fast::Hasher::new())
    }

    fn hash_update(&mut self, input: &[u8]) {
        self.0.update(input);
    }

    fn hash_finalize(&mut self, out: &mut [u8]) {
        let result = self.0.clone().finalize();
        let slice = result.to_be_bytes();
        out.copy_from_slice(&slice);
    }

    fn reset(&mut self) {
        self.0.reset();
    }

    fn output_bits(&self) -> usize {
        32
    }

    fn result_str(&mut self) -> String {
        let mut out = [0; 4];
        self.hash_finalize(&mut out);
        format!("{}", u32::from_be_bytes(out))
    }
}

pub struct BSD {
    state: u16,
}
impl Digest for BSD {
    fn new() -> Self {
        Self { state: 0 }
    }

    fn hash_update(&mut self, input: &[u8]) {
        for &byte in input {
            self.state = (self.state >> 1) + ((self.state & 1) << 15);
            self.state = self.state.wrapping_add(u16::from(byte));
        }
    }

    fn hash_finalize(&mut self, out: &mut [u8]) {
        out.copy_from_slice(&self.state.to_ne_bytes());
    }

    fn result_str(&mut self) -> String {
        let mut _out: Vec<u8> = vec![0; 2];
        self.hash_finalize(&mut _out);
        format!("{}", self.state)
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn output_bits(&self) -> usize {
        128
    }
}

pub struct SYSV {
    state: u32,
}
impl Digest for SYSV {
    fn new() -> Self {
        Self { state: 0 }
    }

    fn hash_update(&mut self, input: &[u8]) {
        for &byte in input {
            self.state = self.state.wrapping_add(u32::from(byte));
        }
    }

    fn hash_finalize(&mut self, out: &mut [u8]) {
        self.state = (self.state & 0xffff) + (self.state >> 16);
        self.state = (self.state & 0xffff) + (self.state >> 16);
        out.copy_from_slice(&(self.state as u16).to_ne_bytes());
    }

    fn result_str(&mut self) -> String {
        let mut _out: Vec<u8> = vec![0; 2];
        self.hash_finalize(&mut _out);
        format!("{}", self.state)
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn output_bits(&self) -> usize {
        512
    }
}

// Implements the Digest trait for sha2 / sha3 algorithms with fixed output
macro_rules! impl_digest_common {
    ($algo_type: ty, $size: expr) => {
        impl Digest for $algo_type {
            fn new() -> Self {
                Self(Default::default())
            }

            fn hash_update(&mut self, input: &[u8]) {
                digest::Digest::update(&mut self.0, input);
            }

            fn hash_finalize(&mut self, out: &mut [u8]) {
                digest::Digest::finalize_into_reset(&mut self.0, out.into());
            }

            fn reset(&mut self) {
                *self = Self::new();
            }

            fn output_bits(&self) -> usize {
                $size
            }
        }
    };
}

// Implements the Digest trait for sha2 / sha3 algorithms with variable output
macro_rules! impl_digest_shake {
    ($algo_type: ty) => {
        impl Digest for $algo_type {
            fn new() -> Self {
                Self(Default::default())
            }

            fn hash_update(&mut self, input: &[u8]) {
                digest::Update::update(&mut self.0, input);
            }

            fn hash_finalize(&mut self, out: &mut [u8]) {
                digest::ExtendableOutputReset::finalize_xof_reset_into(&mut self.0, out);
            }

            fn reset(&mut self) {
                *self = Self::new();
            }

            fn output_bits(&self) -> usize {
                0
            }
        }
    };
}

pub struct Md5(md5::Md5);
pub struct Sha1(sha1::Sha1);
pub struct Sha224(sha2::Sha224);
pub struct Sha256(sha2::Sha256);
pub struct Sha384(sha2::Sha384);
pub struct Sha512(sha2::Sha512);
impl_digest_common!(Md5, 128);
impl_digest_common!(Sha1, 160);
impl_digest_common!(Sha224, 224);
impl_digest_common!(Sha256, 256);
impl_digest_common!(Sha384, 384);
impl_digest_common!(Sha512, 512);

pub struct Sha3_224(sha3::Sha3_224);
pub struct Sha3_256(sha3::Sha3_256);
pub struct Sha3_384(sha3::Sha3_384);
pub struct Sha3_512(sha3::Sha3_512);
impl_digest_common!(Sha3_224, 224);
impl_digest_common!(Sha3_256, 256);
impl_digest_common!(Sha3_384, 384);
impl_digest_common!(Sha3_512, 512);

pub struct Shake128(sha3::Shake128);
pub struct Shake256(sha3::Shake256);
impl_digest_shake!(Shake128);
impl_digest_shake!(Shake256);

/// A struct that writes to a digest.
///
/// This struct wraps a [`Digest`] and provides a [`Write`]
/// implementation that passes input bytes directly to the
/// [`Digest::hash_update`].
///
/// On Windows, if `binary` is `false`, then the [`write`]
/// implementation replaces instances of "\r\n" with "\n" before passing
/// the input bytes to the [`digest`].
pub struct DigestWriter<'a> {
    digest: &'a mut Box<dyn Digest>,

    /// Whether to write to the digest in binary mode or text mode on Windows.
    ///
    /// If this is `false`, then instances of "\r\n" are replaced with
    /// "\n" before passing input bytes to the [`digest`].
    #[allow(dead_code)]
    binary: bool,

    /// Whether the previous
    #[allow(dead_code)]
    was_last_character_carriage_return: bool,
    // TODO These are dead code only on non-Windows operating systems.
    // It might be better to use a `#[cfg(windows)]` guard here.
}

impl<'a> DigestWriter<'a> {
    pub fn new(digest: &'a mut Box<dyn Digest>, binary: bool) -> Self {
        let was_last_character_carriage_return = false;
        DigestWriter {
            digest,
            binary,
            was_last_character_carriage_return,
        }
    }

    pub fn finalize(&mut self) -> bool {
        if self.was_last_character_carriage_return {
            self.digest.hash_update(b"\r");
            true
        } else {
            false
        }
    }
}

impl Write for DigestWriter<'_> {
    #[cfg(not(windows))]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.digest.hash_update(buf);
        Ok(buf.len())
    }

    #[cfg(windows)]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.binary {
            self.digest.hash_update(buf);
            return Ok(buf.len());
        }

        // The remaining code handles Windows text mode, where we must
        // replace each occurrence of "\r\n" with "\n".
        //
        // First, if the last character written was "\r" and the first
        // character in the current buffer to write is not "\n", then we
        // need to write the "\r" that we buffered from the previous
        // call to `write()`.
        let n = buf.len();
        if self.was_last_character_carriage_return && n > 0 && buf[0] != b'\n' {
            self.digest.hash_update(b"\r");
        }

        // Next, find all occurrences of "\r\n", inputting the slice
        // just before the "\n" in the previous instance of "\r\n" and
        // the beginning of this "\r\n".
        let mut i_prev = 0;
        for i in memmem::find_iter(buf, b"\r\n") {
            self.digest.hash_update(&buf[i_prev..i]);
            i_prev = i + 1;
        }

        // Finally, check whether the last character is "\r". If so,
        // buffer it until we know that the next character is not "\n",
        // which can only be known on the next call to `write()`.
        //
        // This all assumes that `write()` will be called on adjacent
        // blocks of the input.
        if n > 0 && buf[n - 1] == b'\r' {
            self.was_last_character_carriage_return = true;
            self.digest.hash_update(&buf[i_prev..n - 1]);
        } else {
            self.was_last_character_carriage_return = false;
            self.digest.hash_update(&buf[i_prev..n]);
        }

        // Even though we dropped a "\r" for each "\r\n" we found, we
        // still report the number of bytes written as `n`. This is
        // because the meaning of the returned number is supposed to be
        // the number of bytes consumed by the writer, so that if the
        // calling code were calling `write()` in a loop, it would know
        // where the next contiguous slice of the buffer starts.
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    /// Test for replacing a "\r\n" sequence with "\n" when the "\r" is
    /// at the end of one block and the "\n" is at the beginning of the
    /// next block, when reading in blocks.
    #[cfg(windows)]
    #[test]
    fn test_crlf_across_blocks() {
        use std::io::Write;

        use super::Digest;
        use super::DigestWriter;
        use super::Md5;

        // Writing "\r" in one call to `write()`, and then "\n" in another.
        let mut digest = Box::new(Md5::new()) as Box<dyn Digest>;
        let mut writer_crlf = DigestWriter::new(&mut digest, false);
        writer_crlf.write_all(b"\r").unwrap();
        writer_crlf.write_all(b"\n").unwrap();
        writer_crlf.finalize();
        let result_crlf = digest.result_str();

        // We expect "\r\n" to be replaced with "\n" in text mode on Windows.
        let mut digest = Box::new(Md5::new()) as Box<dyn Digest>;
        let mut writer_lf = DigestWriter::new(&mut digest, false);
        writer_lf.write_all(b"\n").unwrap();
        writer_lf.finalize();
        let result_lf = digest.result_str();

        assert_eq!(result_crlf, result_lf);
    }
}
