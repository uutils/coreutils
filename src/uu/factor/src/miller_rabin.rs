// spell-checker:ignore (URL) appspot

use crate::numeric::*;

// Small set of bases for the Miller-Rabin prime test, valid for all 64b integers;
//  discovered by Jim Sinclair on 2011-04-20, see miller-rabin.appspot.com
#[allow(clippy::unreadable_literal)]
const BASIS: [u64; 7] = [2, 325, 9375, 28178, 450775, 9780504, 1795265022];

#[derive(Eq, PartialEq)]
pub(crate) enum Result {
    Prime,
    Pseudoprime,
    Composite(u64),
}

impl Result {
    pub(crate) fn is_prime(&self) -> bool {
        *self == Result::Prime
    }
}

// Deterministic Miller-Rabin primality-checking algorithm, adapted to extract
// (some) dividers; it will fail to factor strong pseudoprimes.
#[allow(clippy::many_single_char_names)]
pub(crate) fn test<A: Arithmetic>(m: A) -> Result {
    use self::Result::*;

    let n = m.modulus();
    if n < 2 {
        return Pseudoprime;
    }
    if n % 2 == 0 {
        return if n == 2 { Prime } else { Composite(2) };
    }

    // n-1 = r 2ⁱ
    let i = (n - 1).trailing_zeros();
    let r = (n - 1) >> i;

    for _a in BASIS.iter() {
        let _a = _a % n;
        if _a == 0 {
            break;
        }

        let a = m.from_u64(_a);

        // x = a^r mod n
        let mut x = m.pow(a, r);

        {
            // y = ((x²)²...)² i times = x ^ (2ⁱ) = a ^ (r 2ⁱ) = x ^ (n - 1)
            let mut y = x;
            for _ in 0..i {
                y = m.mul(y, y)
            }
            if y != m.one() {
                return Pseudoprime;
            };
        }

        if x == m.one() || x == m.minus_one() {
            break;
        }

        loop {
            let y = m.mul(x, x);
            if y == m.one() {
                return Composite(gcd(m.to_u64(x) - 1, m.modulus()));
            }
            if y == m.minus_one() {
                // This basis element is not a witness of `n` being composite.
                // Keep looking.
                break;
            }
            x = y;
        }
    }

    Prime
}

// Used by build.rs' tests
#[allow(dead_code)]
pub(crate) fn is_prime(n: u64) -> bool {
    test::<Montgomery>(Montgomery::new(n)).is_prime()
}

#[cfg(test)]
mod tests {
    use super::is_prime;
    const LARGEST_U64_PRIME: u64 = 0xFFFFFFFFFFFFFFC5;

    #[test]
    fn largest_prime() {
        assert!(is_prime(LARGEST_U64_PRIME));
    }

    #[test]
    fn first_primes() {
        use crate::table::{NEXT_PRIME, P_INVS_U64};
        for (p, _, _) in P_INVS_U64.iter() {
            assert!(is_prime(*p), "{} reported composite", p);
        }
        assert!(is_prime(NEXT_PRIME));
    }
}
