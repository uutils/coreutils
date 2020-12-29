// * This file is part of the uutils coreutils package.
// *
// * (c) 2020 nicoo <nicoo@debian.org>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

use smallvec::SmallVec;
use std::cell::RefCell;
use std::fmt;

use crate::numeric::{gcd, Arithmetic, Montgomery};
use crate::{miller_rabin, rho, table};

type Exponent = u8;

#[derive(Clone, Debug)]
struct Decomposition(SmallVec<[(u64, Exponent); NUM_FACTORS_INLINE]>);

// The number of factors to inline directly into a `Decomposition` object.
// As a consequence of the Erdős–Kac theorem, the average number of prime factors
// of integers < 10²⁵ ≃ 2⁸³ is 4, so we can use a slightly higher value.
const NUM_FACTORS_INLINE: usize = 5;

impl Decomposition {
    fn one() -> Decomposition {
        Decomposition(SmallVec::new())
    }

    fn add(&mut self, factor: u64, exp: Exponent) {
        debug_assert!(exp > 0);
        // Assert the factor doesn't already exist in the Decomposition object
        debug_assert_eq!(self.0.iter_mut().find(|(f, _)| *f == factor), None);

        self.0.push((factor, exp))
    }

    fn is_one(&self) -> bool {
        self.0.is_empty()
    }

    fn pop(&mut self) -> Option<(u64, Exponent)> {
        self.0.pop()
    }

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
    fn eq(&self, other: &Decomposition) -> bool {
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
    pub fn one() -> Factors {
        Factors(RefCell::new(Decomposition::one()))
    }

    pub fn add(&mut self, prime: u64, exp: Exponent) {
        debug_assert!(miller_rabin::is_prime(prime));
        self.0.borrow_mut().add(prime, exp)
    }

    #[cfg(test)]
    pub fn push(&mut self, prime: u64) {
        self.add(prime, 1)
    }

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
                write!(f, " {}", p)?
            }
        }

        Ok(())
    }
}

fn _find_factor<A: Arithmetic + miller_rabin::Basis>(num: u64) -> Option<u64> {
    use miller_rabin::Result::*;

    let n = A::new(num);
    match miller_rabin::test::<A>(n) {
        Prime => None,
        Composite(d) => Some(d),
        Pseudoprime => Some(rho::find_divisor::<A>(n)),
    }
}

fn find_factor(num: u64) -> Option<u64> {
    if num < (1 << 32) {
        _find_factor::<Montgomery<u32>>(num)
    } else {
        _find_factor::<Montgomery<u64>>(num)
    }
}

pub fn factor(num: u64) -> Factors {
    let mut factors = Factors::one();

    if num < 2 {
        return factors;
    }

    let mut n = num;
    let n_zeros = num.trailing_zeros();
    if n_zeros > 0 {
        factors.add(2, n_zeros as Exponent);
        n >>= n_zeros;
    }
    debug_assert_eq!(num, n * factors.product());

    if n == 1 {
        return factors;
    }

    table::factor(&mut n, &mut factors);
    debug_assert_eq!(num, n * factors.product());

    if n == 1 {
        return factors;
    }

    let mut dec = Decomposition::one();
    dec.add(n, 1);

    while !dec.is_one() {
        // Check correctness invariant
        debug_assert_eq!(num, factors.product() * dec.product());

        let (factor, exp) = dec.pop().unwrap();

        if let Some(divisor) = find_factor(factor) {
            let mut gcd_queue = Decomposition::one();

            let quotient = factor / divisor;
            let mut trivial_gcd = quotient == divisor;
            if trivial_gcd {
                gcd_queue.add(divisor, exp + 1);
            } else {
                gcd_queue.add(divisor, exp);
                gcd_queue.add(quotient, exp);
            }

            while !trivial_gcd {
                debug_assert_eq!(factor, gcd_queue.product());

                let mut tmp = Decomposition::one();
                trivial_gcd = true;
                for i in 0..gcd_queue.0.len() - 1 {
                    let (mut a, exp_a) = gcd_queue.0[i];
                    let (mut b, exp_b) = gcd_queue.0[i + 1];

                    if a == 1 {
                        continue;
                    }

                    let g = gcd(a, b);
                    if g != 1 {
                        trivial_gcd = false;
                        a /= g;
                        b /= g;
                    }
                    if a != 1 {
                        tmp.add(a, exp_a);
                    }
                    if g != 1 {
                        tmp.add(g, exp_a + exp_b);
                    }

                    if i + 1 != gcd_queue.0.len() - 1 {
                        gcd_queue.0[i + 1].0 = b;
                    } else if b != 1 {
                        tmp.add(b, exp_b);
                    }
                }
                gcd_queue = tmp;
            }

            debug_assert_eq!(factor, gcd_queue.product());
            dec.0.extend(gcd_queue.0);
        } else {
            // factor is prime
            factors.add(factor, exp);
        }
    }

    factors
}

#[cfg(test)]
mod tests {
    use super::{factor, Factors};
    use quickcheck::quickcheck;

    #[test]
    fn factor_correctly_recombines_prior_test_failures() {
        let prior_failures = [
            // * integers with duplicate factors (ie, N.pow(M))
            4566769_u64, // == 2137.pow(2)
            2044854919485649_u64,
            18446739546814299361_u64,
            18446738440860217487_u64,
            18446736729316206481_u64,
        ];
        assert!(prior_failures.iter().all(|i| factor(*i).product() == *i));
    }

    #[test]
    fn factor_recombines_small() {
        assert!((1..10_000)
            .map(|i| 2 * i + 1)
            .all(|i| factor(i).product() == i));
    }

    #[test]
    fn factor_recombines_small_squares() {
        // factor(18446736729316206481) == 4294966441 ** 2 ; causes debug_assert fault for repeated decomposition factor in add()
        // ToDO: explain/combine with factor_18446736729316206481 and factor_18446739546814299361 tests
        assert!((1..10_000)
            .map(|i| (2 * i + 1) * (2 * i + 1))
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

        fn recombines_factors(f: Factors) -> bool {
            assert_eq!(factor(f.product()), f);
            true
        }
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for Factors {
    fn arbitrary<G: quickcheck::Gen>(gen: &mut G) -> Self {
        use rand::Rng;
        let mut f = Factors::one();
        let mut g = 1u64;
        let mut n = u64::MAX;

        // Adam Kalai's algorithm for generating uniformly-distributed
        // integers and their factorization.
        //
        // See Generating Random Factored Numbers, Easily, J. Cryptology (2003)
        'attempt: loop {
            while n > 1 {
                n = gen.gen_range(1, n);
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
