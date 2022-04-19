// spell-checker:ignore memmem
//! Implementations of digest functions, like md5 and sha1.
//!
//! The [`Digest`] trait represents the interface for providing inputs
//! to these digest functions and accessing the resulting hash. The
//! [`DigestWriter`] struct provides a wrapper around [`Digest`] that
//! implements the [`Write`] trait, for use in situations where calling
//! [`write`] would be useful.
extern crate digest;
extern crate md5;
extern crate sha1;
extern crate sha2;
extern crate sha3;

use std::io::Write;

use hex::encode;
#[cfg(windows)]
use memchr::memmem;

pub trait Digest {
    fn new() -> Self
    where
        Self: Sized;
    fn input(&mut self, input: &[u8]);
    fn result(&mut self, out: &mut [u8]);
    fn reset(&mut self);
    fn output_bits(&self) -> usize;
    fn output_bytes(&self) -> usize {
        (self.output_bits() + 7) / 8
    }
    fn result_str(&mut self) -> String {
        let mut buf: Vec<u8> = vec![0; self.output_bytes()];
        self.result(&mut buf);
        encode(buf)
    }
}

impl Digest for blake2b_simd::State {
    fn new() -> Self {
        Self::new()
    }

    fn input(&mut self, input: &[u8]) {
        self.update(input);
    }

    fn result(&mut self, out: &mut [u8]) {
        let hash_result = &self.finalize();
        out.copy_from_slice(hash_result.as_bytes());
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn output_bits(&self) -> usize {
        512
    }
}

impl Digest for blake3::Hasher {
    fn new() -> Self {
        Self::new()
    }

    fn input(&mut self, input: &[u8]) {
        self.update(input);
    }

    fn result(&mut self, out: &mut [u8]) {
        let hash_result = &self.finalize();
        out.copy_from_slice(hash_result.as_bytes());
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn output_bits(&self) -> usize {
        256
    }
}

// Implements the Digest trait for sha2 / sha3 algorithms with fixed output
macro_rules! impl_digest_common {
    ($type: ty, $size: expr) => {
        impl Digest for $type {
            fn new() -> Self {
                Self::default()
            }

            fn input(&mut self, input: &[u8]) {
                digest::Digest::update(self, input);
            }

            fn result(&mut self, out: &mut [u8]) {
                digest::Digest::finalize_into_reset(self, out.into());
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
    ($type: ty) => {
        impl Digest for $type {
            fn new() -> Self {
                Self::default()
            }

            fn input(&mut self, input: &[u8]) {
                digest::Update::update(self, input);
            }

            fn result(&mut self, out: &mut [u8]) {
                digest::ExtendableOutputReset::finalize_xof_reset_into(self, out);
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

impl_digest_common!(md5::Md5, 128);
impl_digest_common!(sha1::Sha1, 160);
impl_digest_common!(sha2::Sha224, 224);
impl_digest_common!(sha2::Sha256, 256);
impl_digest_common!(sha2::Sha384, 384);
impl_digest_common!(sha2::Sha512, 512);

impl_digest_common!(sha3::Sha3_224, 224);
impl_digest_common!(sha3::Sha3_256, 256);
impl_digest_common!(sha3::Sha3_384, 384);
impl_digest_common!(sha3::Sha3_512, 512);
impl_digest_shake!(sha3::Shake128);
impl_digest_shake!(sha3::Shake256);

/// A struct that writes to a digest.
///
/// This struct wraps a [`Digest`] and provides a [`Write`]
/// implementation that passes input bytes directly to the
/// [`Digest::input`].
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
    pub fn new(digest: &'a mut Box<dyn Digest>, binary: bool) -> DigestWriter {
        let was_last_character_carriage_return = false;
        DigestWriter {
            digest,
            binary,
            was_last_character_carriage_return,
        }
    }

    pub fn finalize(&mut self) -> bool {
        if self.was_last_character_carriage_return {
            self.digest.input(&[b'\r']);
            true
        } else {
            false
        }
    }
}

impl<'a> Write for DigestWriter<'a> {
    #[cfg(not(windows))]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.digest.input(buf);
        Ok(buf.len())
    }

    #[cfg(windows)]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.binary {
            self.digest.input(buf);
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
            self.digest.input(&[b'\r']);
        }

        // Next, find all occurrences of "\r\n", inputting the slice
        // just before the "\n" in the previous instance of "\r\n" and
        // the beginning of this "\r\n".
        let mut i_prev = 0;
        for i in memmem::find_iter(buf, b"\r\n") {
            self.digest.input(&buf[i_prev..i]);
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
            self.digest.input(&buf[i_prev..n - 1]);
        } else {
            self.was_last_character_carriage_return = false;
            self.digest.input(&buf[i_prev..n]);
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

        use crate::digest::Digest;
        use crate::digest::DigestWriter;

        // Writing "\r" in one call to `write()`, and then "\n" in another.
        let mut digest = Box::new(md5::Md5::new()) as Box<dyn Digest>;
        let mut writer_crlf = DigestWriter::new(&mut digest, false);
        writer_crlf.write_all(&[b'\r']).unwrap();
        writer_crlf.write_all(&[b'\n']).unwrap();
        writer_crlf.finalize();
        let result_crlf = digest.result_str();

        // We expect "\r\n" to be replaced with "\n" in text mode on Windows.
        let mut digest = Box::new(md5::Md5::new()) as Box<dyn Digest>;
        let mut writer_lf = DigestWriter::new(&mut digest, false);
        writer_lf.write_all(&[b'\n']).unwrap();
        writer_lf.finalize();
        let result_lf = digest.result_str();

        assert_eq!(result_crlf, result_lf);
    }
}
