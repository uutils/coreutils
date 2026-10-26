// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore dtoastr ftoastr

use half::{bf16, f16};

use crate::formatter_item_info::{FormatWriter, FormatterItemInfo};

pub static FORMAT_ITEM_F16: FormatterItemInfo = FormatterItemInfo {
    byte_size: 2,
    print_width: 16,
    formatter: FormatWriter::FloatWriter(format_item_f16),
};

pub static FORMAT_ITEM_F32: FormatterItemInfo = FormatterItemInfo {
    byte_size: 4,
    print_width: 16,
    formatter: FormatWriter::FloatWriter(format_item_f32),
};

pub static FORMAT_ITEM_F64: FormatterItemInfo = FormatterItemInfo {
    byte_size: 8,
    print_width: 25,
    formatter: FormatWriter::FloatWriter(format_item_f64),
};

pub static FORMAT_ITEM_LONG_DOUBLE: FormatterItemInfo = FormatterItemInfo {
    byte_size: 16,
    print_width: 40,
    formatter: FormatWriter::LongDoubleWriter(format_item_long_double),
};

pub static FORMAT_ITEM_BF16: FormatterItemInfo = FormatterItemInfo {
    byte_size: 2,
    print_width: 16,
    formatter: FormatWriter::BFloatWriter(format_item_bf16),
};

pub fn format_item_f16(f: f64) -> String {
    // GNU widens half precision to float before printing it.
    format!(" {}", format_f32(f32::from(f16::from_f64(f))))
}

pub fn format_item_f32(f: f64) -> String {
    format!(" {}", format_f32(f as f32))
}

pub fn format_item_f64(f: f64) -> String {
    format!(" {}", format_f64(f))
}

pub fn format_item_long_double(f: f64) -> String {
    format!(" {}", format_long_double(f))
}

pub fn format_item_bf16(f: f64) -> String {
    format!(" {}", format_f32(f32::from(bf16::from_f32(f as f32))))
}

/// Right-padded to 15 chars (`format_item_f32` prepends one space -> 16 total).
fn format_f32(f: f32) -> String {
    let s = gnu_float_repr(&shortest_f32(f), f.is_nan(), f.is_sign_negative(), 6);
    format!("{s:>15}")
}

/// Right-padded to 24 chars (`format_item_f64` prepends one space -> 25 total).
fn format_f64(f: f64) -> String {
    let s = gnu_float_repr(&shortest_f64(f), f.is_nan(), f.is_sign_negative(), 15);
    format!("{s:>24}")
}

/// The fewest significant digits that read back as `f`, in `{:e}` form,
/// rounded to nearest with ties to even like GNU. `{:e}` alone may pick
/// different digits: it rounds ties up, and next to a power of two it can
/// find a shorter string that is not the correctly rounded one. Its digit
/// count is still a lower bound, so start there and add digits until the
/// correctly rounded string reads back as `f`.
macro_rules! shortest_round_trip {
    ($name:ident, $t:ty) => {
        fn $name(f: $t) -> String {
            let shortest = format!("{f:e}");
            let Some(digits) = significant_digits(&shortest) else {
                return shortest;
            };
            let mut precision = digits - 1;
            let mut rounded = format!("{f:.precision$e}");
            // Equal to `shortest` means it is known to read back as `f`.
            while rounded != shortest && rounded.parse::<$t>() != Ok(f) {
                precision += 1;
                rounded = format!("{f:.precision$e}");
            }
            rounded
        }
    };
}

shortest_round_trip!(shortest_f32, f32);
shortest_round_trip!(shortest_f64, f64);

/// Number of mantissa digits in `{:e}` output, `None` for inf and NaN.
fn significant_digits(sci: &str) -> Option<usize> {
    let (mantissa, _) = sci.split_once('e')?;
    Some(mantissa.chars().filter(char::is_ascii_digit).count())
}

/// Lay out the shortest round-trip digits the way GNU od does: like `%g`
/// with exactly those digits, switching to scientific notation when the
/// exponent is below -4 or at least `max(digits, dig)`. `dig` is the number
/// of decimal digits the type always preserves (6 for f32, 15 for f64).
///
/// `shortest` is in `{:e}` form, e.g. `1.5e-7` or `-2e10`.
fn gnu_float_repr(shortest: &str, is_nan: bool, negative: bool, dig: i32) -> String {
    if is_nan {
        return if negative { "-nan" } else { "nan" }.to_string();
    }
    let Some((mantissa, exp)) = shortest.split_once('e') else {
        // inf and -inf
        return shortest.to_string();
    };
    let exp: i32 = exp.parse().unwrap_or(0);
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let sign = if negative { "-" } else { "" };
    if digits == "0" {
        return format!("{sign}0");
    }

    let len = digits.len() as i32;
    if exp < -4 || exp >= len.max(dig) {
        let (first, rest) = digits.split_at(1);
        let dot = if rest.is_empty() { "" } else { "." };
        let exp_sign = if exp < 0 { '-' } else { '+' };
        format!("{sign}{first}{dot}{rest}e{exp_sign}{:02}", exp.abs())
    } else if exp < 0 {
        let zeros = "0".repeat((-exp - 1) as usize);
        format!("{sign}0.{zeros}{digits}")
    } else if exp + 1 >= len {
        let zeros = "0".repeat((exp + 1 - len) as usize);
        format!("{sign}{digits}{zeros}")
    } else {
        let (int, frac) = digits.split_at((exp + 1) as usize);
        format!("{sign}{int}.{frac}")
    }
}

fn format_long_double(f: f64) -> String {
    // On most platforms, long double is either 64-bit (same as f64) or 80-bit/128-bit
    // Since we're reading it as f64, we format it with extended precision
    // Width is 39 (40 - 1 for leading space), precision is 21 significant digits
    let width: usize = 39;
    let precision: usize = 21;

    // Handle special cases
    if f.is_nan() {
        return format!("{:>width$}", "NaN");
    }
    if f.is_infinite() {
        if f.is_sign_negative() {
            return format!("{:>width$}", "-inf");
        }
        return format!("{:>width$}", "inf");
    }
    if f == 0.0 {
        if f.is_sign_negative() {
            return format!("{:>width$}", "-0");
        }
        return format!("{:>width$}", "0");
    }

    // For normal numbers, format with appropriate precision using exponential notation
    format!("{f:>width$.precision$e}")
}

#[test]
#[allow(clippy::excessive_precision)]
fn test_format_f32() {
    // Expected values are GNU od 9.12 output.
    assert_eq!(format_f32(1.0), "              1");
    assert_eq!(format_f32(100_000.0), "         100000");
    assert_eq!(format_f32(1_000_000.0), "          1e+06");
    assert_eq!(format_f32(9_999_999.0), "        9999999");
    assert_eq!(format_f32(99_999_992.0), "   9.999999e+07");
    assert_eq!(format_f32(1.0e10), "          1e+10");

    assert_eq!(format_f32(0.1), "            0.1");
    assert_eq!(format_f32(1e-4_f32), "         0.0001");
    assert_eq!(format_f32(1e-5_f32), "          1e-05");

    assert_eq!(format_f32(-10.0), "            -10");
    assert_eq!(format_f32(-0.1), "           -0.1");
    assert_eq!(format_f32(3.402_823_3e38), "  3.4028233e+38");
    assert_eq!(format_f32(-3.402_823_3e38), " -3.4028233e+38");
    assert_eq!(format_f32(1e-45), "          1e-45");

    // An exact tie between two shortest candidates rounds to even.
    assert_eq!(format_f32(1.863_281_25), "      1.8632812");
    // 2^90: the correctly rounded 8 digits do not read back, so 9 are needed.
    assert_eq!(format_f32(2f32.powi(90)), " 1.23794004e+27");

    assert_eq!(format_f32(f32::from_bits(0x7fc0_0000)), "            nan");
    assert_eq!(format_f32(f32::from_bits(0xffc0_0000)), "           -nan");
    assert_eq!(format_f32(f32::INFINITY), "            inf");
    assert_eq!(format_f32(f32::NEG_INFINITY), "           -inf");
    assert_eq!(format_f32(-0.0), "             -0");
    assert_eq!(format_f32(0.0), "              0");
}

#[test]
fn test_format_f64() {
    // Expected values are GNU od 9.12 output.
    assert_eq!(format_f64(1.0), "                       1");
    assert_eq!(
        format_f64(100_000_000_000_000.0),
        "         100000000000000"
    );
    assert_eq!(
        format_f64(1_000_000_000_000_000.0),
        "                   1e+15"
    );
    assert_eq!(format_f64(-0.01), "                   -0.01");
    assert_eq!(format_f64(1.5e-9), "                 1.5e-09");

    // od-float.sh test 7: -t f / -t fD with 8-byte little-endian input
    let d = f64::from_bits(u64::from_le_bytes([
        0x00, 0x00, 0x80, 0x3f, 0x00, 0x00, 0x00, 0x40,
    ]));
    assert_eq!(format_f64(d), "       2.000000473111868");

    assert_eq!(
        format_f64(-2.225_073_858_507_201_4e-308),
        "-2.2250738585072014e-308"
    );
    assert_eq!(format_f64(4e-320), "                  4e-320");
    assert_eq!(format_f64(-f64::NAN), "                    -nan");
    assert_eq!(format_f64(f64::NEG_INFINITY), "                    -inf");
    assert_eq!(format_f64(-0.0), "                      -0");
}

#[test]
fn test_format_item_f16() {
    // Expected values are GNU od 9.12 output; GNU prints half floats as float.
    let f16_item = |bits| format_item_f16(f64::from(f16::from_bits(bits)));
    assert_eq!(f16_item(0x8400), "  -6.1035156e-05");
    assert_eq!(f16_item(0x8401), "   -6.109476e-05");
    assert_eq!(f16_item(0x3c00), "               1");
    assert_eq!(f16_item(0x70e2), "           10000");
    assert_eq!(f16_item(0xb266), "     -0.19995117");
    assert_eq!(f16_item(0xa51f), "    -0.020004272");
    assert_eq!(f16_item(0x0001), "   5.9604645e-08");
    assert_eq!(f16_item(0xfbff), "          -65504");
    assert_eq!(f16_item(0x3f74), "       1.8632812");
    assert_eq!(f16_item(0x7e00), "             nan");
    assert_eq!(f16_item(0xfe00), "            -nan");
    assert_eq!(f16_item(0x8000), "              -0");
}

#[test]
fn test_format_item_bf16() {
    let bf16_item = |bits| format_item_bf16(f64::from(bf16::from_bits(bits)));
    assert_eq!(bf16_item(0x3f80), "               1");
    assert_eq!(bf16_item(0x6c80), "  1.23794004e+27");
    assert_eq!(bf16_item(0xffc0), "            -nan");
}
