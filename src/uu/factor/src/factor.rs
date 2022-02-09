// * This file is part of the uutils coreutils package.
// *
// * (c) 2020 nicoo <nicoo@debian.org>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

use smallvec::SmallVec;
use std::cell::RefCell;
use std::fmt;

use crate::numeric::{Arithmetic, Montgomery};
use crate::{miller_rabin, rho, table};

type Exponent = u8;

#[derive(Clone, Debug, Default)]
struct Decomposition(SmallVec<[(u64, Exponent); NUM_FACTORS_INLINE]>);

// spell-checker:ignore (names) Erdős–Kac * Erdős Kac
// The number of factors to inline directly into a `Decomposition` object.
// As a consequence of the Erdős–Kac theorem, the average number of prime factors
// of integers < 10²⁵ ≃ 2⁸³ is 4, so we can use a slightly higher value.
const NUM_FACTORS_INLINE: usize = 5;

impl Decomposition {
    fn one() -> Self {
        Self::default()
    }

    fn add(&mut self, factor: u64, exp: Exponent) {
        debug_assert!(exp > 0);

        if let Some((_, e)) = self.0.iter_mut().find(|(f, _)| *f == factor) {
            *e += exp;
        } else {
            self.0.push((factor, exp));
        }
    }

    #[cfg(test)]
    fn product(&self) -> u64 {
        self.0
            .iter()
            .fold(1, |acc, (p, exp)| acc * p.pow(*exp as u32))
    }

    fn get(&self, p: u64) -> Option<&(u64, u8)> {
        self.0.iter().find(|(q, _)| *q == p)
    }
}

impl PartialEq for Decomposition {
    fn eq(&self, other: &Self) -> bool {
        for p in &self.0 {
            if other.get(p.0) != Some(p) {
                return false;
            }
        }

        for p in &other.0 {
            if self.get(p.0) != Some(p) {
                return false;
            }
        }

        true
    }
}
impl Eq for Decomposition {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Factors(RefCell<Decomposition>);

impl Factors {
    pub fn one() -> Self {
        Self(RefCell::new(Decomposition::one()))
    }

    pub fn add(&mut self, prime: u64, exp: Exponent) {
        debug_assert!(miller_rabin::is_prime(prime));
        self.0.borrow_mut().add(prime, exp);
    }

    pub fn push(&mut self, prime: u64) {
        self.add(prime, 1);
    }

    #[cfg(test)]
    fn product(&self) -> u64 {
        self.0.borrow().product()
    }
}

impl fmt::Display for Factors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v = &mut (self.0).borrow_mut().0;
        v.sort_unstable();

        for (p, exp) in v.iter() {
            for _ in 0..*exp {
                write!(f, " {}", p)?;
            }
        }

        Ok(())
    }
}

fn _factor<A: Arithmetic + miller_rabin::Basis>(num: u64, f: Factors) -> Factors {
    use miller_rabin::Result::*;

    // Shadow the name, so the recursion automatically goes from “Big” arithmetic to small.
    let _factor = |n, f| {
        if n < (1 << 32) {
            _factor::<Montgomery<u32>>(n, f)
        } else {
            _factor::<A>(n, f)
        }
    };

    if num == 1 {
        return f;
    }

    let n = A::new(num);
    let divisor = match miller_rabin::test::<A>(n) {
        Prime => {
            #[cfg(feature = "coz")]
            coz::progress!("factor found");
            let mut r = f;
            r.push(num);
            return r;
        }

        Composite(d) => d,
        Pseudoprime => rho::find_divisor::<A>(n),
    };

    let f = _factor(divisor, f);
    _factor(num / divisor, f)
}

pub fn factor(mut n: u64) -> Factors {
    #[cfg(feature = "coz")]
    coz::begin!("factorization");
    let mut factors = Factors::one();

    if n < 2 {
        return factors;
    }

    let n_zeros = n.trailing_zeros();
    if n_zeros > 0 {
        factors.add(2, n_zeros as Exponent);
        n >>= n_zeros;
    }

    if n == 1 {
        #[cfg(feature = "coz")]
        coz::end!("factorization");
        return factors;
    }

    table::factor(&mut n, &mut factors);

    #[allow(clippy::let_and_return)]
    let r = if n < (1 << 32) {
        _factor::<Montgomery<u32>>(n, factors)
    } else {
        _factor::<Montgomery<u64>>(n, factors)
    };

    #[cfg(feature = "coz")]
    coz::end!("factorization");

    r
}

#[cfg(test)]
mod tests {
    use super::{factor, Decomposition, Exponent, Factors};
    use quickcheck::quickcheck;
    use smallvec::smallvec;
    use std::cell::RefCell;

    #[test]
    fn factor_2044854919485649() {
        let f = Factors(RefCell::new(Decomposition(smallvec![
            (503, 1),
            (2423, 1),
            (40961, 2)
        ])));
        assert_eq!(factor(f.product()), f);
    }

    #[test]
    fn factor_recombines_small() {
        assert!((1..10_000)
            .map(|i| 2 * i + 1)
            .all(|i| factor(i).product() == i));
    }

    #[test]
    fn factor_recombines_overflowing() {
        assert!((0..250)
            .map(|i| 2 * i + 2u64.pow(32) + 1)
            .all(|i| factor(i).product() == i));
    }

    #[test]
    fn factor_recombines_strong_pseudoprime() {
        // This is a strong pseudoprime (wrt. miller_rabin::BASIS)
        //  and triggered a bug in rho::factor's code path handling
        //  miller_rabbin::Result::Composite
        let pseudoprime = 17179869183;
        for _ in 0..20 {
            // Repeat the test 20 times, as it only fails some fraction
            // of the time.
            assert!(factor(pseudoprime).product() == pseudoprime);
        }
    }

    quickcheck! {
        fn factor_recombines(i: u64) -> bool {
            i == 0 || factor(i).product() == i
        }

        fn recombines_factors(f: Factors) -> () {
            assert_eq!(factor(f.product()), f);
        }

        fn exponentiate_factors(f: Factors, e: Exponent) -> () {
            if e == 0 { return; }
            if let Some(fe) = f.product().checked_pow(e.into()) {
                assert_eq!(factor(fe), f ^ e);
            }
        }
    }
}

#[cfg(test)]
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
#[cfg(test)]
impl Distribution<Factors> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Factors {
        let mut f = Factors::one();
        let mut g = 1u64;
        let mut n = u64::MAX;

        // spell-checker:ignore (names) Adam Kalai * Kalai's
        // Adam Kalai's algorithm for generating uniformly-distributed
        // integers and their factorization.
        //
        // See Generating Random Factored Numbers, Easily, J. Cryptology (2003)
        'attempt: loop {
            while n > 1 {
                n = rng.gen_range(1..n);
                if miller_rabin::is_prime(n) {
                    if let Some(h) = g.checked_mul(n) {
                        f.push(n);
                        g = h;
                    } else {
                        // We are overflowing u64, retry
                        continue 'attempt;
                    }
                }
            }

            return f;
        }
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for Factors {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        factor(u64::arbitrary(g))
    }
}

#[cfg(test)]
impl std::ops::BitXor<Exponent> for Factors {
    type Output = Self;

    fn bitxor(self, rhs: Exponent) -> Self {
        debug_assert_ne!(rhs, 0);
        let mut r = Self::one();
        for (p, e) in self.0.borrow().0.iter() {
            r.add(*p, rhs * e);
        }

        debug_assert_eq!(r.product(), self.product().pow(rhs.into()));
        r
    }
}
