// * This file is part of the uutils coreutils package.
// *
// * (c) 2020 nicoo            <nicoo@debian.org>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

mod gcd;
pub use gcd::gcd;

mod traits;
use traits::{DoubleInt, Int, OverflowingAdd};

mod modular_inverse;
pub(crate) use modular_inverse::modular_inverse;

mod montgomery;
pub(crate) use montgomery::{Arithmetic, Montgomery};
