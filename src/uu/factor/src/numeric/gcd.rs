// * This file is part of the uutils coreutils package.
// *
// * (c) 2015 Wiktor Kuropatwa <wiktor.kuropatwa@gmail.com>
// * (c) 2020 nicoo            <nicoo@debian.org>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

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

    // Extract common factor-2: gcd(2ⁱ n, 2ⁱ m) = 2ⁱ gcd(n, m)
    // and reducing until odd gcd(2ⁱ n, m) = gcd(n, m) if m is odd
    let k = {
        let i = u.trailing_zeros();
        let j = v.trailing_zeros();
        u >>= i;
        v >>= j;
        min(i, j)
    };

    loop {
        // Invariant: u odd
        debug_assert!(u % 2 == 1, "u = {} is even", u);

        if u > v {
            swap(&mut u, &mut v);
        }
        v -= u;

        if v == 0 {
            return u << k;
        }

        v >>= v.trailing_zeros();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::quickcheck;

    quickcheck! {
        fn gcd(a: u64, b: u64) -> bool {
            // Test against the Euclidean algorithm
            let g = {
                let (mut a, mut b) = (a, b);
                while b > 0 {
                    a %= b;
                    swap(&mut a, &mut b);
                }
                a
            };
            super::gcd(a, b) == g
        }
    }
}
