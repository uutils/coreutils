//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
//! Types for representing and displaying block sizes.
use crate::{OPT_HUMAN_READABLE, OPT_HUMAN_READABLE_2};
use clap::ArgMatches;

/// A block size to use in condensing the display of a large number of bytes.
///
/// The [`BlockSize::Bytes`] variant represents a static block
/// size. The [`BlockSize::HumanReadableDecimal`] and
/// [`BlockSize::HumanReadableBinary`] variants represent dynamic
/// block sizes: as the number of bytes increases, the divisor
/// increases as well (for example, from 1 to 1,000 to 1,000,000 and
/// so on in the case of [`BlockSize::HumanReadableDecimal`]).
///
/// The default variant is `Bytes(1024)`.
pub(crate) enum BlockSize {
    /// A fixed number of bytes.
    ///
    /// The number must be positive.
    Bytes(u64),

    /// Use the largest divisor corresponding to a unit, like B, K, M, G, etc.
    ///
    /// This variant represents powers of 1,000. Contrast with
    /// [`BlockSize::HumanReadableBinary`], which represents powers of
    /// 1,024.
    HumanReadableDecimal,

    /// Use the largest divisor corresponding to a unit, like B, K, M, G, etc.
    ///
    /// This variant represents powers of 1,024. Contrast with
    /// [`BlockSize::HumanReadableDecimal`], which represents powers
    /// of 1,000.
    HumanReadableBinary,
}

impl Default for BlockSize {
    fn default() -> Self {
        Self::Bytes(1024)
    }
}

impl From<&ArgMatches> for BlockSize {
    fn from(matches: &ArgMatches) -> Self {
        if matches.is_present(OPT_HUMAN_READABLE) {
            Self::HumanReadableBinary
        } else if matches.is_present(OPT_HUMAN_READABLE_2) {
            Self::HumanReadableDecimal
        } else {
            Self::default()
        }
    }
}
