// spell-checker:ignore (URL) appspot

use crate::numeric::*;

pub(crate) trait Basis {
    const BASIS: &'static [u64];
}

impl Basis for Montgomery<u64> {
    // Small set of bases for the Miller-Rabin prime test, valid for all 64b integers;
    //  discovered by Jim Sinclair on 2011-04-20, see miller-rabin.appspot.com
    #[allow(clippy::unreadable_literal)]
    const BASIS: &'static [u64] = &[2, 325, 9375, 28178, 450775, 9780504, 1795265022];
}

impl Basis for Montgomery<u32> {
    // Small set of bases for the Miller-Rabin prime test, valid for all 32b integers;
    //  discovered by Steve Worley on 2013-05-27, see miller-rabin.appspot.com
    #[allow(clippy::unreadable_literal)]
    const BASIS: &'static [u64] = &[
        4230279247111683200,
        14694767155120705706,
        16641139526367750375,
    ];
}

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
pub(crate) fn test<A: Arithmetic + Basis>(m: A) -> Result {
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

    let one = m.one();
    let minus_one = m.minus_one();

    for _a in A::BASIS.iter() {
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
            if y != one {
                return Pseudoprime;
            };
        }

        if x == one || x == minus_one {
            continue;
        }

        loop {
            let y = m.mul(x, x);
            if y == one {
                return Composite(gcd(m.to_u64(x) - 1, m.modulus()));
            }
            if y == minus_one {
                // This basis element is not a witness of `n` being composite.
                // Keep looking.
                break;
            }
            x = y;
        }
    }

    Prime
}

// Used by build.rs' tests and debug assertions
#[allow(dead_code)]
pub(crate) fn is_prime(n: u64) -> bool {
    if n % 2 == 0 {
        n == 2
    } else {
        test::<Montgomery<u64>>(Montgomery::new(n)).is_prime()
    }
}

#[cfg(test)]
mod tests {
    use super::is_prime;
    const LARGEST_U64_PRIME: u64 = 0xFFFFFFFFFFFFFFC5;

    fn primes() -> impl Iterator<Item = u64> {
        use crate::table::{NEXT_PRIME, P_INVS_U64};
        use std::iter::once;
        once(2)
            .chain(P_INVS_U64.iter().map(|(p, _, _)| *p))
            .chain(once(NEXT_PRIME))
    }

    #[test]
    fn largest_prime() {
        assert!(is_prime(LARGEST_U64_PRIME));
    }

    #[test]
    fn largest_composites() {
        for i in LARGEST_U64_PRIME + 1..=u64::MAX {
            assert!(!is_prime(i), "2⁶⁴ - {} reported prime", u64::MAX - i + 1);
        }
    }

    #[test]
    fn first_primes() {
        for p in primes() {
            assert!(is_prime(p), "{} reported composite", p);
        }
    }

    #[test]
    fn first_composites() {
        assert!(!is_prime(0));
        assert!(!is_prime(1));

        for (p, q) in primes().zip(primes().skip(1)) {
            for i in p + 1..q {
                assert!(!is_prime(i), "{} reported prime", i);
            }
        }
    }

    #[test]
    fn issue_1556() {
        // 10 425 511 = 2441 × 4271
        assert!(!is_prime(10_425_511));
    }

    #[test]
    fn small_composites() {
        for p in primes() {
            for q in primes().take_while(|q| *q <= p) {
                let n = p * q;
                assert!(!is_prime(n), "{} = {} × {} reported prime", n, p, q);
            }
        }
    }
}
