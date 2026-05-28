// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Montgomery multiplication for u128 values
//!
//! This module provides fast modular arithmetic for u128 values using
//! Montgomery reduction. Montgomery multiplication replaces expensive
//! division with bit shifts and multiplications, which is critical for
//! Pollard-Rho factorization where we perform millions of modular multiplications.
//!
//! Key insight: On x86-64, u128 multiplication requires 20-30 instructions
//! (no native 128x128→256 multiply), but Montgomery reduction trades
//! expensive modular division for cheaper multiplications and additions.

/// Montgomery context for u128 modular arithmetic
///
/// Pre-computes constants for efficient Montgomery reduction:
/// - R = 2^128 (implicit, not stored)
/// - n' = -n^-1 mod R (for REDC algorithm)
/// - R^2 mod n (for to_montgomery conversion)
#[derive(Clone, Debug)]
pub struct MontgomeryU128 {
    n: u128,       // The odd modulus
    n_prime: u128, // n' = -n^-1 mod 2^128
    r2: u128,      // R^2 mod n (for to_montgomery)
}

impl MontgomeryU128 {
    /// Create a new Montgomery context for modulus n
    ///
    /// Returns None for even moduli or n <= 1 (Montgomery requires odd modulus > 1)
    #[inline]
    pub fn new(n: u128) -> Option<Self> {
        if n <= 1 || n % 2 == 0 {
            return None;
        }

        let n_prime = compute_n_prime_u128(n);
        let r2 = compute_r2_mod_n(n);

        Some(Self { n, n_prime, r2 })
    }

    /// REDC (Montgomery Reduction): compute T * R^-1 mod n
    ///
    /// Input: t = (t_hi, t_lo) representing T = t_hi * 2^128 + t_lo
    /// where T < n * R (guaranteed by multiplication of values < n)
    /// Output: T * R^-1 mod n (fits in u128)
    ///
    /// Algorithm:
    /// 1. m = (T mod R) * n' mod R = t_lo * n_prime (wrapping)
    /// 2. t = (T + m*n) / R
    /// 3. if t >= n then t = t - n
    #[inline]
    pub fn redc(&self, t_hi: u128, t_lo: u128) -> u128 {
        // m = (T mod R) * n' mod R
        // Since R = 2^128, mod R is automatic with wrapping_mul
        let m = t_lo.wrapping_mul(self.n_prime);

        // Compute m * n as 256-bit value
        let (mn_lo, mn_hi) = mul_u128_full(m, self.n);

        // T + m*n = (t_hi * 2^128 + t_lo) + (mn_hi * 2^128 + mn_lo)
        let (_sum_lo, carry1) = t_lo.overflowing_add(mn_lo);
        let sum_hi = t_hi.wrapping_add(mn_hi).wrapping_add(carry1 as u128);

        // Divide by R = 2^128 is just taking the high 128 bits
        // The low 128 bits are guaranteed to be 0 because:
        // T + m*n ≡ 0 (mod R) by construction of m
        // So (T + m*n) / R is exact (no remainder)
        let result = sum_hi;

        // Final conditional subtraction
        if result >= self.n {
            result - self.n
        } else {
            result
        }
    }

    /// Convert to Montgomery form: a * R mod n
    ///
    /// Uses the identity: a_mont = redc(a * R^2) = a * R^2 * R^-1 = a * R mod n
    #[inline]
    pub fn to_montgomery(&self, a: u128) -> u128 {
        let a = a % self.n; // Ensure a < n
        let (lo, hi) = mul_u128_full(a, self.r2);
        self.redc(hi, lo)
    }

    /// Convert from Montgomery form: a_mont * R^-1 mod n
    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub fn from_montgomery(&self, a_mont: u128) -> u128 {
        self.redc(0, a_mont)
    }

    /// Montgomery multiplication: (a * b * R^-1) mod n
    ///
    /// If inputs are in Montgomery form (a*R, b*R), output is also in
    /// Montgomery form: (a*R) * (b*R) * R^-1 = a*b*R mod n
    #[inline]
    pub fn mul(&self, a: u128, b: u128) -> u128 {
        let (lo, hi) = mul_u128_full(a, b);
        self.redc(hi, lo)
    }

    /// Montgomery squaring: (a * a * R^-1) mod n
    ///
    /// Slightly faster than mul for squaring due to reduced multiplication
    #[inline]
    pub fn square(&self, a: u128) -> u128 {
        let (lo, hi) = mul_u128_full(a, a);
        self.redc(hi, lo)
    }

    /// Montgomery addition: (a + b) mod n
    ///
    /// Works for both Montgomery and regular form
    #[inline]
    pub fn add(&self, a: u128, b: u128) -> u128 {
        let (sum, overflow) = a.overflowing_add(b);
        if overflow || sum >= self.n {
            sum.wrapping_sub(self.n)
        } else {
            sum
        }
    }

    /// Montgomery subtraction: (a - b) mod n
    ///
    /// Returns (a - b + n) mod n, works for Montgomery form values
    #[inline]
    pub fn sub(&self, a: u128, b: u128) -> u128 {
        if a >= b { a - b } else { self.n - (b - a) }
    }

    /// Get the modulus
    #[inline]
    pub fn modulus(&self) -> u128 {
        self.n
    }
}

/// Compute n' = -n^-1 mod 2^128 using Newton-Raphson iteration
///
/// Uses the fact that n is odd, so n^-1 mod 2 = 1.
/// Then lifts using: x_{i+1} = x_i * (2 - n * x_i) mod 2^{2^i}
///
/// After 7 iterations, we have n^-1 mod 2^128, then negate.
#[inline]
fn compute_n_prime_u128(n: u128) -> u128 {
    // Start with n^-1 mod 2 = 1 (since n is odd)
    let mut x = 1u128;

    // Newton-Raphson iteration: double precision each iteration
    // After k iterations: x = n^-1 mod 2^(2^k)
    // We need 2^7 = 128 bits
    for _ in 0..7 {
        x = x.wrapping_mul(2u128.wrapping_sub(n.wrapping_mul(x)));
    }

    // n' = -n^-1 mod 2^128
    x.wrapping_neg()
}

/// Compute R^2 mod n where R = 2^128
///
/// Strategy: Compute 2^128 mod n iteratively, then square
fn compute_r2_mod_n(n: u128) -> u128 {
    // Compute 2^128 mod n using repeated squaring of 2
    // Start with 2^1, square 7 times to get 2^128

    // First, compute 2^64 mod n
    let mut r = 1u128;
    for _ in 0..64 {
        r = addmod_slow(r, r, n); // r = 2*r mod n
    }
    // Now r = 2^64 mod n

    // Square to get 2^128 mod n
    r = mulmod_slow(r, r, n);
    // Now r = 2^128 mod n = R mod n

    // Square again to get R^2 mod n
    mulmod_slow(r, r, n)
}

/// Slow modular addition (for initialization only)
#[inline]
fn addmod_slow(a: u128, b: u128, m: u128) -> u128 {
    let (sum, overflow) = a.overflowing_add(b);
    if overflow || sum >= m {
        sum.wrapping_sub(m)
    } else {
        sum
    }
}

/// Slow modular multiplication using binary method (for initialization only)
///
/// O(log min(a,b)) additions, used only during Montgomery context setup
fn mulmod_slow(mut a: u128, mut b: u128, m: u128) -> u128 {
    a %= m;
    b %= m;

    // Use smaller operand for loop
    if a > b {
        std::mem::swap(&mut a, &mut b);
    }

    let mut result = 0u128;

    while a > 0 {
        if a & 1 == 1 {
            result = addmod_slow(result, b, m);
        }
        b = addmod_slow(b, b, m);
        a >>= 1;
    }

    result
}

/// Full 256-bit multiplication of two u128 values
///
/// Returns (low, high) where result = high * 2^128 + low
///
/// Uses schoolbook multiplication with 64-bit limbs:
/// a = a_hi * 2^64 + a_lo
/// b = b_hi * 2^64 + b_lo
/// a * b = a_hi*b_hi * 2^128 + (a_lo*b_hi + a_hi*b_lo) * 2^64 + a_lo*b_lo
#[inline]
fn mul_u128_full(a: u128, b: u128) -> (u128, u128) {
    let a_lo = a as u64 as u128;
    let a_hi = (a >> 64) as u64 as u128;
    let b_lo = b as u64 as u128;
    let b_hi = (b >> 64) as u64 as u128;

    // Four partial products (each fits in 128 bits)
    let p0 = a_lo * b_lo; // bits 0-127
    let p1 = a_lo * b_hi; // bits 64-191
    let p2 = a_hi * b_lo; // bits 64-191
    let p3 = a_hi * b_hi; // bits 128-255

    // Combine middle terms with carry tracking
    let (mid, carry_mid) = p1.overflowing_add(p2);

    // Low 128 bits: p0 + (mid_lo << 64)
    let mid_lo = mid << 64;
    let (lo, carry_lo) = p0.overflowing_add(mid_lo);

    // High 128 bits: p3 + (mid >> 64) + carries
    let mid_hi = mid >> 64;
    let carry_mid_contrib = if carry_mid { 1u128 << 64 } else { 0 };
    let hi = p3
        .wrapping_add(mid_hi)
        .wrapping_add(carry_mid_contrib)
        .wrapping_add(carry_lo as u128);

    (lo, hi)
}

// =============================================================================
// Pollard-Rho with Montgomery u128 optimization
// =============================================================================

/// Pollard-Rho Brent factorization with Montgomery u128 optimization
///
/// This version keeps all arithmetic in Montgomery form during the inner loop,
/// converting only for GCD checks. This avoids expensive modular division
/// in the hot path.
pub fn pollard_rho_brent_u128_montgomery(n: u128) -> Option<u128> {
    // Quick small factor checks
    if n < 2 {
        return None;
    }
    if n % 2 == 0 {
        return Some(2);
    }
    if n % 3 == 0 {
        return Some(3);
    }
    if n % 5 == 0 {
        return Some(5);
    }

    // Check if prime
    if is_probable_prime_u128_montgomery(n) {
        return None;
    }

    // Create Montgomery context (n is odd at this point)
    let mont = MontgomeryU128::new(n)?;

    // Convert constant 1 to Montgomery form for product accumulation
    let one_mont = mont.to_montgomery(1);

    const MAX_ITERATIONS: u64 = 500_000_000;

    for attempt in 0..20 {
        // Generate pseudo-random starting values based on attempt
        let seed = (n
            .wrapping_mul(1103515245)
            .wrapping_add(12345 + attempt as u128))
            % n;
        let x0 = seed.max(2);
        let c = ((seed.wrapping_mul(1103515245).wrapping_add(12345)) % n).max(1);

        // Convert to Montgomery form
        let x0_mont = mont.to_montgomery(x0);
        let c_mont = mont.to_montgomery(c);

        if let Some(factor) =
            brent_cycle_find_u128_montgomery(&mont, x0_mont, c_mont, one_mont, MAX_ITERATIONS)
        {
            if factor > 1 && factor < n {
                return Some(factor);
            }
        }
    }

    None
}

/// Brent's cycle finding with Montgomery optimization
///
/// All arithmetic stays in Montgomery form except for GCD checks.
/// This is the key optimization over the non-Montgomery version.
#[allow(clippy::many_single_char_names)]
fn brent_cycle_find_u128_montgomery(
    mont: &MontgomeryU128,
    x0_mont: u128,
    c_mont: u128,
    one_mont: u128,
    max_iterations: u64,
) -> Option<u128> {
    let n = mont.modulus();
    let mut x = x0_mont;
    let mut y = x0_mont;
    let mut d;
    let mut r = 1u64;
    let mut q = one_mont;

    // Batch GCD: accumulate products instead of checking GCD every iteration
    const BATCH_SIZE: u64 = 100;

    loop {
        for batch in 0..=(r / BATCH_SIZE) {
            let batch_limit = (batch + 1) * BATCH_SIZE;
            let limit = if batch_limit > r { r } else { batch_limit };
            let start = batch * BATCH_SIZE;

            for _ in start..limit {
                // f(x) = x^2 + c in Montgomery form
                x = mont.add(mont.square(x), c_mont);

                // diff = (x - y) mod n in Montgomery form
                // Using modular subtraction works for GCD because
                // gcd((x-y) mod n, n) = gcd(x-y, n) for any factor p|n
                let diff_mont = mont.sub(x, y);

                // Avoid multiplying by zero
                let diff_mont = if diff_mont == 0 { one_mont } else { diff_mont };

                // Accumulate product: q = q * diff mod n (all in Montgomery form)
                q = mont.mul(q, diff_mont);

                if q == 0 {
                    q = one_mont; // Reset if product collapsed to 0
                }
            }

            // Batch GCD check after BATCH_SIZE iterations
            if batch < r / BATCH_SIZE {
                // Convert from Montgomery form for GCD
                let q_reg = mont.from_montgomery(q);
                d = gcd_u128(q_reg, n);
                if d > 1 && d < n {
                    return Some(d);
                }
                if d == n {
                    // Failure: GCD collapsed to n, this c value won't work
                    return None;
                }
            }
        }

        y = x;
        r *= 2;

        // Final GCD check for this round
        let q_reg = mont.from_montgomery(q);
        d = gcd_u128(q_reg, n);
        if d > 1 && d < n {
            return Some(d);
        }
        if d == n {
            return None;
        }

        // Reset q for next round
        q = one_mont;

        if r > max_iterations {
            return None;
        }
    }
}

/// Miller-Rabin primality test with Montgomery optimization
///
/// Deterministic for all n < 2^82 using specific witness set
pub fn is_probable_prime_u128_montgomery(n: u128) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }

    // Small primes divisibility check
    const SMALL_PRIMES: [u128; 14] = [3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47];
    for &p in &SMALL_PRIMES {
        if n == p {
            return true;
        }
        if n % p == 0 {
            return false;
        }
    }

    // Factor n-1 = d * 2^s
    let n_minus_1 = n - 1;
    let s = n_minus_1.trailing_zeros();
    let d = n_minus_1 >> s;

    // Create Montgomery context
    let Some(mont) = MontgomeryU128::new(n) else {
        return false;
    };

    let one_mont = mont.to_montgomery(1);
    let neg_one_mont = mont.to_montgomery(n - 1);

    // Witnesses for deterministic test
    // These bases guarantee correctness for all n < 3,317,044,064,679,887,385,961,981
    const WITNESSES: [u128; 13] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41];

    'witness: for &a in &WITNESSES {
        if a >= n {
            continue;
        }

        // Compute a^d mod n using Montgomery exponentiation
        let a_mont = mont.to_montgomery(a);
        let mut x = powmod_montgomery(&mont, a_mont, d);

        if x == one_mont || x == neg_one_mont {
            continue 'witness;
        }

        for _ in 0..s - 1 {
            x = mont.square(x);
            if x == neg_one_mont {
                continue 'witness;
            }
        }

        return false;
    }

    true
}

/// Montgomery modular exponentiation
fn powmod_montgomery(mont: &MontgomeryU128, base_mont: u128, mut exp: u128) -> u128 {
    let mut result = mont.to_montgomery(1);
    let mut base = base_mont;

    while exp > 0 {
        if exp & 1 == 1 {
            result = mont.mul(result, base);
        }
        base = mont.square(base);
        exp >>= 1;
    }

    result
}

/// Binary GCD algorithm for u128
#[inline]
fn gcd_u128(mut a: u128, mut b: u128) -> u128 {
    if a == 0 {
        return b;
    }
    if b == 0 {
        return a;
    }

    // Factor out common powers of 2
    let shift = (a | b).trailing_zeros();
    a >>= a.trailing_zeros();

    loop {
        b >>= b.trailing_zeros();
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }
        b -= a;
        if b == 0 {
            return a << shift;
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mul_u128_full() {
        // Test basic multiplication
        let (lo, hi) = mul_u128_full(3, 5);
        assert_eq!(lo, 15);
        assert_eq!(hi, 0);

        // Test with larger values
        let a = (1u128 << 64) + 1;
        let b = (1u128 << 64) + 1;
        let (lo, hi) = mul_u128_full(a, b);
        // (2^64 + 1)^2 = 2^128 + 2^65 + 1
        assert_eq!(lo, (1u128 << 65) + 1);
        assert_eq!(hi, 1);
    }

    #[test]
    fn test_montgomery_context_creation() {
        // Valid odd modulus
        assert!(MontgomeryU128::new(17).is_some());
        assert!(MontgomeryU128::new(1000000007).is_some());

        // Invalid: even modulus
        assert!(MontgomeryU128::new(16).is_none());

        // Invalid: too small
        assert!(MontgomeryU128::new(0).is_none());
        assert!(MontgomeryU128::new(1).is_none());
    }

    #[test]
    fn test_montgomery_round_trip() {
        let n = 17u128;
        let mont = MontgomeryU128::new(n).unwrap();

        for x in 1..17 {
            let x_mont = mont.to_montgomery(x);
            let x_back = mont.from_montgomery(x_mont);
            assert_eq!(x, x_back, "Round-trip failed for {x}");
        }
    }

    #[test]
    fn test_montgomery_multiplication() {
        let n = 17u128;
        let mont = MontgomeryU128::new(n).unwrap();

        // Test 5 * 3 mod 17 = 15
        let a = 5u128;
        let b = 3u128;
        let a_mont = mont.to_montgomery(a);
        let b_mont = mont.to_montgomery(b);
        let result_mont = mont.mul(a_mont, b_mont);
        let result = mont.from_montgomery(result_mont);
        assert_eq!(result, (a * b) % n);
    }

    #[test]
    fn test_montgomery_multiplication_large() {
        let n = 1000000007u128; // Large prime
        let mont = MontgomeryU128::new(n).unwrap();

        let a = 123456789u128;
        let b = 987654321u128;
        let expected = (a * b) % n;

        let a_mont = mont.to_montgomery(a);
        let b_mont = mont.to_montgomery(b);
        let result_mont = mont.mul(a_mont, b_mont);
        let result = mont.from_montgomery(result_mont);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_montgomery_chain_multiplication() {
        let n = 1000000007u128;
        let mont = MontgomeryU128::new(n).unwrap();

        // Compute 2 * 3 * 5 * 7 mod n
        let values = [2u128, 3, 5, 7];
        let expected = values.iter().fold(1u128, |acc, &x| (acc * x) % n);

        let mut result_mont = mont.to_montgomery(1);
        for &v in &values {
            let v_mont = mont.to_montgomery(v);
            result_mont = mont.mul(result_mont, v_mont);
        }
        let result = mont.from_montgomery(result_mont);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_montgomery_squaring() {
        let n = 1000000007u128;
        let mont = MontgomeryU128::new(n).unwrap();

        let a = 123456789u128;
        let expected = (a * a) % n;

        let a_mont = mont.to_montgomery(a);
        let result_mont = mont.square(a_mont);
        let result = mont.from_montgomery(result_mont);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_pollard_rho_montgomery_small() {
        // Test with small semiprimes
        let n = 91u128; // 7 * 13
        let factor = pollard_rho_brent_u128_montgomery(n);
        assert!(factor.is_some());
        let f = factor.unwrap();
        assert!(f == 7 || f == 13);

        let n = 221u128; // 13 * 17
        let factor = pollard_rho_brent_u128_montgomery(n);
        assert!(factor.is_some());
        let f = factor.unwrap();
        assert!(f == 13 || f == 17);
    }

    #[test]
    fn test_pollard_rho_montgomery_medium() {
        // 32-bit semiprime
        let n = 2147483647u128 * 2147483629u128; // Two large primes
        let factor = pollard_rho_brent_u128_montgomery(n);
        assert!(factor.is_some());
        let f = factor.unwrap();
        assert!(n % f == 0);
    }

    #[test]
    fn test_is_prime_small() {
        assert!(is_probable_prime_u128_montgomery(2));
        assert!(is_probable_prime_u128_montgomery(3));
        assert!(is_probable_prime_u128_montgomery(5));
        assert!(is_probable_prime_u128_montgomery(17));
        assert!(is_probable_prime_u128_montgomery(1000000007));
        assert!(is_probable_prime_u128_montgomery(1000000009)); // Also prime!

        assert!(!is_probable_prime_u128_montgomery(1));
        assert!(!is_probable_prime_u128_montgomery(4));
        assert!(!is_probable_prime_u128_montgomery(91)); // = 7 * 13
        assert!(!is_probable_prime_u128_montgomery(561)); // Carmichael number = 3 * 11 * 17
    }

    #[test]
    fn test_gcd_u128() {
        assert_eq!(gcd_u128(48, 18), 6);
        assert_eq!(gcd_u128(101, 103), 1);
        assert_eq!(gcd_u128(0, 5), 5);
        assert_eq!(gcd_u128(5, 0), 5);
    }
}
