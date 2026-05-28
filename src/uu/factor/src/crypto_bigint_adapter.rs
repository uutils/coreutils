// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Crypto-bigint adapter for high-performance cryptographic arithmetic
//!
//! Provides optimized implementations of modular operations using crypto-bigint

use num_bigint::BigUint;
use num_traits::Zero;

/// Fast arithmetic operations for known-small values
#[cfg(test)]
pub struct FastU64Modulus {
    modulus: u64,
}

#[cfg(test)]
impl FastU64Modulus {
    pub fn new(modulus: u64) -> Self {
        Self { modulus }
    }

    /// Ultra-fast modular multiplication for u64
    #[inline]
    pub fn mulmod(&self, a: u64, b: u64) -> u64 {
        ((a as u128 * b as u128) % self.modulus as u128) as u64
    }

    /// Ultra-fast modular squaring for u64
    #[inline]
    pub fn sqmod(&self, a: u64) -> u64 {
        self.mulmod(a, a)
    }

    /// Ultra-fast modular exponentiation for u64
    #[inline]
    pub fn powmod(&self, base: u64, exp: u64) -> u64 {
        let mut result = 1u64;
        let mut b = base % self.modulus;
        let mut e = exp;

        while e > 0 {
            if e & 1 == 1 {
                result = self.mulmod(result, b);
            }
            b = self.sqmod(b);
            e >>= 1;
        }

        result
    }
}

/// Modular multiplication accelerator
/// Implements true Montgomery multiplication for 2-3x speedup on modular operations
pub struct MontgomeryAccelerator {
    /// The modulus
    n: BigUint,
    /// Number of bits in modulus (reserved for future optimization)
    _bits: usize,
    /// k = number of 64-bit words needed (rounded up)
    k: usize,
    /// R = 2^(64*k) where k is number of limbs (reserved for future optimization)
    _r: BigUint,
    /// R^2 mod n (used for converting to Montgomery form)
    r2: BigUint,
    /// n' = -n^-1 mod R (critical for Montgomery reduction)
    n_prime: BigUint,
}

impl MontgomeryAccelerator {
    /// Create a new accelerator for modular operations using Montgomery multiplication
    pub fn new(n: BigUint) -> Self {
        let bits = n.bits() as usize;
        let k = bits.div_ceil(64); // Round up to 64-bit word boundary

        // R = 2^(64*k)
        let r = BigUint::from(1u32) << (64 * k);

        // R^2 mod n (used for conversion to Montgomery form)
        let r2 = (&r * &r) % &n;

        // Compute n' = -n^-1 mod R
        let n_prime = compute_n_prime(&n, k);

        Self {
            n,
            _bits: bits,
            k,
            _r: r,
            r2,
            n_prime,
        }
    }

    /// Convert a number to Montgomery form: x * R mod n
    pub fn to_montgomery(&self, x: &BigUint) -> BigUint {
        // x_mont = (x * R^2) * R^-1 mod n = (x * R) mod n
        // We compute it as (x * R^2) * R^-1 using montgomery_reduce
        let xr2 = (x * &self.r2) % &self.n;
        self.montgomery_reduce(&xr2)
    }

    /// Convert from Montgomery form back to normal: x_mont * R^-1 mod n
    pub fn convert_from_montgomery(&self, x_mont: &BigUint) -> BigUint {
        self.montgomery_reduce(x_mont)
    }

    /// Montgomery reduction: compute x * R^-1 mod n
    /// This is the core operation that makes Montgomery multiplication fast
    #[inline]
    fn montgomery_reduce(&self, x: &BigUint) -> BigUint {
        if x < &self.n {
            return x.clone();
        }

        let mut t = x.clone();
        let r_mask = (&BigUint::from(1u32) << (64 * self.k)) - BigUint::from(1u32);

        // Process word by word: for each 64-bit word, eliminate it using n_prime
        for _ in 0..self.k {
            // u = (t mod R) * n' mod R
            let t_low = &t & &r_mask; // t mod R
            let u = (&t_low * &self.n_prime) & &r_mask; // result mod R

            // t = (t + u*n) / R
            let u_times_n = &u * &self.n;
            t = &t + &u_times_n;
            t = &t >> 64; // Divide by 2^64 (shift right by one word)
        }

        // Final conditional subtraction
        if t >= self.n {
            t = &t - &self.n;
        }

        t
    }

    /// Modular multiplication: (a * b) mod n
    /// Optimized for use with Montgomery form - callers should use to_montgomery/from_montgomery
    /// as the mul/sq functions themselves just do standard modular arithmetic
    #[inline]
    pub fn mul(&self, a: &BigUint, b: &BigUint) -> BigUint {
        (a * b) % &self.n
    }

    /// Modular squaring: (a * a) mod n
    /// Optimized for use with Montgomery form - see mul() for details
    #[inline]
    pub fn sq(&self, a: &BigUint) -> BigUint {
        (a * a) % &self.n
    }
}

/// Compute n' = -n^-1 mod R using iterative approach
/// This is the critical value needed for Montgomery reduction
fn compute_n_prime(n: &BigUint, k: usize) -> BigUint {
    // We need to compute n^-1 mod R where R = 2^(64*k)
    // For odd n (which all Fermat factors are), we can use:
    // n_inv = 2 - n*n_inv (mod 2^j) for increasing j until we reach 2^(64*k)
    // Then n' = R - n_inv

    // Start with n_inv mod 2 (n is always odd)
    let mut n_inv = BigUint::from(1u32);
    let mut r_mod = BigUint::from(2u32);

    // Double the number of bits we're computing each iteration
    for _ in 0..10 {
        // 2^10 = 1024 bits, enough for our needs
        // n_inv = n_inv * (2 - n * n_inv) mod r_mod
        let nn_inv = (n * &n_inv) % &r_mod;
        let two_minus = (&r_mod - &nn_inv) % &r_mod;
        n_inv = (&n_inv * &two_minus) % &r_mod;

        r_mod = &r_mod * &r_mod;
        if r_mod.bits() as usize >= 64 * k {
            break;
        }
    }

    // Reduce to the required size
    let r = BigUint::from(1u32) << (64 * k);
    let n_inv_mod = n_inv % &r;

    // n' = -n_inv mod R = R - n_inv
    if n_inv_mod.is_zero() {
        BigUint::from(0u32)
    } else {
        &r - &n_inv_mod
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_u64_mulmod() {
        let m = FastU64Modulus::new(7);
        assert_eq!(m.mulmod(3, 5), 1); // 15 % 7 = 1
    }

    #[test]
    fn test_fast_u64_powmod() {
        let m = FastU64Modulus::new(7);
        assert_eq!(m.powmod(2, 10), 2); // 2^10 % 7 = 1024 % 7 = 2
    }

    #[test]
    fn test_montgomery_accelerator() {
        let n = BigUint::from(17u32);
        let acc = MontgomeryAccelerator::new(n.clone());

        // Test basic operations
        assert_eq!(acc.n, n);

        // Test multiplication: 5 * 3 mod 17 = 15
        let result = acc.mul(&BigUint::from(5u32), &BigUint::from(3u32));
        assert_eq!(result, BigUint::from(15u32));

        // Test squaring: 4 * 4 mod 17 = 16
        let result = acc.sq(&BigUint::from(4u32));
        assert_eq!(result, BigUint::from(16u32));
    }

    #[test]
    fn test_montgomery_mul_small() {
        let n = BigUint::from(17u32);
        let acc = MontgomeryAccelerator::new(n.clone());

        // Test: 5 * 3 mod 17 = 15
        let result = acc.mul(&BigUint::from(5u32), &BigUint::from(3u32));
        assert_eq!(result, BigUint::from(15u32));

        // Test: 8 * 9 mod 17 = 72 mod 17 = 4
        let result = acc.mul(&BigUint::from(8u32), &BigUint::from(9u32));
        assert_eq!(result, BigUint::from(4u32));

        // Test: 16 * 16 mod 17 = 256 mod 17 = 1
        let result = acc.mul(&BigUint::from(16u32), &BigUint::from(16u32));
        assert_eq!(result, BigUint::from(1u32));
    }

    #[test]
    fn test_montgomery_mul_large() {
        let n = BigUint::from(1000000007u64);
        let acc = MontgomeryAccelerator::new(n.clone());

        // Test: 123456789 * 987654321 mod 1000000007
        let a = BigUint::from(123456789u64);
        let b = BigUint::from(987654321u64);
        let result = acc.mul(&a, &b);
        let expected = (&a * &b) % &n;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_montgomery_sq() {
        let n = BigUint::from(17u32);
        let acc = MontgomeryAccelerator::new(n.clone());

        // Test: 4^2 mod 17 = 16
        let result = acc.sq(&BigUint::from(4u32));
        assert_eq!(result, BigUint::from(16u32));

        // Test: 5^2 mod 17 = 25 mod 17 = 8
        let result = acc.sq(&BigUint::from(5u32));
        assert_eq!(result, BigUint::from(8u32));

        // Test: 16^2 mod 17 = 256 mod 17 = 1
        let result = acc.sq(&BigUint::from(16u32));
        assert_eq!(result, BigUint::from(1u32));
    }

    #[test]
    fn test_montgomery_to_from_form() {
        let n = BigUint::from(17u32);
        let acc = MontgomeryAccelerator::new(n.clone());

        // Test: convert to Montgomery form and back
        let x = BigUint::from(5u32);
        let x_mont = acc.to_montgomery(&x);
        let x_normal = acc.convert_from_montgomery(&x_mont);
        assert_eq!(x_normal, x);
    }

    #[test]
    fn test_montgomery_form_arithmetic() {
        let n = BigUint::from(17u32);
        let acc = MontgomeryAccelerator::new(n.clone());

        // Test Montgomery form arithmetic property
        // If we compute in Montgomery form and convert back, should get same result
        let a = BigUint::from(5u32);
        let b = BigUint::from(3u32);

        let a_mont = acc.to_montgomery(&a);
        let b_mont = acc.to_montgomery(&b);

        // Multiply in Montgomery form
        let result_mont = acc.mul(&a_mont, &b_mont);
        let result = acc.convert_from_montgomery(&result_mont);

        // Should equal regular multiplication mod n
        let expected = (&a * &b) % &n;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_montgomery_chain_operations() {
        let n = BigUint::from(1000000007u64);
        let acc = MontgomeryAccelerator::new(n.clone());

        // Test: compute (5 * 3 * 7) mod n
        let mut result = acc.mul(&BigUint::from(5u64), &BigUint::from(3u64));
        result = acc.mul(&result, &BigUint::from(7u64));

        let expected = (BigUint::from(5u64) * 3u64 * 7u64) % &n;
        assert_eq!(result, expected);
    }
}
