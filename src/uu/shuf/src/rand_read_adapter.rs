// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// Copyright 2018 Developers of the Rand project.
// Copyright 2013 The Rust Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A wrapper around any Read to treat it as an RNG.

use std::cell::Cell;
use std::io::{Error, Read};
use std::rc::Rc;

use rand_core::{RngCore, impls};

/// An RNG that reads random bytes straight from any type supporting
/// [`std::io::Read`], for example files.
///
/// This will work best with an infinite reader, but that is not required.
///
/// This can be used with `/dev/urandom` on Unix but it is recommended to use
/// [`OsRng`] instead.
///
/// # Panics
///
/// `ReadRng` uses [`std::io::Read::read_exact`], which retries on interrupts.
/// All other errors from the underlying reader, including when it does not
/// have enough data, will be reported via the public error field (which can
/// be cloned in advance, as it uses [`Rc`]). This field must be checked for
/// errors after every operation.
///
/// [`OsRng`]: rand::rngs::OsRng
pub struct ReadRng<R> {
    reader: R,
    pub error: ErrorCell,
}

pub type ErrorCell = Rc<Cell<Option<std::io::Error>>>;

impl<R: Read> ReadRng<R> {
    /// Create a new `ReadRng` from a `Read`.
    pub fn new(r: R) -> Self {
        Self {
            reader: r,
            error: Rc::default(),
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        if dest.is_empty() {
            return Ok(());
        }
        // Use `std::io::read_exact`, which retries on `ErrorKind::Interrupted`.
        self.reader.read_exact(dest)
    }
}

impl<R: Read> RngCore for ReadRng<R> {
    fn next_u32(&mut self) -> u32 {
        impls::next_u32_via_fill(self)
    }

    fn next_u64(&mut self) -> u64 {
        impls::next_u64_via_fill(self)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        if let Err(err) = self.try_fill_bytes(dest) {
            // Failed to deliver random data, so the caller must check the error
            // cell before using the result.
            self.error.set(Some(err));
        }
    }
}

#[cfg(test)]
mod test {
    use std::println;

    use super::ReadRng;
    use rand::RngCore;

    #[test]
    fn test_reader_rng_u64() {
        // transmute from the target to avoid endianness concerns.
        #[rustfmt::skip]
        let v = [0u8, 0, 0, 0, 0, 0, 0, 1,
                 0,   4, 0, 0, 3, 0, 0, 2,
                 5,   0, 0, 0, 0, 0, 0, 0];
        let mut rng = ReadRng::new(&v[..]);

        assert_eq!(rng.next_u64(), 1 << 56);
        assert_eq!(rng.next_u64(), (2 << 56) + (3 << 32) + (4 << 8));
        assert_eq!(rng.next_u64(), 5);
    }

    #[test]
    fn test_reader_rng_u32() {
        let v = [0u8, 0, 0, 1, 0, 0, 2, 0, 3, 0, 0, 0];
        let mut rng = ReadRng::new(&v[..]);

        assert_eq!(rng.next_u32(), 1 << 24);
        assert_eq!(rng.next_u32(), 2 << 16);
        assert_eq!(rng.next_u32(), 3);
    }

    #[test]
    fn test_reader_rng_fill_bytes() {
        let v = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let mut w = [0u8; 8];

        let mut rng = ReadRng::new(&v[..]);
        rng.fill_bytes(&mut w);

        assert_eq!(v, w);
    }

    #[test]
    fn test_reader_rng_insufficient_bytes() {
        let v = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let mut w = [0u8; 9];

        let mut rng = ReadRng::new(&v[..]);

        let result = rng.try_fill_bytes(&mut w);
        assert!(result.is_err());
        println!("Error: {}", result.unwrap_err());
    }
}
