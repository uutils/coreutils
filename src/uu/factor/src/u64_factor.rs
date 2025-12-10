// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Optimized factorization for numbers that fit in u64
//!
//! Uses hand-tuned operations for maximum performance

/// Fast Pollard-Rho for u64 numbers using Brent's algorithm
#[inline]
pub fn pollard_rho_brent_u64(n: u64) -> Option<u64> {
    if n < 2 {
        return None;
    }

    // Quick checks
    if n % 2 == 0 {
        return Some(2);
    }
    if n % 3 == 0 {
        return Some(3);
    }
    if n % 5 == 0 {
        return Some(5);
    }

    // Use Miller-Rabin for small primes
    if is_probable_prime_u64(n) {
        return None;
    }

    // Increased MAX_ITERATIONS for hard semiprimes (64-bit products of ~32-bit primes)
    // Theoretical: Pollard-Rho needs O(sqrt(p)) iterations where p ~= 2^32
    // So expect up to 2^16 ~= 65k iterations, but with batching we need more room
    // 100M iterations with batch GCD is still fast due to optimization
    const MAX_ITERATIONS: u64 = 100_000_000;

    for attempt in 0..15 {
        // Use attempt number to vary the starting point
        let seed = (n as u128)
            .wrapping_mul(1103515245)
            .wrapping_add(12345 + attempt as u128);
        let x0 = (seed % n as u128) as u64;
        let c_seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let c = ((c_seed % n as u128) as u64).max(1); // Ensure c >= 1

        if let Some(factor) = brent_cycle_find_u64(x0, c, n, MAX_ITERATIONS) {
            if factor > 1 && factor < n {
                return Some(factor);
            }
        }
    }

    None
}

/// Brent's cycle finding for u64 with batch GCD optimization
/// Accumulates differences as products and checks GCD periodically
/// This reduces expensive GCD operations from O(r) to O(1) per batch
#[allow(clippy::many_single_char_names)]
#[inline]
fn brent_cycle_find_u64(x0: u64, c: u64, n: u64, max_iterations: u64) -> Option<u64> {
    let mut x = x0;
    let mut y = x0;
    let mut d;
    let mut r = 1u64;
    let mut q = 1u64;

    // Batch GCD: accumulate products instead of checking GCD every iteration
    // This is the Pollard & Brent optimization: 100 GCDs → 99 mults + 1 GCD
    const BATCH_SIZE: u64 = 100;

    loop {
        // r iterations with batched GCD
        for batch in 0..=(r / BATCH_SIZE) {
            let batch_limit = (batch + 1) * BATCH_SIZE;
            let limit = if batch_limit > r { r } else { batch_limit };
            let start = batch * BATCH_SIZE;

            // Accumulate BATCH_SIZE differences as products (cheap multiplications)
            for _ in start..limit {
                // f(x) = x^2 + c mod n
                x = ((x as u128 * x as u128 + c as u128) % n as u128) as u64;
                let diff = x.abs_diff(y);

                // Accumulate product: q = (q * diff) mod n
                q = ((q as u128 * diff as u128) % n as u128) as u64;

                if q == 0 {
                    q = 1; // Avoid zero in GCD
                }
            }

            // Batch GCD check after BATCH_SIZE iterations
            if batch < r / BATCH_SIZE {
                d = num_integer::gcd(q, n);
                if d > 1 && d < n {
                    return Some(d);
                }
                if d == n {
                    // Failure: GCD collapsed to n, this c value won't work
                    return None;
                }
                // Continue accumulating for next batch
            }
        }

        y = x;
        r *= 2;

        // Final GCD check for this round
        d = num_integer::gcd(q, n);
        if d > 1 && d < n {
            return Some(d);
        }
        if d == n {
            return None; // Failure
        }

        // Reset q for next round
        q = 1u64;

        if r > max_iterations {
            return None; // Too many iterations
        }
    }
}

/// Miller-Rabin for u64 with deterministic bases
#[inline]
pub fn is_probable_prime_u64(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 || n == 3 || n == 5 || n == 7 || n == 11 || n == 13 {
        return true;
    }
    if n % 2 == 0 || n % 3 == 0 || n % 5 == 0 {
        return false;
    }

    // Deterministic Miller-Rabin for 64-bit numbers
    // These bases are sufficient for n < 2^64
    const WITNESSES: [u64; 12] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];

    let mut d = n - 1;
    let mut s = 0;
    while d % 2 == 0 {
        d /= 2;
        s += 1;
    }

    'witness: for &a in &WITNESSES {
        if a >= n {
            continue;
        }

        let mut x = powmod_u64(a, d, n);
        if x == 1 || x == n - 1 {
            continue 'witness;
        }

        for _ in 0..s - 1 {
            x = ((x as u128 * x as u128) % n as u128) as u64;
            if x == n - 1 {
                continue 'witness;
            }
        }

        return false;
    }

    true
}

/// Fast modular exponentiation for u64 (pure Rust)
#[inline]
pub fn powmod_u64(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    if modulus == 1 {
        return 0;
    }

    let mut result = 1u64;
    base %= modulus;

    while exp > 0 {
        if exp & 1 == 1 {
            result = ((result as u128 * base as u128) % modulus as u128) as u64;
        }
        base = ((base as u128 * base as u128) % modulus as u128) as u64;
        exp >>= 1;
    }

    result
}

/// Fast trial division for u64 using wheel factorization
pub fn trial_division_u64(n: &mut u64, max_divisor: u64) -> Vec<u64> {
    let mut factors = Vec::new();

    while *n % 2 == 0 {
        factors.push(2);
        *n /= 2;
    }

    while *n % 3 == 0 {
        factors.push(3);
        *n /= 3;
    }

    // Wheel: check numbers of form 6k ± 1
    let mut divisor = 5;
    let mut add = 2;

    while divisor * divisor <= *n && divisor <= max_divisor {
        while *n % divisor == 0 {
            factors.push(divisor);
            *n /= divisor;
        }

        divisor += add;
        add = 6 - add; // Alternate between +2 and +4
    }

    factors
}
