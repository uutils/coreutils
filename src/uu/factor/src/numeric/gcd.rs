// * This file is part of the uutils coreutils package.
// *
// * (c) 2015 Wiktor Kuropatwa <wiktor.kuropatwa@gmail.com>
// * (c) 2020 nicoo            <nicoo@debian.org>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

use std::cmp::min;
use std::mem::swap;

pub fn gcd(mut n: u64, mut m: u64) -> u64 {
    // Stein's binary GCD algorithm
    // Base cases: gcd(n, 0) = gcd(0, n) = n
    if n == 0 {
        return m;
    } else if m == 0 {
        return n;
    }

    // Extract common factor-2: gcd(2ⁱ n, 2ⁱ m) = 2ⁱ gcd(n, m)
    // and reducing until odd gcd(2ⁱ n, m) = gcd(n, m) if m is odd
    let k = {
        let k_n = n.trailing_zeros();
        let k_m = m.trailing_zeros();
        n >>= k_n;
        m >>= k_m;
        min(k_n, k_m)
    };

    loop {
        // Invariant: n odd
        debug_assert!(n % 2 == 1, "n = {} is even", n);

        if n > m {
            swap(&mut n, &mut m);
        }
        m -= n;

        if m == 0 {
            return n << k;
        }

        m >>= m.trailing_zeros();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::quickcheck;

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
    }
}
