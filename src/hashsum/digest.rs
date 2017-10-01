extern crate crypto;
extern crate rustc_serialize;

use crypto::md5::Md5;
use crypto::sha1::Sha1;
use crypto::sha2::{Sha224, Sha256, Sha384, Sha512};
use crypto::sha3::Sha3;
use crypto::digest::Digest as CryptoDigest;

pub trait Digest {
    fn input(&mut self, input: &[u8]);
    fn result(&mut self, out: &mut [u8]);
    fn reset(&mut self);
    fn output_bits(&self) -> usize;
    fn output_bytes(&self) -> usize {
        (self.output_bits() + 7) / 8
    }
    fn result_str(&mut self) -> String {
        use self::rustc_serialize::hex::ToHex;

        let mut buf: Vec<u8> = vec![0; self.output_bytes()];
        self.result(&mut buf);
        buf.to_hex()
    }
}

impl Digest for Md5 {
    fn input(&mut self, input: &[u8]) {
        CryptoDigest::input(self, input)
    }

    fn result(&mut self, out: &mut [u8]) {
        CryptoDigest::result(self, out)
    }

    fn reset(&mut self) {
        CryptoDigest::reset(self)
    }

    fn output_bits(&self) -> usize { CryptoDigest::output_bits(self) }
}

impl Digest for Sha1 {
    fn input(&mut self, input: &[u8]) {
        CryptoDigest::input(self, input)
    }

    fn result(&mut self, out: &mut [u8]) {
        CryptoDigest::result(self, out)
    }

    fn reset(&mut self) {
        CryptoDigest::reset(self)
    }

    fn output_bits(&self) -> usize { CryptoDigest::output_bits(self) }
}

impl Digest for Sha224 {
    fn input(&mut self, input: &[u8]) {
        CryptoDigest::input(self, input)
    }

    fn result(&mut self, out: &mut [u8]) {
        CryptoDigest::result(self, out)
    }

    fn reset(&mut self) {
        CryptoDigest::reset(self)
    }

    fn output_bits(&self) -> usize { CryptoDigest::output_bits(self) }
}

impl Digest for Sha256 {
    fn input(&mut self, input: &[u8]) {
        CryptoDigest::input(self, input)
    }

    fn result(&mut self, out: &mut [u8]) {
        CryptoDigest::result(self, out)
    }

    fn reset(&mut self) {
        CryptoDigest::reset(self)
    }

    fn output_bits(&self) -> usize { CryptoDigest::output_bits(self) }
}

impl Digest for Sha384 {
    fn input(&mut self, input: &[u8]) {
        CryptoDigest::input(self, input)
    }

    fn result(&mut self, out: &mut [u8]) {
        CryptoDigest::result(self, out)
    }

    fn reset(&mut self) {
        CryptoDigest::reset(self)
    }

    fn output_bits(&self) -> usize { CryptoDigest::output_bits(self) }
}

impl Digest for Sha512 {
    fn input(&mut self, input: &[u8]) {
        CryptoDigest::input(self, input)
    }

    fn result(&mut self, out: &mut [u8]) {
        CryptoDigest::result(self, out)
    }

    fn reset(&mut self) {
        CryptoDigest::reset(self)
    }

    fn output_bits(&self) -> usize { CryptoDigest::output_bits(self) }
}

impl Digest for Sha3 {
    fn input(&mut self, input: &[u8]) {
        CryptoDigest::input(self, input)
    }

    fn result(&mut self, out: &mut [u8]) {
        CryptoDigest::result(self, out)
    }

    fn reset(&mut self) {
        CryptoDigest::reset(self)
    }

    fn output_bits(&self) -> usize { CryptoDigest::output_bits(self) }
}
