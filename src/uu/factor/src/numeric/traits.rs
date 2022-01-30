// * This file is part of the uutils coreutils package.
// *
// * (c) 2020 Alex Lyon  <arcterus@mail.com>
// * (c) 2020 nicoo      <nicoo@debian.org>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

pub(crate) use num_traits::{
    identities::{One, Zero},
    ops::overflowing::OverflowingAdd,
};
use num_traits::{
    int::PrimInt,
    ops::wrapping::{WrappingMul, WrappingNeg, WrappingSub},
};
use std::fmt::{Debug, Display};

pub(crate) trait Int:
    Display + Debug + PrimInt + OverflowingAdd + WrappingNeg + WrappingSub + WrappingMul
{
    fn as_u64(&self) -> u64;
    fn from_u64(n: u64) -> Self;

    #[cfg(debug_assertions)]
    fn as_u128(&self) -> u128;
}

pub(crate) trait DoubleInt: Int {
    /// An integer type with twice the width of `Self`.
    /// In particular, multiplications (of `Int` values) can be performed in
    ///  `Self::DoubleWidth` without possibility of overflow.
    type DoubleWidth: Int;

    fn as_double_width(self) -> Self::DoubleWidth;
    fn from_double_width(n: Self::DoubleWidth) -> Self;
}

macro_rules! int {
    ( $x:ty ) => {
        impl Int for $x {
            fn as_u64(&self) -> u64 {
                *self as u64
            }
            fn from_u64(n: u64) -> Self {
                n as _
            }
            #[cfg(debug_assertions)]
            fn as_u128(&self) -> u128 {
                *self as u128
            }
        }
    };
}
macro_rules! double_int {
    ( $x:ty, $y:ty ) => {
        int!($x);
        impl DoubleInt for $x {
            type DoubleWidth = $y;

            fn as_double_width(self) -> $y {
                self as _
            }
            fn from_double_width(n: $y) -> Self {
                n as _
            }
        }
    };
}
double_int!(u32, u64);
double_int!(u64, u128);
int!(u128);

/// Helper macro for instantiating tests over u32 and u64
#[cfg(test)]
#[macro_export]
macro_rules! parametrized_check {
    ( $f:ident ) => {
        paste::item! {
            #[test]
            fn [< $f _ u32 >]() {
                $f::<u32>()
            }
            #[test]
            fn [< $f _ u64 >]() {
                $f::<u64>()
            }
        }
    };
}
