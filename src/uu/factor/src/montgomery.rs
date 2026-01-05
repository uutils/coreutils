// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Montgomery multiplication module for efficient modular arithmetic
//!
//! Montgomery multiplication replaces expensive division with bit shifts,
//! allowing us to compute (a * b) mod n efficiently. This is critical for
//! Pollard's rho factorization where we perform thousands of modular multiplications.

use num_bigint::BigUint;
use num_integer::Integer;
use num_traits::Zero;

/// Montgomery multiplication context for efficient modular arithmetic
///
/// True Montgomery multiplication using REDC algorithm.
/// Replaces expensive division with bit shifts and multiplications.
pub struct MontgomeryContext {
    n: BigUint,       // The modulus
    r: BigUint,       // R = 2^k where k >= bits(n)
    _r_inv: BigUint,  // R^-1 mod n (reserved for future optimization)
    n_prime: BigUint, // -n^-1 mod R
    k: u64,           // Bit length (R = 2^k)
}

impl MontgomeryContext {
    /// Create a new Montgomery context for modulus n
    ///
    /// Returns None for even moduli (not supported by Montgomery reduction).
    pub fn new(n: &BigUint) -> Option<Self> {
        if n.is_zero() || n.is_even() {
            // Montgomery reduction requires odd modulus
            return None;
        }

        let k = n.bits();
        let r = BigUint::from(1u32) << k; // R = 2^k

        // Compute R^-1 mod n using modular inverse
        let r_inv = mod_inverse(&r, n)?;

        // Compute n' = -n^-1 mod R
        let n_inv = mod_inverse(n, &r)?;
        let n_prime = (&r - n_inv) % &r; // -n_inv mod R

        Some(Self {
            n: n.clone(),
            r,
            _r_inv: r_inv,
            n_prime,
            k,
        })
    }

    /// REDC (Montgomery Reduction) algorithm
    /// Input: T < R*n
    /// Output: T*R^-1 mod n
    fn redc(&self, t: &BigUint) -> BigUint {
        // m = (T mod R) * n' mod R
        let t_mod_r = t & (&self.r - BigUint::from(1u32)); // T mod R (since R is power of 2)
        let m = (&t_mod_r * &self.n_prime) & (&self.r - BigUint::from(1u32)); // mod R

        // t = (T + m*n) / R
        let mn = &m * &self.n;
        let t_plus_mn = t + &mn;
        let result = &t_plus_mn >> self.k; // Divide by R = 2^k

        // Final reduction
        if result >= self.n {
            &result - &self.n
        } else {
            result
        }
    }

    /// Convert to Montgomery form: a*R mod n
    /// Since a can be >= n, we use standard modulo here.
    /// The benefit of Montgomery is in the mul() and add() operations.
    pub fn to_montgomery(&self, a: &BigUint) -> BigUint {
        (a * &self.r) % &self.n
    }

    /// Convert from Montgomery form: a*R^-1 mod n
    pub fn convert_from_montgomery(&self, a: &BigUint) -> BigUint {
        self.redc(a)
    }

    /// Montgomery multiplication: (a * b) mod n
    /// Inputs must be in Montgomery form
    /// Output is in Montgomery form
    pub fn mul(&self, a: &BigUint, b: &BigUint) -> BigUint {
        let product = a * b;
        self.redc(&product)
    }

    /// Montgomery addition: (a + b) mod n
    /// Inputs must be in Montgomery form
    /// Output is in Montgomery form
    /// Reserved for future Montgomery chain optimization
    pub fn _add(&self, a: &BigUint, b: &BigUint) -> BigUint {
        let sum = a + b;
        if sum >= self.n { &sum - &self.n } else { sum }
    }

    /// Get the modulus
    /// Reserved for future use
    pub fn _modulus(&self) -> &BigUint {
        &self.n
    }
}

/// Fast modular multiplication using optimized reduction
///
/// This uses Montgomery-inspired techniques to speed up modular multiplication.
/// Falls back to standard modulo for even moduli.
/// Reserved for future integration into factorization pipeline
pub fn _montg_mul(a: &BigUint, b: &BigUint, n: &BigUint) -> BigUint {
    if n.is_even() {
        // Fall back to standard modular multiplication for even moduli
        return (a * b) % n;
    }

    // For odd n, use Montgomery context
    if let Some(ctx) = MontgomeryContext::new(n) {
        ctx.mul(a, b)
    } else {
        (a * b) % n
    }
}

/// Batch modular multiplication with Montgomery context
///
/// More efficient than calling montg_mul repeatedly since it reuses
/// the Montgomery context across multiple operations.
/// Reserved for future batch optimization
pub fn _batch_montg_mul(operations: &[(BigUint, BigUint)], n: &BigUint) -> Vec<BigUint> {
    if n.is_even() {
        return operations.iter().map(|(a, b)| (a * b) % n).collect();
    }

    if let Some(ctx) = MontgomeryContext::new(n) {
        operations.iter().map(|(a, b)| ctx.mul(a, b)).collect()
    } else {
        operations.iter().map(|(a, b)| (a * b) % n).collect()
    }
}

/// Compute modular inverse: a^-1 mod m
/// Returns None if inverse doesn't exist
fn mod_inverse(a: &BigUint, m: &BigUint) -> Option<BigUint> {
    use num_traits::One;

    if m <= &BigUint::one() {
        return None;
    }

    let mut t = BigUint::zero();
    let mut newt = BigUint::one();
    let mut r = m.clone();
    let mut newr = a % m;

    while !newr.is_zero() {
        let quotient = &r / &newr;

        let temp = newt.clone();
        newt = if t >= &quotient * &newt {
            &t - &quotient * &newt
        } else {
            // t < quotient * newt, so result would be negative
            // In modular arithmetic: -x mod m = m - (x mod m)
            let diff = (&quotient * &newt - &t) % m;
            if diff.is_zero() {
                BigUint::zero()
            } else {
                m - diff
            }
        };
        t = temp;

        // Update r and newr
        let temp = newr.clone();
        newr = &r - &quotient * &newr;
        r = temp;
    }

    if r > BigUint::one() {
        return None; // No inverse exists
    }

    Some(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_montgomery_multiplication() {
        let n = BigUint::from(17u64);
        let a = BigUint::from(5u64);
        let b = BigUint::from(3u64);

        let ctx = MontgomeryContext::new(&n).unwrap();

        // Convert to Montgomery form
        let a_mont = ctx.to_montgomery(&a);
        let b_mont = ctx.to_montgomery(&b);

        // Multiply in Montgomery form
        let result_mont = ctx.mul(&a_mont, &b_mont);

        // Convert back from Montgomery form
        let result = ctx.convert_from_montgomery(&result_mont);
        let expected = (&a * &b) % &n;

        assert_eq!(result, expected);
    }

    #[test]
    fn test_montgomery_large() {
        let n = BigUint::from(1000000007u64);
        let a = BigUint::from(123456789u64);
        let b = BigUint::from(987654321u64);

        let ctx = MontgomeryContext::new(&n).unwrap();

        // Convert to Montgomery form
        let a_mont = ctx.to_montgomery(&a);
        let b_mont = ctx.to_montgomery(&b);

        // Multiply in Montgomery form
        let result_mont = ctx.mul(&a_mont, &b_mont);

        // Convert back from Montgomery form
        let result = ctx.convert_from_montgomery(&result_mont);
        let expected = (&a * &b) % &n;

        assert_eq!(result, expected);
    }

    #[test]
    fn test_context_creation() {
        let n = BigUint::from(15u64); // 15 is odd
        assert!(MontgomeryContext::new(&n).is_some());

        let n_even = BigUint::from(16u64);
        assert!(MontgomeryContext::new(&n_even).is_none());
    }
}
