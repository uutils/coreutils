// * This file is part of the uutils coreutils package.
// *
// * (c) 2015 Wiktor Kuropatwa <wiktor.kuropatwa@gmail.com>
// * (c) 2020 nicoo            <nicoo@debian.org>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

use super::traits::Int;

// extended Euclid algorithm
// precondition: a is odd
pub(crate) fn modular_inverse<T: Int>(a: T) -> T {
    let zero = T::zero();
    let one = T::one();
    debug_assert!(a % (one + one) == one, "{:?} is not odd", a);

    let mut t = zero;
    let mut new_t = one;
    let mut r = zero;
    let mut new_r = a;

    while new_r != zero {
        let quot = if r == zero {
            // special case when we're just starting out
            // This works because we know that
            // a does not divide 2^64, so floor(2^64 / a) == floor((2^64-1) / a);
            T::max_value()
        } else {
            r
        } / new_r;

        let new_tp = t.wrapping_sub(&quot.wrapping_mul(&new_t));
        t = new_t;
        new_t = new_tp;

        let new_rp = r.wrapping_sub(&quot.wrapping_mul(&new_r));
        r = new_r;
        new_r = new_rp;
    }

    debug_assert_eq!(r, one);
    t
}

#[cfg(test)]
mod tests {
    use super::{super::traits::Int, *};
    use crate::parametrized_check;
    use quickcheck::quickcheck;

    fn small_values<T: Int>() {
        // All odd integers from 1 to 20 000
        let one = T::from(1).unwrap();
        let two = T::from(2).unwrap();
        let mut test_values = (0..10_000)
            .map(|i| T::from(i).unwrap())
            .map(|i| two * i + one);

        assert!(test_values.all(|x| x.wrapping_mul(&modular_inverse(x)) == one));
    }
    parametrized_check!(small_values);

    quickcheck! {
        fn random_values_u32(n: u32) -> bool {
            match 2_u32.checked_mul(n) {
                Some(n) => modular_inverse(n + 1).wrapping_mul(n + 1) == 1,
                _ => true,
            }
        }

        fn random_values_u64(n: u64) -> bool {
            match 2_u64.checked_mul(n) {
                Some(n) => modular_inverse(n + 1).wrapping_mul(n + 1) == 1,
                _ => true,
            }
        }
    }
}
