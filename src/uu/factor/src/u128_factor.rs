// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore REDC BPSW Baillie Selfridge Jacobi cofactor primorial funcs splitmix ilog modpow Mersenne

//! Factorization of `u128` values, without heap allocation.
//!
//! This follows [`crate::u64_factor`] step by step, on wider arithmetic:
//! trial division by the same small primes, a primality test, then Pollard's
//! rho. Whatever fits in a `u64` along the way is handed to the `u64` code,
//! which is faster and whose primality test is deterministic.
//!
//! No small set of Miller-Rabin bases is known to be exact up to 2^128, so
//! primality is decided by the Baillie-PSW test instead. No composite number
//! passing it is known.

use crate::u64_factor::{self, Factors, SMALL_PRIMES};

/// A `u128` has at most 26 distinct prime factors: the product of the first
/// 27 primes does not fit.
pub(crate) type U128Factors = Factors<u128, 26>;

/// Factorize `n`. Zero and one have no prime factors.
pub(crate) fn factorize(mut n: u128) -> U128Factors {
    let mut factors = Factors::new();
    if n < 2 {
        return factors;
    }

    let twos = n.trailing_zeros();
    if twos > 0 {
        factors.push(2, twos as u8);
        n >>= twos;
    }

    for small in &SMALL_PRIMES {
        if u64::try_from(n).is_ok() {
            // The `u64` code does the rest of the trial division faster.
            break;
        }
        // One more Newton step turns the inverse modulo 2^64 into the
        // inverse modulo 2^128.
        let prime = u128::from(small.prime);
        let inverse = u128::from(small.inverse);
        let inverse = inverse.wrapping_mul(2u128.wrapping_sub(prime.wrapping_mul(inverse)));

        let mut exponent = 0;
        while let Some(quotient) = exact_quotient(n, small.prime, inverse) {
            n = quotient;
            exponent += 1;
        }
        if exponent > 0 {
            factors.push(prime, exponent);
        }
    }

    factorize_cofactor(n, &mut factors);
    factors
}

/// `n / prime` if `prime` divides `n`, given the inverse of `prime` modulo
/// 2^128.
#[inline]
fn exact_quotient(n: u128, prime: u64, inverse: u128) -> Option<u128> {
    // `quotient * prime = n (mod 2^128)`, so `quotient` is `n / prime` exactly
    // when that product does not wrap around.
    let quotient = n.wrapping_mul(inverse);
    let prime = u128::from(prime);
    let low = (quotient as u64 as u128) * prime;
    let high = (quotient >> 64) * prime + (low >> 64);
    (high >> 64 == 0).then_some(quotient)
}

/// Factorize an odd `n`, or 1.
///
/// The recursion is shallow: it only goes on while `n` does not fit in a
/// `u64`, and each level splits `n` in two.
fn factorize_cofactor(n: u128, factors: &mut U128Factors) {
    if let Ok(n) = u64::try_from(n) {
        for (prime, exponent) in u64_factor::factorize(n).iter() {
            // An exponent is at most 63 here.
            factors.push(u128::from(prime), exponent as u8);
        }
        return;
    }

    let modulus = Montgomery::new(n);
    if modulus.is_prime() {
        factors.push(n, 1);
        return;
    }
    let divisor = modulus.find_divisor();
    factorize_cofactor(divisor, factors);
    factorize_cofactor(n / divisor, factors);
}

/// The 256-bit product of `a` and `b`, as `(low half, high half)`.
#[inline]
fn mul_wide(a: u128, b: u128) -> (u128, u128) {
    const LOW: u128 = u64::MAX as u128;

    let (a_low, a_high) = (a & LOW, a >> 64);
    let (b_low, b_high) = (b & LOW, b >> 64);
    let low_low = a_low * b_low;
    let low_high = a_low * b_high;
    let high_low = a_high * b_low;
    let high_high = a_high * b_high;

    // The three terms that overlap bits 64..128; their sum is below 2^66.
    let middle = (low_low >> 64) + (low_high & LOW) + (high_low & LOW);
    let low = (low_low & LOW) | (middle << 64);
    let high = high_high + (low_high >> 64) + (high_low >> 64) + (middle >> 64);
    (low, high)
}

/// Arithmetic modulo an odd `n`, on values kept in Montgomery form
/// (`a * 2^128 mod n`). See `u64_factor::Montgomery`.
struct Montgomery {
    n: u128,
    /// The inverse of `n` modulo 2^128.
    n_inverse: u128,
    /// The Montgomery form of 1, i.e. `2^128 mod n`.
    one: u128,
    /// `2^256 mod n`: multiplying by it converts to Montgomery form.
    r_squared: u128,
}

impl Montgomery {
    /// `n` must be odd and greater than 1.
    fn new(n: u128) -> Self {
        debug_assert!(n % 2 == 1 && n > 1);

        // Newton steps double the number of correct bits: 3, 6, ..., 192.
        let mut n_inverse = n;
        for _ in 0..6 {
            n_inverse = n_inverse.wrapping_mul(2u128.wrapping_sub(n.wrapping_mul(n_inverse)));
        }

        let one = n.wrapping_neg() % n;
        let mut modulus = Self {
            n,
            n_inverse,
            one,
            r_squared: 0,
        };
        // Doubling `2^128 mod n` 128 times gives `2^256 mod n`.
        modulus.r_squared = (0..128).fold(one, |power, _| modulus.add(power, power));
        modulus
    }

    /// Montgomery reduction (REDC): `t / 2^128 mod n`, for a `t` below
    /// `n * 2^128` given as `(low half, high half)`.
    #[inline]
    fn reduce(&self, (low, high): (u128, u128)) -> u128 {
        // `m * n` has the same low half as `t`, so only the high halves need
        // subtracting.
        let m = low.wrapping_mul(self.n_inverse);
        let (_, m_n_high) = mul_wide(m, self.n);
        let (difference, borrow) = high.overflowing_sub(m_n_high);
        if borrow {
            difference.wrapping_add(self.n)
        } else {
            difference
        }
    }

    /// The Montgomery form of `a * b`, for `a` and `b` in Montgomery form.
    #[inline]
    fn mul(&self, a: u128, b: u128) -> u128 {
        self.reduce(mul_wide(a, b))
    }

    #[inline]
    fn add(&self, a: u128, b: u128) -> u128 {
        let (sum, carry) = a.overflowing_add(b);
        if carry || sum >= self.n {
            sum.wrapping_sub(self.n)
        } else {
            sum
        }
    }

    #[inline]
    fn sub(&self, a: u128, b: u128) -> u128 {
        let (difference, borrow) = a.overflowing_sub(b);
        if borrow {
            difference.wrapping_add(self.n)
        } else {
            difference
        }
    }

    /// `a / 2 mod n`
    #[inline]
    fn halve(&self, a: u128) -> u128 {
        if a % 2 == 0 {
            a / 2
        } else {
            // `(a + n) / 2`, without overflowing: both are odd.
            a / 2 + self.n / 2 + 1
        }
    }

    /// The Montgomery form of a small integer, which may be negative.
    fn small_to_montgomery(&self, a: i64) -> u128 {
        let magnitude = self.mul(u128::from(a.unsigned_abs()) % self.n, self.r_squared);
        if a < 0 {
            self.sub(0, magnitude)
        } else {
            magnitude
        }
    }

    fn pow(&self, base: u128, mut exponent: u128) -> u128 {
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

    /// The Baillie-PSW test: a Miller-Rabin test to base 2 followed by a
    /// strong Lucas test.
    fn is_prime(&self) -> bool {
        self.is_strong_probable_prime_base_2() && self.is_strong_lucas_probable_prime()
    }

    fn is_strong_probable_prime_base_2(&self) -> bool {
        let shift = (self.n - 1).trailing_zeros();
        let minus_one = self.n - self.one;
        let mut x = self.pow(self.add(self.one, self.one), (self.n - 1) >> shift);
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

    /// The strong Lucas probable prime test, with Selfridge's choice of
    /// parameters: `D` is the first of 5, -7, 9, -11, ... whose Jacobi symbol
    /// over `n` is -1, `P = 1` and `Q = (1 - D) / 4`.
    ///
    /// With `n + 1 = d * 2^s` and `d` odd, `n` passes when `U_d = 0` or
    /// `V_(d * 2^r) = 0` for some `r < s`, where `U` and `V` are the Lucas
    /// sequences of `P` and `Q` modulo `n`.
    fn is_strong_lucas_probable_prime(&self) -> bool {
        let n = self.n;
        // A square has no `D` with Jacobi symbol -1. `n + 1` must not wrap.
        let root = n.isqrt();
        if root * root == n || n == u128::MAX {
            return false;
        }

        let mut discriminant: i64 = 5;
        loop {
            let magnitude = u128::from(discriminant.unsigned_abs());
            let residue = if discriminant < 0 {
                n - magnitude
            } else {
                magnitude
            };
            match jacobi(residue, n) {
                -1 => break,
                // `D` shares a factor with `n`, which is far larger than `D`.
                0 => return false,
                _ if discriminant > 0 => discriminant = -(discriminant + 2),
                _ => discriminant = 2 - discriminant,
            }
        }
        let q = self.small_to_montgomery((1 - discriminant) / 4);
        let discriminant = self.small_to_montgomery(discriminant);

        let shift = (n + 1).trailing_zeros();
        let odd_part = (n + 1) >> shift;

        // Walk the bits of `odd_part` from the top, keeping `U_k`, `V_k` and
        // `Q^k`. Doubling `k`:
        //   U_2k = U_k * V_k        V_2k = V_k^2 - 2 * Q^k
        // and adding one to `k`, with `P = 1`:
        //   U_(k+1) = (U_k + V_k) / 2        V_(k+1) = (D * U_k + V_k) / 2
        let (mut lucas_u, mut lucas_v, mut q_power) = (self.one, self.one, q);
        for bit in (0..odd_part.ilog2()).rev() {
            lucas_u = self.mul(lucas_u, lucas_v);
            lucas_v = self.sub(self.mul(lucas_v, lucas_v), self.add(q_power, q_power));
            q_power = self.mul(q_power, q_power);
            if (odd_part >> bit) & 1 == 1 {
                let next_u = self.halve(self.add(lucas_u, lucas_v));
                lucas_v = self.halve(self.add(self.mul(discriminant, lucas_u), lucas_v));
                lucas_u = next_u;
                q_power = self.mul(q_power, q);
            }
        }

        if lucas_u == 0 || lucas_v == 0 {
            return true;
        }
        for _ in 1..shift {
            lucas_v = self.sub(self.mul(lucas_v, lucas_v), self.add(q_power, q_power));
            if lucas_v == 0 {
                return true;
            }
            q_power = self.mul(q_power, q_power);
        }
        false
    }

    /// A proper divisor of the composite `n`, found with Brent's variant of
    /// Pollard's rho. See `u64_factor::Montgomery::find_divisor`.
    fn find_divisor(&self) -> u128 {
        const BATCH: u64 = 256;

        let n = self.n;
        if let Some(divisor) = fermat_divisor(n) {
            return divisor;
        }

        let mut increment: u128 = 0;
        loop {
            increment += 1;
            // `x^2 + increment` stays below `n * 2^128`: its high half is at
            // most `n - 2` before the carry.
            let step = |x: u128| {
                let (low, high) = mul_wide(x, x);
                let (low, carry) = low.overflowing_add(increment);
                self.reduce((low, high + u128::from(carry)))
            };

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

/// The Jacobi symbol of `a` over the odd number `n`.
fn jacobi(mut a: u128, mut n: u128) -> i32 {
    let mut sign = 1;
    a %= n;
    while a != 0 {
        let twos = a.trailing_zeros();
        a >>= twos;
        // 2 is a quadratic non-residue modulo `n = 3 or 5 (mod 8)`.
        if twos % 2 == 1 && matches!(n % 8, 3 | 5) {
            sign = -sign;
        }
        // Quadratic reciprocity: swapping two numbers that are both
        // `3 (mod 4)` changes the sign.
        if a % 4 == 3 && n % 4 == 3 {
            sign = -sign;
        }
        std::mem::swap(&mut a, &mut n);
        a %= n;
    }
    if n == 1 { sign } else { 0 }
}

/// Fermat's method, for a few steps only. See `u64_factor::fermat_divisor`.
fn fermat_divisor(n: u128) -> Option<u128> {
    const STEPS: u128 = 4;

    let root = n.isqrt();
    if root * root == n {
        return Some(root);
    }
    (root + 1..=root + STEPS).find_map(|a| {
        // `a^2` does not fit when `n` is close to `u128::MAX`.
        let square = a.checked_mul(a)? - n;
        let b = square.isqrt();
        (b * b == square).then(|| a - b)
    })
}

/// The greatest common divisor of `a` and the odd number `b`.
fn gcd(mut a: u128, mut b: u128) -> u128 {
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
    use super::{Montgomery, U128Factors, factorize, fermat_divisor, gcd, jacobi, mul_wide};
    use num_bigint::BigUint;
    use num_prime::nt_funcs::{factorize128, is_prime};
    use num_prime::{Primality, PrimalityTestConfig};

    fn pairs(factors: &U128Factors) -> Vec<(u128, usize)> {
        factors.iter().collect()
    }

    /// A small deterministic generator (splitmix64), so failures reproduce.
    fn random_numbers(seed: u64) -> impl Iterator<Item = u128> {
        let mut state = seed;
        let mut next = move || {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        std::iter::repeat_with(move || (u128::from(next()) << 64) | u128::from(next()))
    }

    fn reference_is_prime(n: u128) -> bool {
        let config = Some(PrimalityTestConfig::bpsw());
        !matches!(is_prime(&BigUint::from(n), config), Primality::No)
    }

    fn next_prime(from: u128) -> u128 {
        (from..=u128::MAX).find(|&n| reference_is_prime(n)).unwrap()
    }

    fn assert_matches_reference(n: u128) {
        let expected: Vec<(u128, usize)> = factorize128(n).into_iter().collect();
        assert_eq!(pairs(&factorize(n)), expected, "factors of {n}");
    }

    fn assert_factors_multiply_back(n: u128) {
        let factors = factorize(n);
        let product = factors.iter().fold(1u128, |product, (prime, exponent)| {
            product * prime.pow(exponent as u32)
        });
        assert_eq!(product, n, "product of the factors of {n}");
        for (prime, _) in factors.iter() {
            assert!(reference_is_prime(prime), "{prime} is prime");
        }
    }

    #[test]
    fn mul_wide_matches_big_integers() {
        let extremes = [
            0,
            1,
            u128::from(u64::MAX),
            1 << 64,
            u128::MAX - 1,
            u128::MAX,
        ];
        let operands: Vec<u128> = extremes
            .into_iter()
            .chain(random_numbers(1).take(200))
            .collect();
        for &a in &operands {
            for &b in &operands {
                let (low, high) = mul_wide(a, b);
                let expected = BigUint::from(a) * BigUint::from(b);
                let actual = (BigUint::from(high) << 128) + BigUint::from(low);
                assert_eq!(actual, expected, "{a} * {b}");
            }
        }
    }

    #[test]
    fn montgomery_arithmetic_matches_big_integers() {
        let moduli = [3, (1 << 64) + 13, (1 << 127) - 1, u128::MAX, u128::MAX - 2]
            .into_iter()
            .chain(random_numbers(2).map(|n| n | 1).take(50));
        for n in moduli {
            let modulus = Montgomery::new(n);
            let big_n = BigUint::from(n);
            let to_montgomery = |a: u128| modulus.mul(a % n, modulus.r_squared);
            let from_montgomery = |a: u128| modulus.reduce((a, 0));

            assert_eq!(from_montgomery(modulus.one), 1 % n);
            let operands = [0, 1, 2, n - 1, n / 2]
                .into_iter()
                .chain(random_numbers(n as u64).map(|a| a % n).take(20));
            for a in operands {
                let b = (a ^ (n >> 1)) % n;
                let (big_a, big_b) = (BigUint::from(a), BigUint::from(b));
                let (ma, mb) = (to_montgomery(a), to_montgomery(b));

                assert_eq!(from_montgomery(ma), a, "round trip of {a} mod {n}");
                assert_eq!(
                    BigUint::from(from_montgomery(modulus.mul(ma, mb))),
                    (&big_a * &big_b) % &big_n,
                    "{a} * {b} mod {n}"
                );
                assert_eq!(
                    BigUint::from(from_montgomery(modulus.add(ma, mb))),
                    (&big_a + &big_b) % &big_n,
                    "{a} + {b} mod {n}"
                );
                assert_eq!(
                    BigUint::from(from_montgomery(modulus.sub(ma, mb))),
                    (&big_a + &big_n - &big_b) % &big_n,
                    "{a} - {b} mod {n}"
                );
                let half = from_montgomery(modulus.halve(ma));
                assert_eq!(
                    (BigUint::from(half) * 2u32) % &big_n,
                    big_a,
                    "{a} / 2 mod {n}"
                );
                assert_eq!(
                    BigUint::from(from_montgomery(modulus.pow(ma, b))),
                    big_a.modpow(&big_b, &big_n),
                    "{a} ^ {b} mod {n}"
                );
            }
            assert_eq!(
                BigUint::from(from_montgomery(modulus.small_to_montgomery(-7))),
                (&big_n - (BigUint::from(7u32) % &big_n)) % &big_n
            );
        }
    }

    #[test]
    fn jacobi_symbol() {
        // Squares are residues, and the symbol is multiplicative.
        assert_eq!(jacobi(0, 9), 0);
        assert_eq!(jacobi(1, 9), 1);
        assert_eq!(jacobi(2, 7), 1);
        assert_eq!(jacobi(3, 7), -1);
        assert_eq!(jacobi(5, 21), 1);
        assert_eq!(jacobi(6, 21), 0);
        assert_eq!(jacobi(1001, 9907), -1);
        for n in random_numbers(3).map(|n| n | 1).take(200) {
            let (a, b) = (n >> 70, n >> 80);
            assert_eq!(jacobi(a, n) * jacobi(b, n), jacobi(a * b, n));
            assert!(matches!(jacobi(b * b, n), 0 | 1));
        }
    }

    #[test]
    fn known_primes_and_composites() {
        let primes = [
            (1 << 64) + 13,
            (1 << 89) - 1,
            (1 << 107) - 1,
            (1 << 127) - 1,
            // The largest prime below 2^128.
            u128::MAX - 158,
        ];
        for n in primes {
            assert!(Montgomery::new(n).is_prime(), "{n} is prime");
            assert_eq!(pairs(&factorize(n)), [(n, 1)]);
        }

        let p = (1 << 61) - 1;
        let q = (1 << 64) + 13;
        let composites = [
            // Squares and products of large primes.
            p * p,
            next_prime(1 << 63) * next_prime(1 << 63),
            next_prime(1 << 63) * next_prime((1 << 63) + 1000),
            p * q,
            // Mersenne numbers with a prime exponent are strong pseudoprimes
            // to base 2: 2^67 - 1, 2^101 - 1, 2^103 - 1.
            (1 << 67) - 1,
            (1 << 101) - 1,
            (1 << 103) - 1,
            // Fermat numbers are too: 2^64 + 1.
            (1 << 64) + 1,
            u128::MAX,
        ];
        for n in composites {
            assert!(!reference_is_prime(n));
            assert!(!Montgomery::new(n).is_prime(), "{n} is composite");
        }
    }

    #[test]
    fn primality_matches_reference() {
        // Odd numbers of every size above 2^64, with enough primes among
        // them: every candidate is tested along with the next odd numbers.
        let candidates = random_numbers(4)
            .enumerate()
            .map(|(i, n)| (n >> (i % 63)) | (1 << 64) | 1)
            .take(3_000);
        let mut primes = 0;
        for start in candidates {
            for n in (start..).step_by(2).take(8) {
                let expected = reference_is_prime(n);
                assert_eq!(Montgomery::new(n).is_prime(), expected, "{n}");
                primes += usize::from(expected);
            }
        }
        assert!(primes > 100, "only {primes} primes were tested");
    }

    #[test]
    fn edge_cases() {
        assert_eq!(pairs(&factorize(0)), []);
        assert_eq!(pairs(&factorize(1)), []);
        assert_eq!(pairs(&factorize(97)), [(97, 1)]);
        assert_eq!(pairs(&factorize(1 << 127)), [(2, 127)]);
        assert_eq!(pairs(&factorize(1 << 64)), [(2, 64)]);
        assert_eq!(pairs(&factorize(3_u128.pow(80))), [(3, 80)]);
        assert_eq!(
            pairs(&factorize(u128::MAX)),
            [
                (3, 1),
                (5, 1),
                (17, 1),
                (257, 1),
                (641, 1),
                (65537, 1),
                (274_177, 1),
                (6_700_417, 1),
                (67_280_421_310_721, 1)
            ]
        );

        // The product of the first 26 primes: as many distinct factors as fit.
        let primorial: u128 = (2..=101).filter(|&n| reference_is_prime(n)).product();
        let first_primes = factorize(primorial);
        assert_eq!(first_primes.iter().count(), 26);
        assert!(first_primes.iter().all(|(_, exponent)| exponent == 1));
        assert_matches_reference(primorial);

        // Powers of primes on both sides of 2^64.
        let below = 18_446_744_073_709_551_557;
        let above = (1 << 64) + 13;
        assert_eq!(pairs(&factorize(below * below)), [(below, 2)]);
        assert_eq!(pairs(&factorize(above * below)), [(below, 1), (above, 1)]);
        assert_eq!(pairs(&factorize(4099_u128.pow(10))), [(4099, 10)]);
        assert_eq!(pairs(&factorize(1_000_003_u128.pow(6))), [(1_000_003, 6)]);
        let p = next_prime(1 << 40);
        assert_eq!(pairs(&factorize(p * p * p)), [(p, 3)]);
        assert_eq!(
            pairs(&factorize(4 * 9 * p * p * p)),
            [(2, 2), (3, 2), (p, 3)]
        );

        for n in (1 << 64) - 300..(1 << 64) + 300 {
            assert_matches_reference(n);
        }
    }

    #[test]
    fn random_numbers_match_reference() {
        // Products of two random numbers, one of them of 64 bits: above
        // 2^64, with prime factors that Pollard's rho finds quickly.
        for (i, n) in random_numbers(5).take(300).enumerate() {
            let (a, b) = (n >> 64 | 1 << 63, (n as u64 as u128) >> (i % 40 + 24));
            assert_matches_reference(a * (b | 1));
        }
    }

    #[test]
    fn products_of_large_primes() {
        let mut primes = random_numbers(6)
            .map(|n| (n >> 96) | (1 << 31) | 1)
            .filter(|&n| reference_is_prime(n));
        for _ in 0..30 {
            let mut chosen = [
                primes.next().unwrap(),
                primes.next().unwrap(),
                primes.next().unwrap(),
            ];
            chosen.sort_unstable();
            let [p, q, r] = chosen;
            assert_eq!(pairs(&factorize(p * q * r)), [(p, 1), (q, 1), (r, 1)]);
            assert_eq!(pairs(&factorize(p * p * q)), [(p, 2), (q, 1)]);
            assert_matches_reference(p * q * r);
            assert_factors_multiply_back(6 * 4099 * p * q * r);
        }
    }

    #[test]
    fn fermat_finds_close_factors_only() {
        let p = next_prime(1 << 63);
        let q = next_prime(p + 1);
        assert_eq!(fermat_divisor(p * p), Some(p));
        assert_eq!(fermat_divisor(p * q), Some(p));
        // 2^128 - 1 = (2^64 - 1) * (2^64 + 1), with `a^2` past `u128::MAX`.
        assert_eq!(fermat_divisor(u128::MAX), None);
        assert_eq!(fermat_divisor(4099 * p), None);
    }

    #[test]
    fn gcd_with_an_odd_number() {
        assert_eq!(gcd(0, 9), 9);
        assert_eq!(gcd(12, 9), 3);
        assert_eq!(gcd(1 << 100, 9), 1);
        let p = (1 << 64) + 13;
        assert_eq!(gcd(p * 6, p * 35), p);
    }
}
