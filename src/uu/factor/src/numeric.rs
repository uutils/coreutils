// * This file is part of the uutils coreutils package.
// *
// * (c) Wiktor Kuropatwa <wiktor.kuropatwa@gmail.com>
// * (c) kwantam <kwantam@gmail.com>
// *     * 20150507 ~ added big_ routines to prevent overflow when num > 2^63
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

use std::mem::swap;

pub fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b > 0 {
        a %= b;
        swap(&mut a, &mut b);
    }
    a
}

pub(crate) trait Arithmetic: Copy + Sized {
    type I: Copy + Sized + Eq;

    fn new(m: u64) -> Self;
    fn modulus(&self) -> u64;
    fn from_u64(&self, n: u64) -> Self::I;
    fn to_u64(&self, n: Self::I) -> u64;
    fn add(&self, a: Self::I, b: Self::I) -> Self::I;
    fn mul(&self, a: Self::I, b: Self::I) -> Self::I;

    fn pow(&self, mut a: Self::I, mut b: u64) -> Self::I {
        let (_a, _b) = (a, b);
        let mut result = self.one();
        while b > 0 {
            if b & 1 != 0 {
                result = self.mul(result, a);
            }
            a = self.mul(a, a);
            b >>= 1;
        }

        // Check that r (reduced back to the usual representation) equals
        //  a^b % n, unless the latter computation overflows
        // Temporarily commented-out, as there u64::checked_pow is not available
        //  on the minimum supported Rust version, nor is an appropriate method
        //  for compiling the check conditionally.
        //debug_assert!(self
        //    .to_u64(_a)
        //    .checked_pow(_b as u32)
        //    .map(|r| r % self.modulus() == self.to_u64(result))
        //    .unwrap_or(true));

        result
    }

    fn one(&self) -> Self::I {
        self.from_u64(1)
    }
    fn minus_one(&self) -> Self::I {
        self.from_u64(self.modulus() - 1)
    }
    fn zero(&self) -> Self::I {
        self.from_u64(0)
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Montgomery {
    a: u64,
    n: u64,
}

impl Montgomery {
    /// computes x/R mod n efficiently
    fn reduce(&self, x: u64) -> u64 {
        // TODO: optimiiiiiiise
        let Montgomery { a, n } = self;
        let t = x.wrapping_mul(*a);
        let nt = (*n as u128) * (t as u128);
        let y = ((x as u128 + nt) >> 64) as u64;
        if y >= *n {
            y - n
        } else {
            y
        }
    }
}

impl Arithmetic for Montgomery {
    // Montgomery transform, R=2⁶⁴
    // Provides fast arithmetic mod n (n odd, u64)
    type I = u64;

    fn new(n: u64) -> Self {
        let a = inv_mod_u64(n).wrapping_neg();
        debug_assert_eq!(n.wrapping_mul(a), 1_u64.wrapping_neg());
        Montgomery { a, n }
    }

    fn modulus(&self) -> u64 {
        self.n
    }

    fn from_u64(&self, x: u64) -> Self::I {
        // TODO: optimise!
        assert!(x < self.n);
        let r = (((x as u128) << 64) % self.n as u128) as u64;
        debug_assert_eq!(x, self.to_u64(r));
        r
    }

    fn to_u64(&self, n: Self::I) -> u64 {
        self.reduce(n)
    }

    fn add(&self, a: Self::I, b: Self::I) -> Self::I {
        let r = a + b;

        // Check that r (reduced back to the usual representation) equals
        // a+b % n
        #[cfg(debug_assertions)]
        {
            let a_r = self.to_u64(a);
            let b_r = self.to_u64(b);
            let r_r = self.to_u64(r);
            let r_2 = (((a_r as u128) + (b_r as u128)) % (self.n as u128)) as u64;
            debug_assert_eq!(
                r_r, r_2,
                "[{}] = {} ≠ {} = {} + {} = [{}] + [{}] mod {}; a = {}",
                r, r_r, r_2, a_r, b_r, a, b, self.n, self.a
            );
        }
        r
    }

    fn mul(&self, a: Self::I, b: Self::I) -> Self::I {
        let r = self.reduce(a.wrapping_mul(b));

        // Check that r (reduced back to the usual representation) equals
        // a*b % n
        #[cfg(debug_assertions)]
        {
            let a_r = self.to_u64(a);
            let b_r = self.to_u64(b);
            let r_r = self.to_u64(r);
            let r_2 = (((a_r as u128) * (b_r as u128)) % (self.n as u128)) as u64;
            debug_assert_eq!(
                r_r, r_2,
                "[{}] = {} ≠ {} = {} * {} = [{}] * [{}] mod {}; a = {}",
                r, r_r, r_2, a_r, b_r, a, b, self.n, self.a
            );
        }
        r
    }
}

// extended Euclid algorithm
// precondition: a is odd
pub(crate) fn inv_mod_u64(a: u64) -> u64 {
    assert!(a % 2 == 1);
    let mut t = 0u64;
    let mut newt = 1u64;
    let mut r = 0u64;
    let mut newr = a;

    while newr != 0 {
        let quot = if r == 0 {
            // special case when we're just starting out
            // This works because we know that
            // a does not divide 2^64, so floor(2^64 / a) == floor((2^64-1) / a);
            std::u64::MAX
        } else {
            r
        } / newr;

        let newtp = t.wrapping_sub(quot.wrapping_mul(newt));
        t = newt;
        newt = newtp;

        let newrp = r.wrapping_sub(quot.wrapping_mul(newr));
        r = newr;
        newr = newrp;
    }

    assert_eq!(r, 1);
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inverter() {
        // All odd integers from 1 to 20 000
        let mut test_values = (0..10_000u64).map(|i| 2 * i + 1);

        assert!(test_values.all(|x| x.wrapping_mul(inv_mod_u64(x)) == 1));
    }

    #[test]
    fn test_montgomery_add() {
        for n in 0..100 {
            let n = 2 * n + 1;
            let m = Montgomery::new(n);
            for x in 0..n {
                let m_x = m.from_u64(x);
                for y in 0..=x {
                    let m_y = m.from_u64(y);
                    println!("{n:?}, {x:?}, {y:?}", n = n, x = x, y = y);
                    assert_eq!((x + y) % n, m.to_u64(m.add(m_x, m_y)));
                }
            }
        }
    }

    #[test]
    fn test_montgomery_mult() {
        for n in 0..100 {
            let n = 2 * n + 1;
            let m = Montgomery::new(n);
            for x in 0..n {
                let m_x = m.from_u64(x);
                for y in 0..=x {
                    let m_y = m.from_u64(y);
                    assert_eq!((x * y) % n, m.to_u64(m.mul(m_x, m_y)));
                }
            }
        }
    }

    #[test]
    fn test_montgomery_roundtrip() {
        for n in 0..100 {
            let n = 2 * n + 1;
            let m = Montgomery::new(n);
            for x in 0..n {
                let x_ = m.from_u64(x);
                assert_eq!(x, m.to_u64(x_));
            }
        }
    }
}
