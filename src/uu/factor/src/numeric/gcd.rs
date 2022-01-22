// * This file is part of the uutils coreutils package.
// *
// * (c) 2015 Wiktor Kuropatwa <wiktor.kuropatwa@gmail.com>
// * (c) 2020 nicoo            <nicoo@debian.org>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

// spell-checker:ignore (vars) kgcdab gcdac gcdbc

use std::cmp::min;
use std::mem::swap;

pub fn gcd(mut u: u64, mut v: u64) -> u64 {
    // Stein's binary GCD algorithm
    // Base cases: gcd(n, 0) = gcd(0, n) = n
    if u == 0 {
        return v;
    } else if v == 0 {
        return u;
    }

    // gcd(2ⁱ u, 2ʲ v) = 2ᵏ gcd(u, v) with u, v odd and k = min(i, j)
    // 2ᵏ is the greatest power of two that divides both u and v
    let k = {
        let i = u.trailing_zeros();
        let j = v.trailing_zeros();
        u >>= i;
        v >>= j;
        min(i, j)
    };

    loop {
        // Loop invariant: u and v are odd
        debug_assert!(u % 2 == 1, "u = {} is even", u);
        debug_assert!(v % 2 == 1, "v = {} is even", v);

        // gcd(u, v) = gcd(|u - v|, min(u, v))
        if u > v {
            swap(&mut u, &mut v);
        }
        v -= u;

        if v == 0 {
            // Reached the base case; gcd is 2ᵏ u
            return u << k;
        }

        // gcd(u, 2ʲ v) = gcd(u, v) as u is odd
        v >>= v.trailing_zeros();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{quickcheck, TestResult};

    quickcheck! {
        fn euclidean(a: u64, b: u64) -> bool {
            // Test against the Euclidean algorithm
            let g = {
                let (mut a, mut b) = (a, b);
                while b > 0 {
                    a %= b;
                    swap(&mut a, &mut b);
                }
                a
            };
            gcd(a, b) == g
        }

        fn one(a: u64) -> bool {
            gcd(1, a) == 1
        }

        fn zero(a: u64) -> bool {
            gcd(0, a) == a
        }

        fn divisor(a: u64, b: u64) -> TestResult {
            // Test that gcd(a, b) divides a and b, unless a == b == 0
            if a == 0 && b == 0 { return TestResult::discard(); } // restrict test domain to !(a == b == 0)

            let g = gcd(a, b);
            TestResult::from_bool( g != 0 && a % g == 0 && b % g == 0 )
        }

        fn commutative(a: u64, b: u64) -> bool {
            gcd(a, b) == gcd(b, a)
        }

        fn associative(a: u64, b: u64, c: u64) -> bool {
            gcd(a, gcd(b, c)) == gcd(gcd(a, b), c)
        }

        fn scalar_multiplication(a: u64, b: u64, k: u64) -> bool {
            // TODO: #1559 factor n > 2^64 - 1
            match (k.checked_mul(a), k.checked_mul(b), k.checked_mul(gcd(a, b))) {
                (Some(ka), Some(kb), Some(kgcdab)) => gcd(ka, kb) == kgcdab,
                _ => true
            }
        }

        fn multiplicative(a: u64, b: u64, c: u64) -> bool {
            // TODO: #1559 factor n > 2^64 - 1
            match (a.checked_mul(b), gcd(a, c).checked_mul(gcd(b, c))) {
                (Some(ab), Some(gcdac_gcdbc)) => {
                    // gcd(ab, c) = gcd(a, c) gcd(b, c) when a and b coprime
                    gcd(a, b) != 1 || gcd(ab, c) == gcdac_gcdbc
                },
                _ => true,
            }
        }

        fn linearity(a: u64, b: u64, k: u64) -> bool {
            // TODO: #1559 factor n > 2^64 - 1
            match k.checked_mul(b) {
                Some(kb) => {
                    match a.checked_add(kb) {
                        Some(a_plus_kb) => gcd(a_plus_kb, b) == gcd(a, b),
                        _ => true,
                    }
                }
                _ => true,
            }
        }
    }
}
