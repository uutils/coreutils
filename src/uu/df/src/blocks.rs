//  * This file is part of the uutils coreutils package.
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.
//! Types for representing and displaying block sizes.
use crate::{OPT_BLOCKSIZE, OPT_PORTABILITY};
use clap::ArgMatches;
use std::{env, fmt};

use uucore::{
    display::Quotable,
    parse_size::{parse_size, ParseSizeError},
};

/// The first ten powers of 1024.
const IEC_BASES: [u128; 10] = [
    1,
    1_024,
    1_048_576,
    1_073_741_824,
    1_099_511_627_776,
    1_125_899_906_842_624,
    1_152_921_504_606_846_976,
    1_180_591_620_717_411_303_424,
    1_208_925_819_614_629_174_706_176,
    1_237_940_039_285_380_274_899_124_224,
];

/// The first ten powers of 1000.
const SI_BASES: [u128; 10] = [
    1,
    1_000,
    1_000_000,
    1_000_000_000,
    1_000_000_000_000,
    1_000_000_000_000_000,
    1_000_000_000_000_000_000,
    1_000_000_000_000_000_000_000,
    1_000_000_000_000_000_000_000_000,
    1_000_000_000_000_000_000_000_000_000,
];

/// A SuffixType determines whether the suffixes are 1000 or 1024 based, and whether they are
/// intended for HumanReadable mode or not.
#[derive(Clone, Copy)]
pub(crate) enum SuffixType {
    Iec,
    Si,
    HumanReadable(HumanReadable),
}

impl SuffixType {
    /// The first ten powers of 1024 and 1000, respectively.
    fn bases(&self) -> [u128; 10] {
        match self {
            Self::Iec | Self::HumanReadable(HumanReadable::Binary) => IEC_BASES,
            Self::Si | Self::HumanReadable(HumanReadable::Decimal) => SI_BASES,
        }
    }

    /// Suffixes for the first nine multi-byte unit suffixes.
    fn suffixes(&self) -> [&'static str; 9] {
        match self {
            // we use "kB" instead of "KB", same as GNU df
            Self::Si => ["B", "kB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"],
            Self::Iec => ["B", "K", "M", "G", "T", "P", "E", "Z", "Y"],
            Self::HumanReadable(HumanReadable::Binary) => {
                ["", "K", "M", "G", "T", "P", "E", "Z", "Y"]
            }
            Self::HumanReadable(HumanReadable::Decimal) => {
                ["", "k", "M", "G", "T", "P", "E", "Z", "Y"]
            }
        }
    }
}

/// Convert a number into a magnitude and a multi-byte unit suffix.
///
/// The returned string has a maximum length of 5 chars, for example: "1.1kB", "999kB", "1MB".
pub(crate) fn to_magnitude_and_suffix(n: u128, suffix_type: SuffixType) -> String {
    let bases = suffix_type.bases();
    let suffixes = suffix_type.suffixes();
    let mut i = 0;

    while bases[i + 1] - bases[i] < n && i < suffixes.len() {
        i += 1;
    }

    let quot = n / bases[i];
    let rem = n % bases[i];
    let suffix = suffixes[i];

    if rem == 0 {
        format!("{}{}", quot, suffix)
    } else {
        let tenths_place = rem / (bases[i] / 10);

        if rem % (bases[i] / 10) == 0 {
            format!("{}.{}{}", quot, tenths_place, suffix)
        } else if tenths_place + 1 == 10 || quot >= 10 {
            format!("{}{}", quot + 1, suffix)
        } else {
            format!("{}.{}{}", quot, tenths_place + 1, suffix)
        }
    }
}

/// A mode to use in condensing the human readable display of a large number
/// of bytes.
///
/// The [`HumanReadable::Decimal`] and[`HumanReadable::Binary`] variants
/// represent dynamic block sizes: as the number of bytes increases, the
/// divisor increases as well (for example, from 1 to 1,000 to 1,000,000
/// and so on in the case of [`HumanReadable::Decimal`]).
#[derive(Clone, Copy)]
pub(crate) enum HumanReadable {
    /// Use the largest divisor corresponding to a unit, like B, K, M, G, etc.
    ///
    /// This variant represents powers of 1,000. Contrast with
    /// [`HumanReadable::Binary`], which represents powers of
    /// 1,024.
    Decimal,

    /// Use the largest divisor corresponding to a unit, like B, K, M, G, etc.
    ///
    /// This variant represents powers of 1,024. Contrast with
    /// [`HumanReadable::Decimal`], which represents powers
    /// of 1,000.
    Binary,
}

/// A block size to use in condensing the display of a large number of bytes.
///
/// The [`BlockSize::Bytes`] variant represents a static block
/// size.
///
/// The default variant is `Bytes(1024)`.
#[derive(Debug, PartialEq)]
pub(crate) enum BlockSize {
    /// A fixed number of bytes.
    ///
    /// The number must be positive.
    Bytes(u64),
}

impl BlockSize {
    /// Returns the associated value
    pub(crate) fn as_u64(&self) -> u64 {
        match *self {
            Self::Bytes(n) => n,
        }
    }
}

impl Default for BlockSize {
    fn default() -> Self {
        if env::var("POSIXLY_CORRECT").is_ok() {
            Self::Bytes(512)
        } else {
            Self::Bytes(1024)
        }
    }
}

pub(crate) fn read_block_size(matches: &ArgMatches) -> Result<BlockSize, ParseSizeError> {
    if matches.contains_id(OPT_BLOCKSIZE) {
        let s = matches.get_one::<String>(OPT_BLOCKSIZE).unwrap();
        let bytes = parse_size(s)?;

        if bytes > 0 {
            Ok(BlockSize::Bytes(bytes))
        } else {
            Err(ParseSizeError::ParseFailure(format!("{}", s.quote())))
        }
    } else if matches.get_flag(OPT_PORTABILITY) {
        Ok(BlockSize::default())
    } else if let Some(bytes) = block_size_from_env() {
        Ok(BlockSize::Bytes(bytes))
    } else {
        Ok(BlockSize::default())
    }
}

fn block_size_from_env() -> Option<u64> {
    for env_var in ["DF_BLOCK_SIZE", "BLOCK_SIZE", "BLOCKSIZE"] {
        if let Ok(env_size) = env::var(env_var) {
            if let Ok(size) = parse_size(&env_size) {
                return Some(size);
            } else {
                return None;
            }
        }
    }

    None
}

impl fmt::Display for BlockSize {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Bytes(n) => {
                let s = if n % 1024 == 0 && n % 1000 != 0 {
                    to_magnitude_and_suffix(*n as u128, SuffixType::Iec)
                } else {
                    to_magnitude_and_suffix(*n as u128, SuffixType::Si)
                };

                write!(f, "{}", s)
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use std::env;

    use crate::blocks::{to_magnitude_and_suffix, BlockSize, SuffixType};

    #[test]
    fn test_to_magnitude_and_suffix_powers_of_1024() {
        assert_eq!(to_magnitude_and_suffix(1024, SuffixType::Iec), "1K");
        assert_eq!(to_magnitude_and_suffix(2048, SuffixType::Iec), "2K");
        assert_eq!(to_magnitude_and_suffix(4096, SuffixType::Iec), "4K");
        assert_eq!(to_magnitude_and_suffix(1024 * 1024, SuffixType::Iec), "1M");
        assert_eq!(
            to_magnitude_and_suffix(2 * 1024 * 1024, SuffixType::Iec),
            "2M"
        );
        assert_eq!(
            to_magnitude_and_suffix(1024 * 1024 * 1024, SuffixType::Iec),
            "1G"
        );
        assert_eq!(
            to_magnitude_and_suffix(34 * 1024 * 1024 * 1024, SuffixType::Iec),
            "34G"
        );
    }

    #[test]
    fn test_to_magnitude_and_suffix_not_powers_of_1024() {
        assert_eq!(to_magnitude_and_suffix(1, SuffixType::Si), "1B");
        assert_eq!(to_magnitude_and_suffix(999, SuffixType::Si), "999B");

        assert_eq!(to_magnitude_and_suffix(1000, SuffixType::Si), "1kB");
        assert_eq!(to_magnitude_and_suffix(1001, SuffixType::Si), "1.1kB");
        assert_eq!(to_magnitude_and_suffix(1023, SuffixType::Si), "1.1kB");
        assert_eq!(to_magnitude_and_suffix(1025, SuffixType::Si), "1.1kB");
        assert_eq!(to_magnitude_and_suffix(10_001, SuffixType::Si), "11kB");
        assert_eq!(to_magnitude_and_suffix(999_000, SuffixType::Si), "999kB");

        assert_eq!(to_magnitude_and_suffix(999_001, SuffixType::Si), "1MB");
        assert_eq!(to_magnitude_and_suffix(999_999, SuffixType::Si), "1MB");
        assert_eq!(to_magnitude_and_suffix(1_000_000, SuffixType::Si), "1MB");
        assert_eq!(to_magnitude_and_suffix(1_000_001, SuffixType::Si), "1.1MB");
        assert_eq!(to_magnitude_and_suffix(1_100_000, SuffixType::Si), "1.1MB");
        assert_eq!(to_magnitude_and_suffix(1_100_001, SuffixType::Si), "1.2MB");
        assert_eq!(to_magnitude_and_suffix(1_900_000, SuffixType::Si), "1.9MB");
        assert_eq!(to_magnitude_and_suffix(1_900_001, SuffixType::Si), "2MB");
        assert_eq!(to_magnitude_and_suffix(9_900_000, SuffixType::Si), "9.9MB");
        assert_eq!(to_magnitude_and_suffix(9_900_001, SuffixType::Si), "10MB");
        assert_eq!(
            to_magnitude_and_suffix(999_000_000, SuffixType::Si),
            "999MB"
        );

        assert_eq!(to_magnitude_and_suffix(999_000_001, SuffixType::Si), "1GB");
        assert_eq!(
            to_magnitude_and_suffix(1_000_000_000, SuffixType::Si),
            "1GB"
        );
        assert_eq!(
            to_magnitude_and_suffix(1_000_000_001, SuffixType::Si),
            "1.1GB"
        );
    }

    #[test]
    fn test_block_size_display() {
        assert_eq!(format!("{}", BlockSize::Bytes(1024)), "1K");
        assert_eq!(format!("{}", BlockSize::Bytes(2 * 1024)), "2K");
        assert_eq!(format!("{}", BlockSize::Bytes(3 * 1024 * 1024)), "3M");
    }

    #[test]
    fn test_block_size_display_multiples_of_1000_and_1024() {
        assert_eq!(format!("{}", BlockSize::Bytes(128_000)), "128kB");
        assert_eq!(format!("{}", BlockSize::Bytes(1000 * 1024)), "1.1MB");
        assert_eq!(format!("{}", BlockSize::Bytes(1_000_000_000_000)), "1TB");
    }

    #[test]
    fn test_default_block_size() {
        assert_eq!(BlockSize::Bytes(1024), BlockSize::default());
        env::set_var("POSIXLY_CORRECT", "1");
        assert_eq!(BlockSize::Bytes(512), BlockSize::default());
        env::remove_var("POSIXLY_CORRECT");
    }
}
