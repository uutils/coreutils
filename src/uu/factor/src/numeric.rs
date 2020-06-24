// * This file is part of the uutils coreutils package.
// *
// * (c) 2015 Wiktor Kuropatwa <wiktor.kuropatwa@gmail.com>
// * (c) 2020 nicoo            <nicoo@debian.org>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

use num_traits::{
    cast::AsPrimitive,
    identities::{One, Zero},
    int::PrimInt,
    ops::wrapping::{WrappingMul, WrappingNeg, WrappingSub},
};
use std::fmt::{Debug, Display};
use std::mem::swap;

// This is incorrectly reported as dead code,
//  presumably when included in build.rs.
#[allow(dead_code)]
pub(crate) fn gcd(mut a: u64, mut b: u64) -> u64 {
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
pub(crate) struct Montgomery<T: Int> {
    a: T,
    n: T,
}

impl<T: Int> Montgomery<T> {
    /// computes x/R mod n efficiently
    fn reduce(&self, x: T::Intermediate) -> T {
        let t_bits = T::zero().count_zeros() as usize;

        debug_assert!(x < (self.n.as_intermediate()) << t_bits);
        // TODO: optimiiiiiiise
        let Montgomery { a, n } = self;
        let m = T::from_intermediate(x).wrapping_mul(a);
        let nm = (n.as_intermediate()) * (m.as_intermediate());
        let (xnm, overflow) = x.overflowing_add_(nm); // x + n*m
        debug_assert_eq!(
            xnm % (T::Intermediate::one() << T::zero().count_zeros() as usize),
            T::Intermediate::zero()
        );

        // (x + n*m) / R
        // in case of overflow, this is (2¹²⁸ + xnm)/2⁶⁴ - n = xnm/2⁶⁴ + (2⁶⁴ - n)
        let y = T::from_intermediate(xnm >> t_bits)
            + if !overflow {
                T::zero()
            } else {
                n.wrapping_neg()
            };

        if y >= *n {
            y - *n
        } else {
            y
        }
    }
}

impl<T: Int> Arithmetic for Montgomery<T> {
    // Montgomery transform, R=2⁶⁴
    // Provides fast arithmetic mod n (n odd, u64)
    type I = T;

    fn new(n: u64) -> Self {
        debug_assert!(T::zero().count_zeros() >= 64 || n < (1 << T::zero().count_zeros() as usize));
        let n = T::from_u64(n);
        let a = modular_inverse(n).wrapping_neg();
        debug_assert_eq!(n.wrapping_mul(&a), T::one().wrapping_neg());
        Montgomery { a, n }
    }

    fn modulus(&self) -> u64 {
        self.n.as_()
    }

    fn from_u64(&self, x: u64) -> Self::I {
        // TODO: optimise!
        debug_assert!(x < self.n.as_());
        let r = T::from_intermediate(
            ((T::Intermediate::from(x)) << T::zero().count_zeros() as usize)
                % self.n.as_intermediate(),
        );
        debug_assert_eq!(x, self.to_u64(r));
        r
    }

    fn to_u64(&self, n: Self::I) -> u64 {
        self.reduce(n.as_intermediate()).as_()
    }

    fn add(&self, a: Self::I, b: Self::I) -> Self::I {
        let (r, overflow) = a.overflowing_add_(b);

        // In case of overflow, a+b = 2⁶⁴ + r = (2⁶⁴ - n) + r (working mod n)
        let r = if !overflow {
            r
        } else {
            r + self.n.wrapping_neg()
        };

        // Normalise to [0; n[
        let r = if r < self.n { r } else { r - self.n };

        // Check that r (reduced back to the usual representation) equals
        // a+b % n
        #[cfg(debug_assertions)]
        {
            let a_r = self.to_u64(a);
            let b_r = self.to_u64(b);
            let r_r = self.to_u64(r);
            let r_2 =
                (((a_r as u128) + (b_r as u128)) % <T as AsPrimitive<u128>>::as_(self.n)) as u64;
            debug_assert_eq!(
                r_r, r_2,
                "[{}] = {} ≠ {} = {} + {} = [{}] + [{}] mod {}; a = {}",
                r, r_r, r_2, a_r, b_r, a, b, self.n, self.a
            );
        }
        r
    }

    fn mul(&self, a: Self::I, b: Self::I) -> Self::I {
        let r = self.reduce(a.as_intermediate() * b.as_intermediate());

        // Check that r (reduced back to the usual representation) equals
        // a*b % n
        #[cfg(debug_assertions)]
        {
            let a_r = self.to_u64(a);
            let b_r = self.to_u64(b);
            let r_r = self.to_u64(r);
            let r_2: u64 = ((T::Intermediate::from(a_r) * T::Intermediate::from(b_r))
                % self.n.as_intermediate())
            .as_();
            debug_assert_eq!(
                r_r, r_2,
                "[{}] = {} ≠ {} = {} * {} = [{}] * [{}] mod {}; a = {}",
                r, r_r, r_2, a_r, b_r, a, b, self.n, self.a
            );
        }
        r
    }
}

pub(crate) trait OverflowingAdd: Sized {
    fn overflowing_add_(self, n: Self) -> (Self, bool);
}
impl OverflowingAdd for u32 {
    fn overflowing_add_(self, n: Self) -> (Self, bool) {
        self.overflowing_add(n)
    }
}
impl OverflowingAdd for u64 {
    fn overflowing_add_(self, n: Self) -> (Self, bool) {
        self.overflowing_add(n)
    }
}
impl OverflowingAdd for u128 {
    fn overflowing_add_(self, n: Self) -> (Self, bool) {
        self.overflowing_add(n)
    }
}

pub(crate) trait Int:
    AsPrimitive<u64>
    + AsPrimitive<u128>
    + Display
    + Debug
    + PrimInt
    + OverflowingAdd
    + WrappingNeg
    + WrappingSub
    + WrappingMul
{
    type Intermediate: From<u64>
        + AsPrimitive<u64>
        + Display
        + Debug
        + PrimInt
        + OverflowingAdd
        + WrappingNeg
        + WrappingSub
        + WrappingMul;

    fn as_intermediate(self) -> Self::Intermediate;
    fn from_intermediate(n: Self::Intermediate) -> Self;
    fn from_u64(n: u64) -> Self;
}
impl Int for u64 {
    type Intermediate = u128;

    fn as_intermediate(self) -> u128 {
        self as _
    }
    fn from_intermediate(n: u128) -> u64 {
        n as _
    }
    fn from_u64(n: u64) -> u64 {
        n
    }
}
impl Int for u32 {
    type Intermediate = u64;

    fn as_intermediate(self) -> u64 {
        self as _
    }
    fn from_intermediate(n: u64) -> u32 {
        n as _
    }
    fn from_u64(n: u64) -> u32 {
        n as _
    }
}

// extended Euclid algorithm
// precondition: a is odd
pub(crate) fn modular_inverse<T: Int>(a: T) -> T {
    let zero = T::zero();
    let one = T::one();
    debug_assert!(a % (one + one) == one, "{:?} is not odd", a);

    let mut t = zero;
    let mut newt = one;
    let mut r = zero;
    let mut newr = a;

    while newr != zero {
        let quot = if r == zero {
            // special case when we're just starting out
            // This works because we know that
            // a does not divide 2^64, so floor(2^64 / a) == floor((2^64-1) / a);
            T::max_value()
        } else {
            r
        } / newr;

        let newtp = t.wrapping_sub(&quot.wrapping_mul(&newt));
        t = newt;
        newt = newtp;

        let newrp = r.wrapping_sub(&quot.wrapping_mul(&newr));
        r = newr;
        newr = newrp;
    }

    debug_assert_eq!(r, one);
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_inverter<T: Int>() {
        // All odd integers from 1 to 20 000
        let one = T::from(1).unwrap();
        let two = T::from(2).unwrap();
        let mut test_values = (0..10_000)
            .map(|i| T::from(i).unwrap())
            .map(|i| two * i + one);

        assert!(test_values.all(|x| x.wrapping_mul(&modular_inverse(x)) == one));
    }

    #[test]
    fn test_inverter_u32() {
        test_inverter::<u32>()
    }

    #[test]
    fn test_inverter_u64() {
        test_inverter::<u64>()
    }

    fn test_add<A: Arithmetic>() {
        for n in 0..100 {
            let n = 2 * n + 1;
            let m = A::new(n);
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
    fn test_add_m32() {
        test_add::<Montgomery<u32>>()
    }

    #[test]
    fn test_add_m64() {
        test_add::<Montgomery<u64>>()
    }

    fn test_mult<A: Arithmetic>() {
        for n in 0..100 {
            let n = 2 * n + 1;
            let m = A::new(n);
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
    fn test_mult_m32() {
        test_mult::<Montgomery<u32>>()
    }

    #[test]
    fn test_mult_m64() {
        test_mult::<Montgomery<u64>>()
    }

    fn test_roundtrip<A: Arithmetic>() {
        for n in 0..100 {
            let n = 2 * n + 1;
            let m = A::new(n);
            for x in 0..n {
                let x_ = m.from_u64(x);
                assert_eq!(x, m.to_u64(x_));
            }
        }
    }

    #[test]
    fn test_roundtrip_m32() {
        test_roundtrip::<Montgomery<u32>>()
    }

    #[test]
    fn test_roundtrip_m64() {
        test_roundtrip::<Montgomery<u64>>()
    }
}
