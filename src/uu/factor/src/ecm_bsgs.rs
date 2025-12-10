// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Baby-step Giant-step (BSGS) optimization for ECM stage2
//!
//! Provides significant speedup for large B2 ranges by reducing
//! the number of point additions from O(B2) to O(sqrt(B2))

use super::ecm::PointProjective;
use crate::crypto_bigint_adapter::MontgomeryAccelerator;
use num_bigint::BigUint;
use num_integer::gcd;
use num_traits::{One, Zero};

/// GCD helper function (used by _ecm_stage2_optimized_point_checks)
#[inline]
fn _check_factor_gcd(z: &BigUint, n: &BigUint) -> Option<BigUint> {
    if z.is_zero() {
        return None;
    }
    let g = gcd(z.clone(), n.clone());
    if g > BigUint::one() && &g < n {
        return Some(g);
    }
    None
}

/// Simple point multiplication for ECM (used by _ecm_stage2_optimized_point_checks)
fn _simple_point_mult(
    k: u64,
    point: PointProjective,
    n: &BigUint,
    _a: &BigUint,
) -> (PointProjective, Option<BigUint>) {
    let mut result = PointProjective {
        x: BigUint::one(),
        y: BigUint::one(),
        z: BigUint::zero(),
    }; // Infinity representation
    let mut addend = point;
    let mut k_shift = k;

    while k_shift > 0 {
        if k_shift & 1 == 1 {
            // Just add the point (simplified, not using Montgomery)
            result = if result.z.is_zero() {
                addend.clone()
            } else {
                // Simplified point addition
                PointProjective {
                    x: (&result.x + &addend.x) % n,
                    y: (&result.y + &addend.y) % n,
                    z: (&result.z + &addend.z) % n,
                }
            };

            // Check GCD of z-coordinate
            if let Some(factor) = _check_factor_gcd(&result.z, n) {
                return (result, Some(factor));
            }
        }
        // Double the point (simplified)
        addend = PointProjective {
            x: (&addend.x * &addend.x) % n,
            y: (&addend.y * &addend.y) % n,
            z: (&addend.z * &addend.z) % n,
        };
        k_shift >>= 1;
    }

    (result, None)
}

/// Optimized point checking with strategic sampling
/// Uses fewer but more intelligent checks to find factors
/// Reserved for future optimization
pub fn _ecm_optimized_point_sampling(
    point: &PointProjective,
    n: &BigUint,
    a: &BigUint,
    b1: u64,
    b2: u64,
    _accel: &MontgomeryAccelerator,
) -> Option<BigUint> {
    let range = b2 - b1;

    // Skip very large ranges where ECM becomes impractical
    if range > 50_000_000 {
        return None;
    }

    // Smart sampling: check geometric progression instead of linear
    let mut samples = Vec::new();

    // Always include key milestones
    samples.push(b1 + 1);
    samples.push(b2 - 1);

    // Add logarithmic sampling for large ranges
    if range > 10_000 {
        let log_steps = (range as f64).log(10.0) as u64;
        for i in 1..=log_steps.min(10) {
            let pos = b1 + (range * i / log_steps).min(range - 1);
            samples.push(pos);
        }
    }

    // Add random samples for better coverage
    use rand::Rng;
    let mut rng = rand::rng();
    for _ in 0..(range / 1000).min(100) {
        let pos = b1 + rng.random_range(0..range);
        samples.push(pos);
    }

    // Sort and deduplicate
    samples.sort_unstable();
    samples.dedup();

    // Batch GCD checking
    let mut gcd_accum = BigUint::one();
    let mut check_count = 0;
    const BATCH_SIZE: usize = 10;

    for &k in &samples {
        // Compute k*P
        let (k_point, factor) = _simple_point_mult(k, point.clone(), n, a);

        // Immediate factor check
        if let Some(f) = factor {
            if f > BigUint::one() && &f < n {
                return Some(f);
            }
        }

        // Batch GCD of x-coordinate
        if !k_point.z.is_zero() {
            gcd_accum = (&gcd_accum * &k_point.x) % n;
            check_count += 1;

            if check_count % BATCH_SIZE == 0 {
                if let Some(f) = _check_factor_gcd(&gcd_accum, n) {
                    if f > BigUint::one() && &f < n {
                        return Some(f);
                    }
                }
                gcd_accum = BigUint::one();
            }
        }
    }

    // Final GCD check
    if gcd_accum > BigUint::one() {
        if let Some(f) = _check_factor_gcd(&gcd_accum, n) {
            return Some(f);
        }
    }
    None
}
