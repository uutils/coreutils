extern crate digest;
extern crate md5;
extern crate sha1;
extern crate sha2;
extern crate sha3;

use digest::digest::{ExtendableOutput, Input, XofReader};
use hex::ToHex;

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
        buf.to_hex()
    }
}

impl Digest for md5::Context {
    fn new() -> Self {
        md5::Context::new()
    }

    fn input(&mut self, input: &[u8]) {
        self.consume(input)
    }

    fn result(&mut self, out: &mut [u8]) {
        out.copy_from_slice(&*self.compute());
    }

    fn reset(&mut self) {
        *self = md5::Context::new();
    }

    fn output_bits(&self) -> usize {
        128
    }
}

impl Digest for sha1::Sha1 {
    fn new() -> Self {
        sha1::Sha1::new()
    }

    fn input(&mut self, input: &[u8]) {
        self.update(input);
    }

    fn result(&mut self, out: &mut [u8]) {
        out.copy_from_slice(&self.digest().bytes());
    }

    fn reset(&mut self) {
        self.reset();
    }

    fn output_bits(&self) -> usize {
        160
    }
}

// Implements the Digest trait for sha2 / sha3 algorithms with fixed ouput
macro_rules! impl_digest_sha {
    ($type: ty, $size: expr) => (
        impl Digest for $type {
            fn new() -> Self {
                Self::default()
            }

            fn input(&mut self, input: &[u8]) {
                digest::Digest::input(self, input);
            }

            fn result(&mut self, out: &mut [u8]) {
                out.copy_from_slice(digest::Digest::result(*self).as_slice());
            }

            fn reset(&mut self) {
                *self = Self::new();
            }

            fn output_bits(&self) -> usize { $size }
        }
    )
}

// Implements the Digest trait for sha2 / sha3 algorithms with variable ouput
macro_rules! impl_digest_shake {
    ($type: ty) => (
        impl Digest for $type {
            fn new() -> Self {
                Self::default()
            }

            fn input(&mut self, input: &[u8]) {
                self.process(input);
            }

            fn result(&mut self, out: &mut [u8]) {
                self.xof_result().read(out);
            }

            fn reset(&mut self) {
                *self = Self::new();
            }

            fn output_bits(&self) -> usize { 0 }
        }
    )
}

impl_digest_sha!(sha2::Sha224, 224);
impl_digest_sha!(sha2::Sha256, 256);
impl_digest_sha!(sha2::Sha384, 384);
impl_digest_sha!(sha2::Sha512, 512);

impl_digest_sha!(sha3::Sha3_224, 224);
impl_digest_sha!(sha3::Sha3_256, 256);
impl_digest_sha!(sha3::Sha3_384, 384);
impl_digest_sha!(sha3::Sha3_512, 512);
impl_digest_shake!(sha3::Shake128);
impl_digest_shake!(sha3::Shake256);
