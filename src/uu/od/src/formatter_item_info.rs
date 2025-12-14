// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (ToDO) formatteriteminfo

use std::fmt;

#[allow(clippy::enum_variant_names)]
#[allow(unpredictable_function_pointer_comparisons)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FormatWriter {
    IntWriter(fn(u64) -> String),
    FloatWriter(fn(f64) -> String),
    LongDoubleWriter(fn(f64) -> String), // On most platforms, long double is f64 or emulated
    BFloatWriter(fn(f64) -> String),
    MultibyteWriter(fn(&[u8]) -> String),
}

impl fmt::Debug for FormatWriter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::IntWriter(ref p) => {
                f.write_str("IntWriter:")?;
                fmt::Pointer::fmt(p, f)
            }
            Self::FloatWriter(ref p) => {
                f.write_str("FloatWriter:")?;
                fmt::Pointer::fmt(p, f)
            }
            Self::LongDoubleWriter(ref p) => {
                f.write_str("LongDoubleWriter:")?;
                fmt::Pointer::fmt(p, f)
            }
            Self::BFloatWriter(ref p) => {
                f.write_str("BFloatWriter:")?;
                fmt::Pointer::fmt(p, f)
            }
            Self::MultibyteWriter(ref p) => {
                f.write_str("MultibyteWriter:")?;
                fmt::Pointer::fmt(&(*p as *const ()), f)
            }
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct FormatterItemInfo {
    pub byte_size: usize,
    pub print_width: usize, // including a space in front of the text
    pub formatter: FormatWriter,
}
