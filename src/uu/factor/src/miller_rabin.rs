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
pub(crate) fn test<A: Arithmetic>(n: u64) -> Result {
    use self::Result::*;

    if n < 2 {
        return Pseudoprime;
    }
    if n % 2 == 0 {
        return if n == 2 { Prime } else { Composite(2) };
    }

    let r = (n - 1) >> (n - 1).trailing_zeros();

    for a in BASIS.iter() {
        let mut x = a % n;
        if x == 0 {
            break;
        }

        if A::pow(x, n - 1, n) != 1 {
            return Pseudoprime;
        }
        x = A::pow(x, r, n);
        if x == 1 || x == n - 1 {
            break;
        }

        loop {
            let y = A::mul(x, x, n);
            if y == 1 {
                return Composite(gcd(x - 1, n));
            }
            if y == n - 1 {
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
    if n < 1 << 63 {
        test::<Small>(n)
    } else {
        test::<Big>(n)
    }
    .is_prime()
}
