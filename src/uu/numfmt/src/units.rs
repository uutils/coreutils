// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use std::fmt;

pub const SI_BASES: [f64; 10] = [1., 1e3, 1e6, 1e9, 1e12, 1e15, 1e18, 1e21, 1e24, 1e27];

pub const IEC_BASES: [f64; 10] = [
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
pub enum RawSuffix {
    K,
    M,
    G,
    T,
    P,
    E,
    Z,
    Y,
}

pub type Suffix = (RawSuffix, WithI);

pub struct DisplayableSuffix(pub Suffix, pub Unit);

impl fmt::Display for DisplayableSuffix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self((ref raw_suffix, ref with_i), unit) = *self;
        match (raw_suffix, unit) {
            (RawSuffix::K, Unit::Si) => write!(f, "k"),
            (RawSuffix::K, _) => write!(f, "K"),
            (RawSuffix::M, _) => write!(f, "M"),
            (RawSuffix::G, _) => write!(f, "G"),
            (RawSuffix::T, _) => write!(f, "T"),
            (RawSuffix::P, _) => write!(f, "P"),
            (RawSuffix::E, _) => write!(f, "E"),
            (RawSuffix::Z, _) => write!(f, "Z"),
            (RawSuffix::Y, _) => write!(f, "Y"),
        }
        .and_then(|()| match with_i {
            true => write!(f, "i"),
            false => Ok(()),
        })
    }
}
