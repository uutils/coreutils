// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
/// Functions for formatting a number as a magnitude and a unit suffix.

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

const IEC_SUFFIXES: [&str; 9] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB"];

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

const SI_SUFFIXES: [&str; 9] = ["B", "kB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];

/// A SuffixType determines whether the suffixes are 1000 or 1024 based.
#[derive(Clone, Copy)]
pub(crate) enum SuffixType {
    Iec,
    Si,
}

impl SuffixType {
    fn base_and_suffix(&self, n: u128) -> (u128, &'static str) {
        let (bases, suffixes) = match self {
            Self::Iec => (IEC_BASES, IEC_SUFFIXES),
            Self::Si => (SI_BASES, SI_SUFFIXES),
        };
        let mut i = 0;
        while bases[i + 1] - bases[i] < n && i < suffixes.len() {
            i += 1;
        }
        (bases[i], suffixes[i])
    }
}

/// Convert a number into a magnitude and a multi-byte unit suffix.
///
/// The returned string has a maximum length of 5 chars, for example: "1.1kB", "999kB", "1MB".
pub(crate) fn to_magnitude_and_suffix(n: u128, suffix_type: SuffixType) -> String {
    let (base, suffix) = suffix_type.base_and_suffix(n);
    // TODO To match dd on my machine, we would need to round like
    // this:
    //
    // 1049 => 1.0 kB
    // 1050 => 1.0 kB  # why is this different?
    // 1051 => 1.1 kB
    // ...
    // 1149 => 1.1 kB
    // 1150 => 1.2 kB
    // ...
    // 1250 => 1.2 kB
    // 1251 => 1.3 kB
    // ..
    // 10500 => 10 kB
    // 10501 => 11 kB
    //
    let quotient = (n as f64) / (base as f64);
    if quotient < 10.0 {
        format!("{quotient:.1} {suffix}")
    } else {
        format!("{} {}", quotient.round(), suffix)
    }
}

#[cfg(test)]
mod tests {

    use crate::numbers::{to_magnitude_and_suffix, SuffixType};

    #[test]
    fn test_to_magnitude_and_suffix_powers_of_1024() {
        assert_eq!(to_magnitude_and_suffix(1024, SuffixType::Iec), "1.0 KiB");
        assert_eq!(to_magnitude_and_suffix(2048, SuffixType::Iec), "2.0 KiB");
        assert_eq!(to_magnitude_and_suffix(4096, SuffixType::Iec), "4.0 KiB");
        assert_eq!(
            to_magnitude_and_suffix(1024 * 1024, SuffixType::Iec),
            "1.0 MiB"
        );
        assert_eq!(
            to_magnitude_and_suffix(2 * 1024 * 1024, SuffixType::Iec),
            "2.0 MiB"
        );
        assert_eq!(
            to_magnitude_and_suffix(1024 * 1024 * 1024, SuffixType::Iec),
            "1.0 GiB"
        );
        assert_eq!(
            to_magnitude_and_suffix(34 * 1024 * 1024 * 1024, SuffixType::Iec),
            "34 GiB"
        );
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn test_to_magnitude_and_suffix_not_powers_of_1024() {
        assert_eq!(to_magnitude_and_suffix(1, SuffixType::Si), "1.0 B");
        assert_eq!(to_magnitude_and_suffix(999, SuffixType::Si), "999 B");

        assert_eq!(to_magnitude_and_suffix(1000, SuffixType::Si), "1.0 kB");
        assert_eq!(to_magnitude_and_suffix(1001, SuffixType::Si), "1.0 kB");
        assert_eq!(to_magnitude_and_suffix(1023, SuffixType::Si), "1.0 kB");
        assert_eq!(to_magnitude_and_suffix(1025, SuffixType::Si), "1.0 kB");
        assert_eq!(to_magnitude_and_suffix(10_001, SuffixType::Si), "10 kB");
        assert_eq!(to_magnitude_and_suffix(999_000, SuffixType::Si), "999 kB");

        assert_eq!(to_magnitude_and_suffix(999_001, SuffixType::Si), "1.0 MB");
        assert_eq!(to_magnitude_and_suffix(999_999, SuffixType::Si), "1.0 MB");
        assert_eq!(to_magnitude_and_suffix(1_000_000, SuffixType::Si), "1.0 MB");
        assert_eq!(to_magnitude_and_suffix(1_000_001, SuffixType::Si), "1.0 MB");
        assert_eq!(to_magnitude_and_suffix(1_100_000, SuffixType::Si), "1.1 MB");
        assert_eq!(to_magnitude_and_suffix(1_100_001, SuffixType::Si), "1.1 MB");
        assert_eq!(to_magnitude_and_suffix(1_900_000, SuffixType::Si), "1.9 MB");
        assert_eq!(to_magnitude_and_suffix(1_900_001, SuffixType::Si), "1.9 MB");
        assert_eq!(to_magnitude_and_suffix(9_900_000, SuffixType::Si), "9.9 MB");
        assert_eq!(to_magnitude_and_suffix(9_900_001, SuffixType::Si), "9.9 MB");
        assert_eq!(
            to_magnitude_and_suffix(999_000_000, SuffixType::Si),
            "999 MB"
        );

        assert_eq!(
            to_magnitude_and_suffix(999_000_001, SuffixType::Si),
            "1.0 GB"
        );
        assert_eq!(
            to_magnitude_and_suffix(1_000_000_000, SuffixType::Si),
            "1.0 GB"
        );
        assert_eq!(
            to_magnitude_and_suffix(1_000_000_001, SuffixType::Si),
            "1.0 GB"
        );
    }
}
