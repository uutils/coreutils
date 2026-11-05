// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// This file is written to solve #11654
// This mod is to preserve numeric precision for large integers.
// GNU numfmt has a better precision on floats due to 'long double'.

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ParsedNumber {
    ExactInt(i128),
    Float(f64),
}

impl ParsedNumber {
    pub(crate) fn to_f64(self) -> f64 {
        match self {
            Self::ExactInt(n) => n as f64,
            Self::Float(n) => n,
        }
    }

    pub(crate) fn exact_int(self) -> Option<i128> {
        match self {
            Self::ExactInt(n) => Some(n),
            Self::Float(_) => None,
        }
    }
}
