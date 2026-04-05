// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use std::fmt;

pub const SI_BASES: [f64; 11] = [1., 1e3, 1e6, 1e9, 1e12, 1e15, 1e18, 1e21, 1e24, 1e27, 1e30];

pub const IEC_BASES: [f64; 11] = [
    1.,
    1_024.,
    1_048_576.,
    1_073_741_824.,
    1_099_511_627_776.,
    1_125_899_906_842_624.,
    1_152_921_504_606_846_976.,
    1_180_591_620_717_411_303_424.,
    1_208_925_819_614_629_174_706_176.,
    1_237_940_039_285_380_274_899_124_224.,
    1_267_650_600_228_229_401_496_703_205_376.,
];

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
    /// Index of this suffix in [`SI_BASES`] / [`IEC_BASES`] minus one.
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
