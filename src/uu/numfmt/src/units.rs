// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use std::fmt;
use uucore::parser::parse_size::{IEC_BASES, SI_BASES};

/// `f64` view of [`uucore::parser::parse_size::SI_BASES`] for numfmt's
/// floating-point math paths.
pub fn si_bases_f64() -> [f64; 11] {
    SI_BASES.map(|b| b as f64)
}

/// `f64` view of [`uucore::parser::parse_size::IEC_BASES`].
pub fn iec_bases_f64() -> [f64; 11] {
    IEC_BASES.map(|b| b as f64)
}

pub type WithI = bool;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Auto,
    Si,
    Iec(WithI),
    None,
}

pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy, Debug)]
#[repr(usize)]
pub enum RawSuffix {
    K = 0,
    M,
    G,
    T,
    P,
    E,
    Z,
    Y,
    R,
    Q,
}

impl RawSuffix {
    /// Index of this suffix in the base arrays, minus one.
    /// `K` is 0, `M` is 1, ..., `Q` is 9. The associated base is
    /// `BASES[self.index() + 1]`.
    pub fn index(self) -> usize {
        self as usize
    }
}

impl TryFrom<&char> for RawSuffix {
    type Error = String;

    fn try_from(value: &char) -> Result<Self> {
        match value {
            'K' | 'k' => Ok(Self::K),
            'M' => Ok(Self::M),
            'G' => Ok(Self::G),
            'T' => Ok(Self::T),
            'P' => Ok(Self::P),
            'E' => Ok(Self::E),
            'Z' => Ok(Self::Z),
            'Y' => Ok(Self::Y),
            'R' => Ok(Self::R),
            'Q' => Ok(Self::Q),
            _ => Err(format!("Invalid suffix: {value}")),
        }
    }
}

pub type Suffix = (RawSuffix, WithI);

pub struct DisplayableSuffix(pub Suffix, pub Unit);

/// Upper-case characters for each [`RawSuffix`], indexed by [`RawSuffix::index`].
const SUFFIX_CHARS: [char; 10] = ['K', 'M', 'G', 'T', 'P', 'E', 'Z', 'Y', 'R', 'Q'];

impl fmt::Display for DisplayableSuffix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self((raw_suffix, with_i), unit) = *self;
        let ch = match (raw_suffix, unit) {
            (RawSuffix::K, Unit::Si) => 'k',
            _ => SUFFIX_CHARS[raw_suffix.index()],
        };
        write!(f, "{ch}")?;
        if with_i {
            write!(f, "i")?;
        }
        Ok(())
    }
}
