// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! Types for representing and displaying block sizes.
use crate::{OPT_BLOCKSIZE, OPT_PORTABILITY};
use clap::ArgMatches;
use std::{env, fmt};

use uucore::{
    display::Quotable,
    parser::parse_size::{
        ParseSizeError, extract_thousands_separator_flag, parse_size_non_zero_u64, parse_size_u64,
    },
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

/// A `SuffixType` determines whether the suffixes are 1000 or 1024 based, and whether they are
/// intended for `HumanReadable` mode or not.
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
/// `add_tracing_zero` allows to add tracing zero for values in 0 < x <= 9
///
pub(crate) fn to_magnitude_and_suffix(
    n: u128,
    suffix_type: SuffixType,
    add_tracing_zero: bool,
) -> String {
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
        if add_tracing_zero && !suffix.is_empty() && quot != 0 && quot <= 9 {
            format!("{quot}.0{suffix}")
        } else {
            format!("{quot}{suffix}")
        }
    } else {
        let tenths_place = rem / (bases[i] / 10);

        if quot >= 100 && rem > 0 {
            format!("{}{suffix}", quot + 1)
        } else if rem % (bases[i] / 10) == 0 {
            format!("{quot}.{tenths_place}{suffix}")
        } else if tenths_place + 1 == 10 || quot >= 10 {
            let quot = quot + 1;
            if add_tracing_zero && !suffix.is_empty() && quot <= 9 {
                format!("{quot}.0{suffix}")
            } else {
                format!("{quot}{suffix}")
            }
        } else {
            format!("{quot}.{}{suffix}", tenths_place + 1)
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

/// Configuration for block size display, including thousands separator flag.
#[derive(Debug, PartialEq)]
pub(crate) struct BlockSizeConfig {
    pub(crate) block_size: BlockSize,
    pub(crate) use_thousands_separator: bool,
}

impl BlockSize {
    /// Returns the associated value
    pub(crate) fn as_u64(&self) -> u64 {
        match *self {
            Self::Bytes(n) => n,
        }
    }

    pub(crate) fn to_header(&self) -> String {
        match self {
            Self::Bytes(n) => {
                if n % 1024 == 0 && n % 1000 != 0 {
                    to_magnitude_and_suffix(*n as u128, SuffixType::Iec, false)
                } else {
                    to_magnitude_and_suffix(*n as u128, SuffixType::Si, false)
                }
            }
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

pub(crate) fn read_block_size(matches: &ArgMatches) -> Result<BlockSizeConfig, ParseSizeError> {
    if matches.contains_id(OPT_BLOCKSIZE) {
        let s = matches.get_one::<String>(OPT_BLOCKSIZE).unwrap();
        let (cleaned, use_thousands) = extract_thousands_separator_flag(s);
        let bytes = parse_size_u64(cleaned)?;

        if bytes > 0 {
            Ok(BlockSizeConfig {
                block_size: BlockSize::Bytes(bytes),
                use_thousands_separator: use_thousands,
            })
        } else {
            Err(ParseSizeError::ParseFailure(format!("{}", s.quote())))
        }
    } else if matches.get_flag(OPT_PORTABILITY) {
        Ok(BlockSizeConfig {
            block_size: BlockSize::default(),
            use_thousands_separator: false,
        })
    } else if let Some((bytes, use_thousands)) = block_size_from_env() {
        Ok(BlockSizeConfig {
            block_size: BlockSize::Bytes(bytes),
            use_thousands_separator: use_thousands,
        })
    } else {
        Ok(BlockSizeConfig {
            block_size: BlockSize::default(),
            use_thousands_separator: false,
        })
    }
}

fn block_size_from_env() -> Option<(u64, bool)> {
    for env_var in ["DF_BLOCK_SIZE", "BLOCK_SIZE", "BLOCKSIZE"] {
        if let Ok(env_size) = env::var(env_var) {
            let (cleaned, use_thousands) = extract_thousands_separator_flag(&env_size);
            if let Ok(size) = parse_size_non_zero_u64(cleaned) {
                return Some((size, use_thousands));
            }
            // If env var is set but invalid, return None (don't check other env vars)
            return None;
        }
    }

    None
}

impl fmt::Display for BlockSize {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Bytes(n) => {
                let s = if n % 1024 == 0 && n % 1000 != 0 {
                    to_magnitude_and_suffix(*n as u128, SuffixType::Iec, true)
                } else {
                    to_magnitude_and_suffix(*n as u128, SuffixType::Si, true)
                };

                write!(f, "{s}")
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use std::env;

    use crate::blocks::{BlockSize, SuffixType, to_magnitude_and_suffix};

    #[test]
    fn test_to_magnitude_and_suffix_rounding() {
        assert_eq!(
            to_magnitude_and_suffix(999_440, SuffixType::Si, true),
            "1.0MB"
        );
        assert_eq!(
            to_magnitude_and_suffix(819_200, SuffixType::Si, true),
            "820kB"
        );
        assert_eq!(
            to_magnitude_and_suffix(819_936, SuffixType::Si, true),
            "820kB"
        );
        assert_eq!(
            to_magnitude_and_suffix(818_400, SuffixType::Si, true),
            "819kB"
        );
        assert_eq!(
            to_magnitude_and_suffix(817_600, SuffixType::Si, true),
            "818kB"
        );
        assert_eq!(
            to_magnitude_and_suffix(817_200, SuffixType::Si, true),
            "818kB"
        );
    }

    #[test]
    fn test_to_magnitude_and_suffix_add_tracing_zero() {
        assert_eq!(to_magnitude_and_suffix(1024, SuffixType::Iec, true), "1.0K");
        assert_eq!(to_magnitude_and_suffix(2048, SuffixType::Iec, true), "2.0K");
        assert_eq!(to_magnitude_and_suffix(10240, SuffixType::Iec, true), "10K");

        assert_eq!(to_magnitude_and_suffix(1024, SuffixType::Iec, false), "1K");
        assert_eq!(to_magnitude_and_suffix(2048, SuffixType::Iec, false), "2K");
        assert_eq!(
            to_magnitude_and_suffix(10240, SuffixType::Iec, false),
            "10K"
        );
    }

    #[test]
    fn test_to_magnitude_and_suffix_powers_of_1024() {
        assert_eq!(to_magnitude_and_suffix(1024, SuffixType::Iec, false), "1K");
        assert_eq!(
            to_magnitude_and_suffix(10240, SuffixType::Iec, false),
            "10K"
        );
        assert_eq!(to_magnitude_and_suffix(2048, SuffixType::Iec, false), "2K");
        assert_eq!(
            to_magnitude_and_suffix(1024 * 40, SuffixType::Iec, false),
            "40K"
        );
        assert_eq!(
            to_magnitude_and_suffix(1024 * 1024, SuffixType::Iec, false),
            "1M"
        );
        assert_eq!(
            to_magnitude_and_suffix(2 * 1024 * 1024, SuffixType::Iec, false),
            "2M"
        );
        assert_eq!(
            to_magnitude_and_suffix(1024 * 1024 * 1024, SuffixType::Iec, false),
            "1G"
        );
        assert_eq!(
            to_magnitude_and_suffix(34 * 1024 * 1024 * 1024, SuffixType::Iec, false),
            "34G"
        );
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn test_to_magnitude_and_suffix_not_powers_of_1024() {
        assert_eq!(to_magnitude_and_suffix(1, SuffixType::Si, true), "1.0B");
        assert_eq!(to_magnitude_and_suffix(999, SuffixType::Si, true), "999B");

        assert_eq!(to_magnitude_and_suffix(1000, SuffixType::Si, true), "1.0kB");
        assert_eq!(to_magnitude_and_suffix(1001, SuffixType::Si, true), "1.1kB");
        assert_eq!(to_magnitude_and_suffix(1023, SuffixType::Si, true), "1.1kB");
        assert_eq!(to_magnitude_and_suffix(1025, SuffixType::Si, true), "1.1kB");
        assert_eq!(
            to_magnitude_and_suffix(10_001, SuffixType::Si, true),
            "11kB"
        );
        assert_eq!(
            to_magnitude_and_suffix(999_000, SuffixType::Si, true),
            "999kB"
        );

        assert_eq!(
            to_magnitude_and_suffix(999_001, SuffixType::Si, true),
            "1.0MB"
        );
        assert_eq!(
            to_magnitude_and_suffix(999_999, SuffixType::Si, true),
            "1.0MB"
        );
        assert_eq!(
            to_magnitude_and_suffix(1_000_000, SuffixType::Si, true),
            "1.0MB"
        );
        assert_eq!(
            to_magnitude_and_suffix(1_000_001, SuffixType::Si, true),
            "1.1MB"
        );
        assert_eq!(
            to_magnitude_and_suffix(1_100_000, SuffixType::Si, true),
            "1.1MB"
        );
        assert_eq!(
            to_magnitude_and_suffix(1_100_001, SuffixType::Si, true),
            "1.2MB"
        );
        assert_eq!(
            to_magnitude_and_suffix(1_900_000, SuffixType::Si, true),
            "1.9MB"
        );
        assert_eq!(
            to_magnitude_and_suffix(1_900_001, SuffixType::Si, true),
            "2.0MB"
        );
        assert_eq!(
            to_magnitude_and_suffix(9_900_000, SuffixType::Si, true),
            "9.9MB"
        );
        assert_eq!(
            to_magnitude_and_suffix(9_900_001, SuffixType::Si, true),
            "10MB"
        );
        assert_eq!(
            to_magnitude_and_suffix(999_000_000, SuffixType::Si, true),
            "999MB"
        );

        assert_eq!(
            to_magnitude_and_suffix(999_000_001, SuffixType::Si, true),
            "1.0GB"
        );
        assert_eq!(
            to_magnitude_and_suffix(1_000_000_000, SuffixType::Si, true),
            "1.0GB"
        );
        assert_eq!(
            to_magnitude_and_suffix(1_000_000_001, SuffixType::Si, true),
            "1.1GB"
        );
    }

    #[test]
    fn test_block_size_display() {
        assert_eq!(format!("{}", BlockSize::Bytes(1024)), "1.0K");
        assert_eq!(format!("{}", BlockSize::Bytes(2 * 1024)), "2.0K");
        assert_eq!(format!("{}", BlockSize::Bytes(3 * 1024 * 1024)), "3.0M");
    }

    #[test]
    fn test_block_size_display_multiples_of_1000_and_1024() {
        assert_eq!(format!("{}", BlockSize::Bytes(128_000)), "128kB");
        assert_eq!(format!("{}", BlockSize::Bytes(1000 * 1024)), "1.1MB");
        assert_eq!(format!("{}", BlockSize::Bytes(1_000_000_000_000)), "1.0TB");
    }

    #[test]
    fn test_default_block_size() {
        assert_eq!(BlockSize::Bytes(1024), BlockSize::default());
        unsafe { env::set_var("POSIXLY_CORRECT", "1") };
        assert_eq!(BlockSize::Bytes(512), BlockSize::default());
        unsafe { env::remove_var("POSIXLY_CORRECT") };
    }
}
