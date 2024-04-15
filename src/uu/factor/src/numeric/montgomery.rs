// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use super::*;

use traits::{DoubleInt, Int, One, OverflowingAdd, Zero};

pub(crate) trait Arithmetic: Copy + Sized {
    // The type of integers mod m, in some opaque representation
    type ModInt: Copy + Sized + Eq;

    fn new(m: u64) -> Self;
    fn modulus(&self) -> u64;
    fn to_mod(&self, n: u64) -> Self::ModInt;
    fn to_u64(&self, n: Self::ModInt) -> u64;
    fn add(&self, a: Self::ModInt, b: Self::ModInt) -> Self::ModInt;
    fn mul(&self, a: Self::ModInt, b: Self::ModInt) -> Self::ModInt;

    fn pow(&self, mut a: Self::ModInt, mut b: u64) -> Self::ModInt {
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
        debug_assert!(self
            .to_u64(_a)
            .checked_pow(_b as u32)
            .map(|r| r % self.modulus() == self.to_u64(result))
            .unwrap_or(true));

        result
    }

    fn one(&self) -> Self::ModInt {
        self.to_mod(1)
    }

    fn minus_one(&self) -> Self::ModInt {
        self.to_mod(self.modulus() - 1)
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Montgomery<T: DoubleInt> {
    a: T,
    n: T,
}

impl<T: DoubleInt> Montgomery<T> {
    /// computes x/R mod n efficiently
    fn reduce(&self, x: T::DoubleWidth) -> T {
        let t_bits = T::zero().count_zeros() as usize;

        debug_assert!(x < (self.n.as_double_width()) << t_bits);
        // TODO: optimize
        let Self { a, n } = self;
        let m = T::from_double_width(x).wrapping_mul(a);
        let nm = (n.as_double_width()) * (m.as_double_width());
        let (xnm, overflow) = x.overflowing_add(&nm); // x + n*m
        debug_assert_eq!(
            xnm % (T::DoubleWidth::one() << T::zero().count_zeros() as usize),
            T::DoubleWidth::zero()
        );

        // (x + n*m) / R
        // in case of overflow, this is (2¹²⁸ + xnm)/2⁶⁴ - n = xnm/2⁶⁴ + (2⁶⁴ - n)
        let y = T::from_double_width(xnm >> t_bits)
            + if overflow {
                n.wrapping_neg()
            } else {
                T::zero()
            };

        if y >= *n {
            y - *n
        } else {
            y
        }
    }
}

impl<T: DoubleInt> Arithmetic for Montgomery<T> {
    // Montgomery transform, R=2⁶⁴
    // Provides fast arithmetic mod n (n odd, u64)
    type ModInt = T;

    fn new(n: u64) -> Self {
        debug_assert!(T::zero().count_zeros() >= 64 || n < (1 << T::zero().count_zeros() as usize));
        let n = T::from_u64(n);
        let a = modular_inverse(n).wrapping_neg();
        debug_assert_eq!(n.wrapping_mul(&a), T::one().wrapping_neg());
        Self { a, n }
    }

    fn modulus(&self) -> u64 {
        self.n.as_u64()
    }

    fn to_mod(&self, x: u64) -> Self::ModInt {
        // TODO: optimize!
        debug_assert!(x < self.n.as_u64());
        let r = T::from_double_width(
            ((T::DoubleWidth::from_u64(x)) << T::zero().count_zeros() as usize)
                % self.n.as_double_width(),
        );
        debug_assert_eq!(x, self.to_u64(r));
        r
    }

    fn to_u64(&self, n: Self::ModInt) -> u64 {
        self.reduce(n.as_double_width()).as_u64()
    }

    fn add(&self, a: Self::ModInt, b: Self::ModInt) -> Self::ModInt {
        let (r, overflow) = a.overflowing_add(&b);

        // In case of overflow, a+b = 2⁶⁴ + r = (2⁶⁴ - n) + r (working mod n)
        let r = if overflow {
            r + self.n.wrapping_neg()
        } else {
            r
        };

        // Normalize to [0; n[
        let r = if r < self.n { r } else { r - self.n };

        // Check that r (reduced back to the usual representation) equals
        // a+b % n
        #[cfg(debug_assertions)]
        {
            let a_r = self.to_u64(a) as u128;
            let b_r = self.to_u64(b) as u128;
            let r_r = self.to_u64(r);
            let r_2 = ((a_r + b_r) % self.n.as_u128()) as u64;
            debug_assert_eq!(
                r_r, r_2,
                "[{}] = {} ≠ {} = {} + {} = [{}] + [{}] mod {}; a = {}",
                r, r_r, r_2, a_r, b_r, a, b, self.n, self.a
            );
        }
        r
    }

    fn mul(&self, a: Self::ModInt, b: Self::ModInt) -> Self::ModInt {
        let r = self.reduce(a.as_double_width() * b.as_double_width());

        // Check that r (reduced back to the usual representation) equals
        // a*b % n
        #[cfg(debug_assertions)]
        {
            let a_r = self.to_u64(a) as u128;
            let b_r = self.to_u64(b) as u128;
            let r_r = self.to_u64(r);
            let r_2: u64 = ((a_r * b_r) % self.n.as_u128()) as u64;
            debug_assert_eq!(
                r_r, r_2,
                "[{}] = {} ≠ {} = {} * {} = [{}] * [{}] mod {}; a = {}",
                r, r_r, r_2, a_r, b_r, a, b, self.n, self.a
            );
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn test_add<A: DoubleInt>() {
        for n in 0..100 {
            let n = 2 * n + 1;
            let m = Montgomery::<A>::new(n);
            for x in 0..n {
                let m_x = m.to_mod(x);
                for y in 0..=x {
                    let m_y = m.to_mod(y);
                    println!("{n:?}, {x:?}, {y:?}");
                    assert_eq!((x + y) % n, m.to_u64(m.add(m_x, m_y)));
                }
            }
        }
    }

    #[test]
    fn add_u32() {
        test_add::<u32>();
    }

    #[test]
    fn add_u64() {
        test_add::<u64>();
    }

    fn test_multiplication<A: DoubleInt>() {
        for n in 0..100 {
            let n = 2 * n + 1;
            let m = Montgomery::<A>::new(n);
            for x in 0..n {
                let m_x = m.to_mod(x);
                for y in 0..=x {
                    let m_y = m.to_mod(y);
                    assert_eq!((x * y) % n, m.to_u64(m.mul(m_x, m_y)));
                }
            }
        }
    }

    #[test]
    fn multiplication_u32() {
        test_multiplication::<u32>();
    }

    #[test]
    fn multiplication_u64() {
        test_multiplication::<u64>();
    }

    fn test_roundtrip<A: DoubleInt>() {
        for n in 0..100 {
            let n = 2 * n + 1;
            let m = Montgomery::<A>::new(n);
            for x in 0..n {
                let x_ = m.to_mod(x);
                assert_eq!(x, m.to_u64(x_));
            }
        }
    }

    #[test]
    fn roundtrip_u32() {
        test_roundtrip::<u32>();
    }

    #[test]
    fn roundtrip_u64() {
        test_roundtrip::<u64>();
    }
}
