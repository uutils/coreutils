extern crate digest;
extern crate md5;
extern crate rustc_serialize;
extern crate sha1;
extern crate sha2;
extern crate sha3;

use digest::digest::{Input, ExtendableOutput, XofReader};

pub trait Digest {
    fn new() -> Self where Self: Sized;
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

    fn output_bits(&self) -> usize { 128 }
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

    fn output_bits(&self) -> usize { 160 }
}

impl Digest for sha2::Sha224 {
    fn new() -> Self {
        sha2::Sha224::default()
    }

    fn input(&mut self, input: &[u8]) {
        digest::Digest::input(self, input);
    }

    fn result(&mut self, out: &mut [u8]) {
        out.copy_from_slice(digest::Digest::result(*self).as_slice());
    }

    fn reset(&mut self) {
        *self = sha2::Sha224::default();
    }

    fn output_bits(&self) -> usize { 224 }
}

impl Digest for sha2::Sha256 {
    fn new() -> Self {
        sha2::Sha256::default()
    }

    fn input(&mut self, input: &[u8]) {
        digest::Digest::input(self, input);
    }

    fn result(&mut self, out: &mut [u8]) {
        out.copy_from_slice(digest::Digest::result(*self).as_slice());
    }

    fn reset(&mut self) {
        *self = sha2::Sha256::default();
    }

    fn output_bits(&self) -> usize { 256 }
}

impl Digest for sha2::Sha384 {
    fn new() -> Self {
        sha2::Sha384::default()
    }

    fn input(&mut self, input: &[u8]) {
        digest::Digest::input(self, input)
    }

    fn result(&mut self, out: &mut [u8]) {
        out.copy_from_slice(digest::Digest::result(*self).as_slice());
    }

    fn reset(&mut self) {
        *self = sha2::Sha384::default();
    }

    fn output_bits(&self) -> usize { 384 }
}

impl Digest for sha2::Sha512 {
    fn new() -> Self {
        sha2::Sha512::default()
    }

    fn input(&mut self, input: &[u8]) {
        digest::Digest::input(self, input)
    }

    fn result(&mut self, out: &mut [u8]) {
        out.copy_from_slice(digest::Digest::result(*self).as_slice());
    }

    fn reset(&mut self) {
        *self = sha2::Sha512::default();
    }

    fn output_bits(&self) -> usize { 512 }
}

impl Digest for sha3::Sha3_224 {
    fn new() -> Self {
        Self::default()
    }

    fn input(&mut self, input: &[u8]) {
        digest::Digest::input(self, input)
    }

    fn result(&mut self, out: &mut [u8]) {
        out.copy_from_slice(digest::Digest::result(*self).as_slice());
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn output_bits(&self) -> usize { 224 }
}

impl Digest for sha3::Sha3_256 {
    fn new() -> Self {
        sha3::Sha3_256::default()
    }

    fn input(&mut self, input: &[u8]) {
        digest::Digest::input(self, input)
    }

    fn result(&mut self, out: &mut [u8]) {
        out.copy_from_slice(digest::Digest::result(*self).as_slice());
    }

    fn reset(&mut self) {
        *self = sha3::Sha3_256::default();
    }

    fn output_bits(&self) -> usize { 256 }
}

impl Digest for sha3::Sha3_384 {
    fn new() -> Self {
        sha3::Sha3_384::default()
    }

    fn input(&mut self, input: &[u8]) {
        digest::Digest::input(self, input)
    }

    fn result(&mut self, out: &mut [u8]) {
        out.copy_from_slice(digest::Digest::result(*self).as_slice());
    }

    fn reset(&mut self) {
        *self = sha3::Sha3_384::default();
    }

    fn output_bits(&self) -> usize { 384 }
}

impl Digest for sha3::Sha3_512 {
    fn new() -> Self {
        sha3::Sha3_512::default()
    }

    fn input(&mut self, input: &[u8]) {
        digest::Digest::input(self, input)
    }

    fn result(&mut self, out: &mut [u8]) {
        out.copy_from_slice(digest::Digest::result(*self).as_slice());
    }

    fn reset(&mut self) {
        *self = sha3::Sha3_512::default();
    }

    fn output_bits(&self) -> usize { 512 }
}

impl Digest for sha3::Shake128 {
    fn new() -> Self {
        sha3::Shake128::default()
    }

    fn input(&mut self, input: &[u8]) {
        self.process(input);
    }

    fn result(&mut self, out: &mut [u8]) {
        self.xof_result().read(out);
    }

    fn reset(&mut self) {
        *self = sha3::Shake128::default();
    }

    fn output_bits(&self) -> usize { 0 }
}

impl Digest for sha3::Shake256 {
    fn new() -> Self {
        sha3::Shake256::default()
    }

    fn input(&mut self, input: &[u8]) {
        self.process(input);
    }

    fn result(&mut self, out: &mut [u8]) {
        self.xof_result().read(out);
    }

    fn reset(&mut self) {
        *self = sha3::Shake256::default();
    }

    fn output_bits(&self) -> usize { 0 }
}
