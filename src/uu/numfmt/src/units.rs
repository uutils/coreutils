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

pub struct DisplayableSuffix(pub Suffix);

impl fmt::Display for DisplayableSuffix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let DisplayableSuffix((ref raw_suffix, ref with_i)) = *self;
        match raw_suffix {
            RawSuffix::K => write!(f, "K"),
            RawSuffix::M => write!(f, "M"),
            RawSuffix::G => write!(f, "G"),
            RawSuffix::T => write!(f, "T"),
            RawSuffix::P => write!(f, "P"),
            RawSuffix::E => write!(f, "E"),
            RawSuffix::Z => write!(f, "Z"),
            RawSuffix::Y => write!(f, "Y"),
        }
        .and_then(|()| match with_i {
            true => write!(f, "i"),
            false => Ok(()),
        })
    }
}
