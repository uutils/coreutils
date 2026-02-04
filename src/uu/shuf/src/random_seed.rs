// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::ops::RangeInclusive;

use rand::{RngCore as _, SeedableRng as _};
use rand_chacha::ChaCha12Rng;
use sha3::{Digest as _, Sha3_256};

/// Reproducible seeded random number generation.
///
/// The behavior should stay the same between releases, so don't change it without
/// a very good reason.
///
/// # How it works
///
/// - Take a Unicode string as the seed.
///
/// - Encode this seed as UTF-8.
///
/// - Take the SHA3-256 hash of the encoded seed.
///
/// - Use that hash as the input for a [`rand_chacha`] ChaCha12 RNG.
///   (We don't touch the nonce, so that's probably zero.)
///
/// - Take 64-bit samples from the RNG.
///
/// - Use Lemire's method to generate uniformly distributed integers and:
///
///   - With --repeat, use these to pick elements from ranges.
///
///   - Without --repeat, use these to do left-to-right modern Fisher-Yates.
///
/// # Why it works like this
///
/// - Unicode string: Greatest common denominator between platforms. Windows doesn't
///   let you pass raw bytes as a CLI argument and that would be bad practice anyway.
///   A decimal or hex number would work but this is much more flexible without being
///   unmanageable.
///
///   (Footgun: if the user passes a filename we won't read from the file but the
///   command will run anyway.)
///
/// - UTF-8: That's what Rust likes and it's the least unreasonable Unicode encoding.
///
/// - SHA3-256: We want to make good use of the entire user input and SHA-3 is
///   state of the art. ChaCha12 takes a 256-bit seed.
///
/// - ChaCha12: [`rand`]'s default rng as of writing. Seems state of the art.
///
/// - 64-bit samples: We could often get away with 32-bit samples but let's keep things
///   simple and only use one width. (There doesn't seem to be much of a performance hit.)
///
/// - Lemire, Fisher-Yates: These are very easy to implement and maintain ourselves.
///   `rand` provides fancier implementations but only promises reproducibility within
///   patch releases: <https://rust-random.github.io/book/crate-reprod.html>
///
///   Strictly speaking even `ChaCha12` is subject to breakage. But since it's a very
///   specific algorithm I assume it's safe in practice.
pub struct SeededRng(Box<ChaCha12Rng>);

impl SeededRng {
    pub fn new(seed: &str) -> Self {
        let mut hasher = Sha3_256::new();
        hasher.update(seed.as_bytes());
        let seed = hasher.finalize();
        let seed = seed.as_slice().try_into().unwrap();
        Self(Box::new(ChaCha12Rng::from_seed(seed)))
    }

    #[allow(clippy::many_single_char_names)] // use original lemire names for easy comparison
    fn generate_at_most(&mut self, at_most: u64) -> u64 {
        if at_most == u64::MAX {
            return self.0.next_u64();
        }

        // https://lemire.me/blog/2019/06/06/nearly-divisionless-random-integer-generation-on-various-systems/
        let s: u64 = at_most + 1;
        let mut x: u64 = self.0.next_u64();
        let mut m: u128 = u128::from(x) * u128::from(s);
        let mut l: u64 = m as u64;
        if l < s {
            let t: u64 = s.wrapping_neg() % s;
            while l < t {
                x = self.0.next_u64();
                m = u128::from(x) * u128::from(s);
                l = m as u64;
            }
        }
        (m >> 64) as u64
    }

    pub fn choose_from_range(&mut self, range: RangeInclusive<u64>) -> u64 {
        let offset = self.generate_at_most(*range.end() - *range.start());
        *range.start() + offset
    }

    pub fn choose_from_slice<T: Copy>(&mut self, vals: &[T]) -> T {
        assert!(!vals.is_empty());
        let idx = self.generate_at_most(vals.len() as u64 - 1) as usize;
        vals[idx]
    }

    pub fn shuffle<'a, T>(&mut self, vals: &'a mut [T], amount: usize) -> &'a mut [T] {
        // Fisher-Yates shuffle.
        let amount = amount.min(vals.len());
        for idx in 0..amount {
            let other_idx = self.generate_at_most((vals.len() - idx - 1) as u64) as usize + idx;
            vals.swap(idx, other_idx);
        }
        &mut vals[..amount]
    }
}
