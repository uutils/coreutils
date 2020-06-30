// * This file is part of the uutils coreutils package.
// *
// * (c) 2020 nicoo <nicoo@debian.org>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

extern crate rand;

use std::collections::BTreeMap;
use std::fmt;

use crate::numeric::{Arithmetic, Montgomery};
use crate::{miller_rabin, rho, table};

pub struct Factors {
    f: BTreeMap<u64, u8>,
}

impl Factors {
    pub fn one() -> Factors {
        Factors { f: BTreeMap::new() }
    }

    pub fn prime(p: u64) -> Factors {
        let mut f = Factors::one();
        f.push(p);
        f
    }

    pub fn add(&mut self, prime: u64, exp: u8) {
        debug_assert!(miller_rabin::is_prime(prime));
        debug_assert!(exp > 0);
        let n = *self.f.get(&prime).unwrap_or(&0);
        self.f.insert(prime, exp + n);
    }

    pub fn push(&mut self, prime: u64) {
        self.add(prime, 1)
    }

    #[cfg(test)]
    fn product(&self) -> u64 {
        self.f
            .iter()
            .fold(1, |acc, (p, exp)| acc * p.pow(*exp as u32))
    }
}

impl fmt::Display for Factors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (p, exp) in self.f.iter() {
            for _ in 0..*exp {
                write!(f, " {}", p)?
            }
        }

        Ok(())
    }
}

fn _factor<A: Arithmetic>(num: u64, f: Factors) -> Factors {
    use miller_rabin::Result::*;
    // Shadow the name, so the recursion automatically goes from “Big” arithmetic to small.
    let _factor = |n, f| {
        // TODO: Optimise with 32 and 64b versions
        _factor::<A>(n, f)
    };

    if num == 1 {
        return f;
    }

    let n = A::new(num);
    let divisor = match miller_rabin::test::<A>(n) {
        Prime => {
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
    let mut factors = Factors::one();

    if n < 2 {
        factors.push(n);
        return factors;
    }

    let z = n.trailing_zeros();
    if z > 0 {
        factors.add(2, z as u8);
        n >>= z;
    }

    if n == 1 {
        return factors;
    }

    let (factors, n) = table::factor(n, factors);

    if n < (1 << 32) {
        _factor::<Montgomery>(n, factors)
    } else {
        _factor::<Montgomery>(n, factors)
    }
}

#[cfg(test)]
mod tests {
    use super::factor;

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
        //  and triggered a bug in rho::factor's codepath handling
        //  miller_rabbin::Result::Composite
        let pseudoprime = 17179869183;
        for _ in 0..20 {
            // Repeat the test 20 times, as it only fails some fraction
            // of the time.
            assert!(factor(pseudoprime).product() == pseudoprime);
        }
    }
}
