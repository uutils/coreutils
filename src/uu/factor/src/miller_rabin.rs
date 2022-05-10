// * This file is part of the uutils coreutils package.
// *
// * (c) 2020 nicoo <nicoo@debian.org>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

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
    // spell-checker:ignore (names) Steve Worley
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
#[must_use = "Ignoring the output of a primality test."]
pub(crate) enum Result {
    Prime,
    Pseudoprime,
    Composite(u64),
}

impl Result {
    pub(crate) fn is_prime(&self) -> bool {
        *self == Self::Prime
    }
}

// Deterministic Miller-Rabin primality-checking algorithm, adapted to extract
// (some) dividers; it will fail to factor strong pseudoprimes.
#[allow(clippy::many_single_char_names)]
pub(crate) fn test<A: Arithmetic + Basis>(m: A) -> Result {
    use self::Result::*;

    let n = m.modulus();
    debug_assert!(n > 1);
    debug_assert!(n % 2 != 0);

    // n-1 = r 2ⁱ
    let i = (n - 1).trailing_zeros();
    let r = (n - 1) >> i;

    let one = m.one();
    let minus_one = m.minus_one();

    'witness: for _a in A::BASIS.iter() {
        let _a = _a % n;
        if _a == 0 {
            continue;
        }

        let a = m.to_mod(_a);

        // x = a^r mod n
        let mut x = m.pow(a, r);

        if x == one || x == minus_one {
            continue;
        }

        for _ in 1..i {
            let y = m.mul(x, x);
            if y == one {
                return Composite(gcd(m.to_u64(x) - 1, m.modulus()));
            } else if y == minus_one {
                // This basis element is not a witness of `n` being composite.
                // Keep looking.
                continue 'witness;
            }
            x = y;
        }

        return Pseudoprime;
    }

    Prime
}

// Used by build.rs' tests and debug assertions
#[allow(dead_code)]
pub(crate) fn is_prime(n: u64) -> bool {
    if n < 2 {
        false
    } else if n % 2 == 0 {
        n == 2
    } else {
        test::<Montgomery<u64>>(Montgomery::new(n)).is_prime()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numeric::{traits::DoubleInt, Arithmetic, Montgomery};
    use crate::parametrized_check;
    use quickcheck::quickcheck;
    use std::iter;
    const LARGEST_U64_PRIME: u64 = 0xFFFFFFFFFFFFFFC5;

    fn primes() -> impl Iterator<Item = u64> {
        iter::once(2).chain(odd_primes())
    }

    fn odd_primes() -> impl Iterator<Item = u64> {
        use crate::table::{NEXT_PRIME, PRIME_INVERSIONS_U64};
        PRIME_INVERSIONS_U64
            .iter()
            .map(|(p, _, _)| *p)
            .chain(iter::once(NEXT_PRIME))
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
    fn two() {
        assert!(is_prime(2));
    }

    fn first_primes<A: DoubleInt>()
    where
        Montgomery<A>: Basis,
    {
        for p in odd_primes() {
            assert!(
                test(Montgomery::<A>::new(p)).is_prime(),
                "{} reported composite",
                p
            );
        }
    }
    parametrized_check!(first_primes);

    #[test]
    fn one() {
        assert!(!is_prime(1));
    }
    #[test]
    fn zero() {
        assert!(!is_prime(0));
    }

    #[test]
    fn first_composites() {
        for (p, q) in primes().zip(odd_primes()) {
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

    fn small_semiprimes<A: DoubleInt>()
    where
        Montgomery<A>: Basis,
    {
        for p in odd_primes() {
            for q in odd_primes().take_while(|q| *q <= p) {
                let n = p * q;
                let m = Montgomery::<A>::new(n);
                assert!(!test(m).is_prime(), "{} = {} × {} reported prime", n, p, q);
            }
        }
    }
    parametrized_check!(small_semiprimes);

    quickcheck! {
        fn composites(i: u64, j: u64) -> bool {
            // TODO: #1559 factor n > 2^64 - 1
            match i.checked_mul(j) {
                Some(n) => i < 2 || j < 2 || !is_prime(n),
                _ => true,
            }
        }
    }
}
