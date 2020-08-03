// * This file is part of the uutils coreutils package.
// *
// * (c) 2020 nicoo            <nicoo@debian.org>
// *
// * For the full copyright and license information, please view the LICENSE file
// * that was distributed with this source code.

mod gcd;
pub use gcd::gcd;

pub(crate) mod traits;

mod modular_inverse;
pub(crate) use modular_inverse::modular_inverse;

mod montgomery;
pub(crate) use montgomery::{Arithmetic, Montgomery};
