// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore gnulibs sfmt

//! `human`-size formatting
//!
//! Format sizes like gnulibs human_readable() would

use number_prefix::NumberPrefix;

#[derive(Copy, Clone, PartialEq)]
pub enum SizeFormat {
    Bytes,
    Binary,  // Powers of 1024, --human-readable, -h
    Decimal, // Powers of 1000, --si
}

// There are a few peculiarities to how GNU formats the sizes:
// 1. One decimal place is given if and only if the size is smaller than 10
// 2. It rounds sizes up.
// 3. The human-readable format uses powers for 1024, but does not display the "i"
//    that is commonly used to denote Kibi, Mebi, etc.
// 4. Kibi and Kilo are denoted differently ("k" and "K", respectively)
fn format_prefixed(prefixed: &NumberPrefix<f64>) -> String {
    match prefixed {
        NumberPrefix::Standalone(bytes) => bytes.to_string(),
        NumberPrefix::Prefixed(prefix, bytes) => {
            // Remove the "i" from "Ki", "Mi", etc. if present
            let prefix_str = prefix.symbol().trim_end_matches('i');

            // Check whether we get more than 10 if we round up to the first decimal
            // because we want do display 9.81 as "9.9", not as "10".
            if (10.0 * bytes).ceil() >= 100.0 {
                format!("{:.0}{}", bytes.ceil(), prefix_str)
            } else {
                format!("{:.1}{}", (10.0 * bytes).ceil() / 10.0, prefix_str)
            }
        }
    }
}

pub fn human_readable(size: u64, sfmt: SizeFormat) -> String {
    match sfmt {
        SizeFormat::Binary => format_prefixed(&NumberPrefix::binary(size as f64)),
        SizeFormat::Decimal => format_prefixed(&NumberPrefix::decimal(size as f64)),
        SizeFormat::Bytes => size.to_string(),
    }
}

#[cfg(test)]
#[test]
fn test_human_readable() {
    let test_cases = [
        (133456345, SizeFormat::Binary, "128M"),
        (12 * 1024 * 1024, SizeFormat::Binary, "12M"),
        (8500, SizeFormat::Binary, "8.4K"),
    ];

    for &(size, sfmt, expected_str) in &test_cases {
        assert_eq!(human_readable(size, sfmt), expected_str);
    }
}
