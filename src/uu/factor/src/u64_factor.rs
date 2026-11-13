// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore REDC Jaeschke Sinclair cofactor Carmichael primorial funcs splitmix

//! Factorization of `u64` values, without heap allocation.
//!
//! The pipeline is the classic one:
//!
//! 1. trial division by the small primes of [`SMALL_PRIMES`], using a
//!    multiplication by a modular inverse instead of a division;
//! 2. a deterministic Miller-Rabin test on what is left;
//! 3. Pollard's rho (Brent's variant) to split composites.
//!
//! Steps 2 and 3 work in Montgomery form, see [`Montgomery`].

/// A `u64` has at most 15 distinct prime factors: the product of the first 16
/// primes does not fit.
const MAX_DISTINCT_FACTORS: usize = 15;

/// The prime factorization of a `u64`, sorted by increasing prime.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Factors {
    primes: [u64; MAX_DISTINCT_FACTORS],
    exponents: [u8; MAX_DISTINCT_FACTORS],
    len: usize,
}

impl Factors {
    /// Record `exponent` more occurrences of `prime`, keeping the primes sorted.
    fn push(&mut self, prime: u64, exponent: u8) {
        let mut at = self.len;
        while at > 0 && self.primes[at - 1] > prime {
            at -= 1;
        }
        if at > 0 && self.primes[at - 1] == prime {
            self.exponents[at - 1] += exponent;
            return;
        }
        self.primes.copy_within(at..self.len, at + 1);
        self.exponents.copy_within(at..self.len, at + 1);
        self.primes[at] = prime;
        self.exponents[at] = exponent;
        self.len += 1;
    }

    /// The `(prime, exponent)` pairs, by increasing prime.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (u64, usize)> + '_ {
        self.primes[..self.len]
            .iter()
            .zip(&self.exponents[..self.len])
            .map(|(&prime, &exponent)| (prime, usize::from(exponent)))
    }
}

/// Trial division covers the primes below this bound.
const SMALL_PRIME_BOUND: usize = 1 << 12;

/// An odd prime `p` together with what is needed to test divisibility by it
/// with one multiplication: `n` is a multiple of `p` exactly when
/// `n * inverse (mod 2^64) <= limit`, and that product is then `n / p`.
#[derive(Clone, Copy)]
struct SmallPrime {
    prime: u64,
    /// The inverse of `prime` modulo 2^64.
    inverse: u64,
    /// `u64::MAX / prime`
    limit: u64,
}

/// `sieve[i]` tells whether `i` is composite, for `i >= 2`.
const fn composite_sieve() -> [bool; SMALL_PRIME_BOUND] {
    let mut composite = [false; SMALL_PRIME_BOUND];
    let mut i = 2;
    while i * i < SMALL_PRIME_BOUND {
        if !composite[i] {
            let mut multiple = i * i;
            while multiple < SMALL_PRIME_BOUND {
                composite[multiple] = true;
                multiple += i;
            }
        }
        i += 1;
    }
    composite
}

const fn count_odd_primes() -> usize {
    let composite = composite_sieve();
    let mut count = 0;
    let mut i = 3;
    while i < SMALL_PRIME_BOUND {
        if !composite[i] {
            count += 1;
        }
        i += 2;
    }
    count
}

/// The inverse of the odd number `n` modulo 2^64.
const fn inverse_mod_2_64(n: u64) -> u64 {
    // `n * n = 1 (mod 8)`, so `n` is its own inverse on the low 3 bits. Each
    // Newton step doubles the number of correct bits: 3, 6, 12, 24, 48, 96.
    let mut inverse = n;
    let mut step = 0;
    while step < 5 {
        inverse = inverse.wrapping_mul(2u64.wrapping_sub(n.wrapping_mul(inverse)));
        step += 1;
    }
    inverse
}

const SMALL_PRIME_COUNT: usize = count_odd_primes();

/// The odd primes below [`SMALL_PRIME_BOUND`].
static SMALL_PRIMES: [SmallPrime; SMALL_PRIME_COUNT] = {
    let composite = composite_sieve();
    let mut table = [SmallPrime {
        prime: 0,
        inverse: 0,
        limit: 0,
    }; SMALL_PRIME_COUNT];
    let mut len = 0;
    let mut i = 3;
    while i < SMALL_PRIME_BOUND {
        if !composite[i] {
            let prime = i as u64;
            table[len] = SmallPrime {
                prime,
                inverse: inverse_mod_2_64(prime),
                limit: u64::MAX / prime,
            };
            len += 1;
        }
        i += 2;
    }
    table
};

/// Numbers below this bound with no factor in [`SMALL_PRIMES`] are prime.
const PRIME_BELOW: u64 = (SMALL_PRIME_BOUND as u64) * (SMALL_PRIME_BOUND as u64);

/// Factorize `n`. Zero and one have no prime factors.
pub(crate) fn factorize(mut n: u64) -> Factors {
    let mut factors = Factors::default();
    if n < 2 {
        return factors;
    }

    let twos = n.trailing_zeros();
    if twos > 0 {
        factors.push(2, twos as u8);
        n >>= twos;
    }

    for small in &SMALL_PRIMES {
        if small.prime * small.prime > n {
            // No prime up to the square root of `n` divides it.
            if n > 1 {
                factors.push(n, 1);
            }
            return factors;
        }
        let mut quotient = n.wrapping_mul(small.inverse);
        if quotient <= small.limit {
            let mut exponent = 0;
            while quotient <= small.limit {
                n = quotient;
                exponent += 1;
                quotient = n.wrapping_mul(small.inverse);
            }
            factors.push(small.prime, exponent);
        }
    }

    if n == 1 {
        return factors;
    }
    if n < PRIME_BELOW {
        factors.push(n, 1);
    } else {
        factorize_large(n, &mut factors);
    }
    factors
}

/// Factorize an odd `n` that has no factor in [`SMALL_PRIMES`].
///
/// The recursion is shallow: every prime factor is above 2^12, so there are
/// at most five of them.
fn factorize_large(n: u64, factors: &mut Factors) {
    let modulus = Montgomery::new(n);
    if modulus.is_prime() {
        factors.push(n, 1);
        return;
    }
    let divisor = modulus.find_divisor();
    factorize_large(divisor, factors);
    factorize_large(n / divisor, factors);
}

/// Arithmetic modulo an odd `n`, on values kept in Montgomery form.
///
/// The Montgomery form of `a` is `a * 2^64 mod n`. Multiplying two such values
/// and reducing with [`Montgomery::mul`] needs no division by `n`, which is
/// what makes modular exponentiation and Pollard's rho cheap.
struct Montgomery {
    n: u64,
    /// The inverse of `n` modulo 2^64.
    n_inverse: u64,
    /// The Montgomery form of 1, i.e. `2^64 mod n`.
    one: u64,
    /// `2^128 mod n`: multiplying by it converts to Montgomery form.
    r_squared: u64,
}

impl Montgomery {
    /// `n` must be odd and greater than 1.
    fn new(n: u64) -> Self {
        debug_assert!(n % 2 == 1 && n > 1);
        let one = n.wrapping_neg() % n;
        let r_squared = ((u128::from(one) * u128::from(one)) % u128::from(n)) as u64;
        Self {
            n,
            n_inverse: inverse_mod_2_64(n),
            one,
            r_squared,
        }
    }

    /// Montgomery reduction (REDC): `t / 2^64 mod n`, for `t < n * 2^64`.
    #[inline]
    fn reduce(&self, t: u128) -> u64 {
        // `m * n` has the same low half as `t`, so `t - m * n` is a multiple
        // of 2^64 and only the high halves need subtracting.
        let m = (t as u64).wrapping_mul(self.n_inverse);
        let high = (t >> 64) as u64;
        let m_n_high = ((u128::from(m) * u128::from(self.n)) >> 64) as u64;
        let (difference, borrow) = high.overflowing_sub(m_n_high);
        if borrow {
            difference.wrapping_add(self.n)
        } else {
            difference
        }
    }

    /// The Montgomery form of `a * b`, for `a` and `b` in Montgomery form.
    #[inline]
    fn mul(&self, a: u64, b: u64) -> u64 {
        self.reduce(u128::from(a) * u128::from(b))
    }

    fn to_montgomery(&self, a: u64) -> u64 {
        self.mul(a % self.n, self.r_squared)
    }

    fn pow(&self, base: u64, mut exponent: u64) -> u64 {
        let mut result = self.one;
        let mut square = base;
        while exponent > 0 {
            if exponent & 1 == 1 {
                result = self.mul(result, square);
            }
            square = self.mul(square, square);
            exponent >>= 1;
        }
        result
    }

    /// Deterministic Miller-Rabin: the base sets below are proven to leave no
    /// strong pseudoprime in their range (Jaeschke for the first two, Sinclair
    /// for the one covering every `u64`).
    fn is_prime(&self) -> bool {
        const BASES_32: &[u64] = &[2, 7, 61];
        const BASES_40: &[u64] = &[2, 13, 23, 1_662_803];
        const BASES_64: &[u64] = &[2, 325, 9375, 28178, 450_775, 9_780_504, 1_795_265_022];

        let bases = if self.n < 4_759_123_141 {
            BASES_32
        } else if self.n < 1_122_004_669_633 {
            BASES_40
        } else {
            BASES_64
        };

        let shift = (self.n - 1).trailing_zeros();
        let odd_part = (self.n - 1) >> shift;
        let minus_one = self.n - self.one;
        bases
            .iter()
            .all(|&base| self.is_strong_probable_prime(base, odd_part, shift, minus_one))
    }

    /// Whether `n` passes the Miller-Rabin test for `base`, where
    /// `n - 1 = odd_part * 2^shift`.
    fn is_strong_probable_prime(
        &self,
        base: u64,
        odd_part: u64,
        shift: u32,
        minus_one: u64,
    ) -> bool {
        let base = self.to_montgomery(base);
        if base == 0 {
            // A multiple of `n` says nothing about it.
            return true;
        }
        let mut x = self.pow(base, odd_part);
        if x == self.one || x == minus_one {
            return true;
        }
        for _ in 1..shift {
            x = self.mul(x, x);
            if x == minus_one {
                return true;
            }
        }
        false
    }

    /// A proper divisor of the composite `n`, found with Brent's variant of
    /// Pollard's rho.
    fn find_divisor(&self) -> u64 {
        // Differences are multiplied together and their GCD with `n` is taken
        // once per batch rather than once per step.
        const BATCH: u64 = 128;

        let n = self.n;
        if let Some(divisor) = fermat_divisor(n) {
            return divisor;
        }

        let mut increment: u128 = 0;
        loop {
            // A sequence can cycle without exposing a factor; another
            // polynomial `x^2 + increment` then gives a different sequence.
            increment += 1;
            // The increment is added before reducing, which is one modular
            // addition less per step; `x^2 + increment` stays below `n * 2^64`.
            let step = |x: u64| self.reduce(u128::from(x) * u128::from(x) + increment);

            let mut y = 2;
            let mut x;
            let mut batch_start;
            let mut product = self.one;
            let mut divisor;
            let mut round = 1;
            'search: loop {
                x = y;
                for _ in 0..round {
                    y = step(y);
                }
                let mut done = 0;
                loop {
                    batch_start = y;
                    let batch = BATCH.min(round - done);
                    for _ in 0..batch {
                        y = step(y);
                        product = self.mul(product, x.abs_diff(y));
                    }
                    divisor = gcd(product, n);
                    if divisor != 1 {
                        break 'search;
                    }
                    done += batch;
                    if done >= round {
                        break;
                    }
                }
                round *= 2;
            }

            if divisor == n {
                // Several factors showed up in the same batch: replay it one
                // step at a time.
                loop {
                    batch_start = step(batch_start);
                    divisor = gcd(x.abs_diff(batch_start), n);
                    if divisor != 1 {
                        break;
                    }
                }
            }
            if divisor != n {
                return divisor;
            }
        }
    }
}

/// Fermat's method, for a few steps only: look for `a` with `a^2 - n` a
/// square `b^2`, so that `n = (a - b) * (a + b)`.
///
/// This finds the factors of an odd composite `n` when they are close to its
/// square root, squares of primes included. Those are the factors Pollard's
/// rho takes the longest to find.
fn fermat_divisor(n: u64) -> Option<u64> {
    const STEPS: u64 = 4;

    let root = n.isqrt();
    if root * root == n {
        return Some(root);
    }
    (root + 1..=root + STEPS).find_map(|a| {
        // `a^2` can exceed `u64::MAX` when `n` is close to it.
        let square = u128::from(a) * u128::from(a) - u128::from(n);
        let b = (square as u64).isqrt();
        (u128::from(b) * u128::from(b) == square).then(|| a - b)
    })
}

/// The greatest common divisor of `a` and the odd number `b`.
fn gcd(mut a: u64, mut b: u64) -> u64 {
    if a == 0 {
        return b;
    }
    a >>= a.trailing_zeros();
    loop {
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }
        b -= a;
        if b == 0 {
            return a;
        }
        b >>= b.trailing_zeros();
    }
}

#[cfg(test)]
mod tests {
    use super::{Factors, Montgomery, SMALL_PRIMES, factorize, fermat_divisor, gcd};
    use num_prime::nt_funcs::{factorize64, is_prime64};

    fn pairs(factors: &Factors) -> Vec<(u64, usize)> {
        factors.iter().collect()
    }

    /// A small deterministic generator (splitmix64), so failures reproduce.
    fn random_numbers(seed: u64) -> impl Iterator<Item = u64> {
        let mut state = seed;
        std::iter::repeat_with(move || {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        })
    }

    fn assert_matches_reference(n: u64) {
        let expected: Vec<(u64, usize)> = factorize64(n).into_iter().collect();
        assert_eq!(pairs(&factorize(n)), expected, "factors of {n}");
    }

    #[test]
    fn small_prime_table_is_the_odd_primes_below_the_bound() {
        assert_eq!(SMALL_PRIMES.len(), 563);
        assert_eq!(SMALL_PRIMES[0].prime, 3);
        assert_eq!(SMALL_PRIMES[SMALL_PRIMES.len() - 1].prime, 4093);
        for small in &SMALL_PRIMES {
            assert!(is_prime64(small.prime));
            assert_eq!(small.prime.wrapping_mul(small.inverse), 1);
        }
    }

    #[test]
    fn zero_and_one_have_no_factors() {
        assert_eq!(pairs(&factorize(0)), []);
        assert_eq!(pairs(&factorize(1)), []);
    }

    #[test]
    fn every_number_up_to_a_million() {
        const LIMIT: usize = 1_000_000;
        let mut smallest_factor = vec![0; LIMIT + 1];
        for i in 2..=LIMIT {
            if smallest_factor[i] == 0 {
                for multiple in (i..=LIMIT).step_by(i) {
                    if smallest_factor[multiple] == 0 {
                        smallest_factor[multiple] = i;
                    }
                }
            }
        }

        for n in 2..=LIMIT {
            let mut expected: Vec<(u64, usize)> = Vec::new();
            let mut rest = n;
            while rest > 1 {
                let prime = smallest_factor[rest] as u64;
                match expected.last_mut() {
                    Some((last, exponent)) if *last == prime => *exponent += 1,
                    _ => expected.push((prime, 1)),
                }
                rest /= smallest_factor[rest];
            }
            assert_eq!(pairs(&factorize(n as u64)), expected, "factors of {n}");
        }
    }

    #[test]
    fn edge_cases() {
        // The largest prime below 2^64, and its neighbors.
        assert_eq!(
            pairs(&factorize(18_446_744_073_709_551_557)),
            [(18_446_744_073_709_551_557, 1)]
        );
        assert_eq!(pairs(&factorize(1 << 63)), [(2, 63)]);
        assert_eq!(
            pairs(&factorize(u64::MAX)),
            [
                (3, 1),
                (5, 1),
                (17, 1),
                (257, 1),
                (641, 1),
                (65537, 1),
                (6_700_417, 1)
            ]
        );
        // The product of the first 15 primes: as many distinct factors as fit.
        let primorial = 614_889_782_588_491_410;
        assert_eq!(factorize(primorial).iter().count(), 15);
        assert_matches_reference(primorial);
        // The largest square of a prime, and powers of primes past the table.
        assert_eq!(
            pairs(&factorize(4_294_967_291 * 4_294_967_291)),
            [(4_294_967_291, 2)]
        );
        assert_eq!(pairs(&factorize(4099 * 4099)), [(4099, 2)]);
        assert_eq!(pairs(&factorize(4099_u64.pow(5))), [(4099, 5)]);
        assert_eq!(pairs(&factorize(1_000_003_u64.pow(3))), [(1_000_003, 3)]);
        assert_eq!(pairs(&factorize(3_u64.pow(40))), [(3, 40)]);
        // Two primes just below 2^32: the hardest shape for Pollard's rho.
        assert_eq!(
            pairs(&factorize(4_294_967_279 * 4_294_967_291)),
            [(4_294_967_279, 1), (4_294_967_291, 1)]
        );
        for n in u64::MAX - 2000..=u64::MAX {
            assert_matches_reference(n);
        }
    }

    #[test]
    fn strong_pseudoprimes_are_seen_as_composite() {
        // Carmichael numbers and strong pseudoprimes to several small bases,
        // none of which has a factor in the small prime table.
        for n in [
            // The smallest strong pseudoprime to bases 2, 3, 5 and 7.
            3_215_031_751,
            // The smallest one to the first 5, 6, 7 and 9 prime bases.
            2_152_302_898_747,
            3_474_749_660_383,
            341_550_071_728_321,
            3_825_123_056_546_413_051,
        ] {
            assert!(!is_prime64(n));
            if SMALL_PRIMES.iter().all(|small| n % small.prime != 0) {
                assert!(!Montgomery::new(n).is_prime(), "{n} is composite");
            }
            assert_matches_reference(n);
        }
    }

    #[test]
    fn primality_matches_reference_on_numbers_without_small_factors() {
        let candidates = random_numbers(1)
            .flat_map(|n| [n | 1, (n >> 20) | 1, (n >> 30) | 1])
            .filter(|&n| n > 1 << 24 && SMALL_PRIMES.iter().all(|small| n % small.prime != 0))
            .take(20_000);
        for n in candidates {
            assert_eq!(Montgomery::new(n).is_prime(), is_prime64(n), "{n}");
        }
    }

    #[test]
    fn random_numbers_match_reference() {
        for n in random_numbers(2).take(2_000) {
            assert_matches_reference(n);
            assert_matches_reference(n >> 16);
            assert_matches_reference(n >> 32);
        }
    }

    #[test]
    fn semiprimes_match_reference() {
        let mut primes = random_numbers(3)
            .map(|n| (n >> 32) | 1)
            .filter(|&n| is_prime64(n));
        for _ in 0..200 {
            let (p, q) = (primes.next().unwrap(), primes.next().unwrap());
            let expected = if p == q {
                vec![(p, 2)]
            } else {
                vec![(p.min(q), 1), (p.max(q), 1)]
            };
            assert_eq!(pairs(&factorize(p * q)), expected, "factors of {p} * {q}");
        }
    }

    #[test]
    fn fermat_finds_close_factors_only() {
        assert_eq!(fermat_divisor(4099 * 4099), Some(4099));
        assert_eq!(fermat_divisor(4099 * 4111), Some(4099));
        assert_eq!(
            fermat_divisor(4_294_967_279 * 4_294_967_291),
            Some(4_294_967_279)
        );
        // 2^64 - 1 = (2^32 - 1) * (2^32 + 1), with `a^2` past `u64::MAX`.
        assert_eq!(fermat_divisor(u64::MAX), Some(u64::from(u32::MAX)));
        assert_eq!(fermat_divisor(4099 * 1_000_003), None);
    }

    #[test]
    fn gcd_with_an_odd_number() {
        assert_eq!(gcd(0, 9), 9);
        assert_eq!(gcd(12, 9), 3);
        assert_eq!(gcd(9, 9), 9);
        assert_eq!(gcd(1 << 40, 9), 1);
        assert_eq!(gcd(3 * 5 * 7, 5 * 7 * 11), 35);
    }
}
