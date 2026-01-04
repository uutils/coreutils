// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use half::{bf16, f16};
use std::num::FpCategory;

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

/// Clean up a normalized float string by removing unnecessary padding and digits.
/// - Strip leading spaces.
/// - Trim trailing zeros after the decimal point (and the dot itself if empty).
/// - Leave the exponent part (e/E...) untouched.
fn trim_float_repr(raw: &str) -> String {
    // Drop padding added by `format!` width specification
    let mut s = raw.trim_start().to_string();

    // Keep NaN/Inf representations as-is
    let lower = s.to_ascii_lowercase();
    if lower == "nan" || lower == "inf" || lower == "-inf" {
        return s;
    }

    // Separate exponent from mantissa
    let mut exp_part = String::new();
    if let Some(idx) = s.find(['e', 'E']) {
        exp_part = s[idx..].to_string();
        s.truncate(idx);
    }

    // Trim trailing zeros in mantissa, then remove trailing dot if left alone
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }

    // If everything was trimmed, leave a single zero
    if s.is_empty() || s == "-" || s == "+" {
        s.push('0');
    }

    s.push_str(&exp_part);
    s
}

/// Pad a floating value to a fixed width for column alignment while keeping
/// the original precision (including trailing zeros). This mirrors the
/// behavior of other float formatters (`f32`, `f64`) and keeps the output
/// stable across platforms.
fn pad_float_repr(raw: &str, width: usize) -> String {
    format!("{raw:>width$}")
}

pub fn format_item_f16(f: f64) -> String {
    let value = f16::from_f64(f);
    let width = FORMAT_ITEM_F16.print_width - 1;
    // Format once, trim redundant zeros, then re-pad to the canonical width
    let raw = format_f16(value);
    let trimmed = trim_float_repr(&raw);
    format!(" {}", pad_float_repr(&trimmed, width))
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

fn format_f32_exp(f: f32, width: usize) -> String {
    if f.abs().log10() < 0.0 {
        return format!("{f:width$e}");
    }
    // Leave room for the '+' sign
    let formatted = format!("{f:width$e}", width = width - 1);
    formatted.replace('e', "e+")
}

fn format_f64_exp(f: f64, width: usize) -> String {
    if f.abs().log10() < 0.0 {
        return format!("{f:width$e}");
    }
    // Leave room for the '+' sign
    let formatted = format!("{f:width$e}", width = width - 1);
    formatted.replace('e', "e+")
}

fn format_f64_exp_precision(f: f64, width: usize, precision: usize) -> String {
    if f.abs().log10() < 0.0 {
        return format!("{f:width$.precision$e}");
    }
    // Leave room for the '+' sign
    let formatted = format!("{f:width$.precision$e}", width = width - 1);
    formatted.replace('e', "e+")
}

pub fn format_item_bf16(f: f64) -> String {
    let bf = bf16::from_f32(f as f32);
    let width = FORMAT_ITEM_BF16.print_width - 1;
    let raw = format_binary16_like(f64::from(bf), width, 8, is_subnormal_bf16(bf));
    let trimmed = trim_float_repr(&raw);
    format!(" {}", pad_float_repr(&trimmed, width))
}

fn format_f16(f: f16) -> String {
    let value = f64::from(f);
    format_binary16_like(value, 15, 8, is_subnormal_f16(f))
}

fn format_binary16_like(value: f64, width: usize, precision: usize, force_exp: bool) -> String {
    if force_exp {
        return format_f64_exp_precision(value, width, precision - 1);
    }
    format_float(value, width, precision)
}

fn is_subnormal_f16(value: f16) -> bool {
    let bits = value.to_bits();
    (bits & 0x7C00) == 0 && (bits & 0x03FF) != 0
}

fn is_subnormal_bf16(value: bf16) -> bool {
    let bits = value.to_bits();
    (bits & 0x7F80) == 0 && (bits & 0x007F) != 0
}

/// formats float with 8 significant digits, eg 12345678 or -1.2345678e+12
/// always returns a string of 14 characters
fn format_f32(f: f32) -> String {
    let width: usize = 15;
    let precision: usize = 8;

    if f.classify() == FpCategory::Subnormal {
        // subnormal numbers will be normal as f64, so will print with a wrong precision
        format_f32_exp(f, width) // subnormal numbers
    } else {
        format_float(f64::from(f), width, precision)
    }
}

fn format_f64(f: f64) -> String {
    format_float(f, 24, 17)
}

fn format_float(f: f64, width: usize, precision: usize) -> String {
    if !f.is_normal() {
        if f == -0.0 && f.is_sign_negative() {
            return format!("{:>width$}", "-0");
        }
        if f == 0.0 || !f.is_finite() {
            return format!("{f:width$}");
        }
        return format_f64_exp(f, width); // subnormal numbers
    }

    let mut l = f.abs().log10().floor() as i32;

    let r = 10f64.powi(l);
    if (f > 0.0 && r > f) || (f < 0.0 && -r < f) {
        // fix precision error
        l -= 1;
    }

    if l >= 0 && l <= (precision as i32 - 1) {
        format!("{f:width$.dec$}", dec = (precision - 1) - l as usize)
    } else if l == -1 {
        format!("{f:width$.precision$}")
    } else {
        format_f64_exp_precision(f, width, precision - 1) // subnormal numbers
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
    assert_eq!(format_f32(1.0), "      1.0000000");
    assert_eq!(format_f32(9.999_999_0), "      9.9999990");
    assert_eq!(format_f32(10.0), "      10.000000");
    assert_eq!(format_f32(99.999_977), "      99.999977");
    assert_eq!(format_f32(99.999_992), "      99.999992");
    assert_eq!(format_f32(100.0), "      100.00000");
    assert_eq!(format_f32(999.99994), "      999.99994");
    assert_eq!(format_f32(1000.0), "      1000.0000");
    assert_eq!(format_f32(9999.9990), "      9999.9990");
    assert_eq!(format_f32(10000.0), "      10000.000");
    assert_eq!(format_f32(99999.992), "      99999.992");
    assert_eq!(format_f32(100_000.0), "      100000.00");
    assert_eq!(format_f32(999_999.94), "      999999.94");
    assert_eq!(format_f32(1_000_000.0), "      1000000.0");
    assert_eq!(format_f32(9_999_999.0), "      9999999.0");
    assert_eq!(format_f32(10_000_000.0), "       10000000");
    assert_eq!(format_f32(99_999_992.0), "       99999992");
    assert_eq!(format_f32(100_000_000.0), "   1.0000000e+8");
    assert_eq!(format_f32(9.999_999_4e8), "   9.9999994e+8");
    assert_eq!(format_f32(1.0e9), "   1.0000000e+9");
    assert_eq!(format_f32(9.999_999_0e9), "   9.9999990e+9");
    assert_eq!(format_f32(1.0e10), "  1.0000000e+10");

    assert_eq!(format_f32(0.1), "     0.10000000");
    assert_eq!(format_f32(0.999_999_94), "     0.99999994");
    assert_eq!(format_f32(0.010_000_001), "   1.0000001e-2");
    assert_eq!(format_f32(0.099_999_994), "   9.9999994e-2");
    assert_eq!(format_f32(0.001), "   1.0000000e-3");
    assert_eq!(format_f32(0.009_999_999_8), "   9.9999998e-3");

    assert_eq!(format_f32(-1.0), "     -1.0000000");
    assert_eq!(format_f32(-9.999_999_0), "     -9.9999990");
    assert_eq!(format_f32(-10.0), "     -10.000000");
    assert_eq!(format_f32(-99.999_977), "     -99.999977");
    assert_eq!(format_f32(-99.999_992), "     -99.999992");
    assert_eq!(format_f32(-100.0), "     -100.00000");
    assert_eq!(format_f32(-999.99994), "     -999.99994");
    assert_eq!(format_f32(-1000.0), "     -1000.0000");
    assert_eq!(format_f32(-9999.9990), "     -9999.9990");
    assert_eq!(format_f32(-10000.0), "     -10000.000");
    assert_eq!(format_f32(-99999.992), "     -99999.992");
    assert_eq!(format_f32(-100_000.0), "     -100000.00");
    assert_eq!(format_f32(-999_999.94), "     -999999.94");
    assert_eq!(format_f32(-1_000_000.0), "     -1000000.0");
    assert_eq!(format_f32(-9_999_999.0), "     -9999999.0");
    assert_eq!(format_f32(-10_000_000.0), "      -10000000");
    assert_eq!(format_f32(-99_999_992.0), "      -99999992");
    assert_eq!(format_f32(-100_000_000.0), "  -1.0000000e+8");
    assert_eq!(format_f32(-9.999_999_4e8), "  -9.9999994e+8");
    assert_eq!(format_f32(-1.0e9), "  -1.0000000e+9");
    assert_eq!(format_f32(-9.999_999_0e9), "  -9.9999990e+9");
    assert_eq!(format_f32(-1.0e10), " -1.0000000e+10");

    assert_eq!(format_f32(-0.1), "    -0.10000000");
    assert_eq!(format_f32(-0.999_999_94), "    -0.99999994");
    assert_eq!(format_f32(-0.010_000_001), "  -1.0000001e-2");
    assert_eq!(format_f32(-0.099_999_994), "  -9.9999994e-2");
    assert_eq!(format_f32(-0.001), "  -1.0000000e-3");
    assert_eq!(format_f32(-0.009_999_999_8), "  -9.9999998e-3");

    assert_eq!(format_f32(3.402_823_3e38), "  3.4028233e+38");
    assert_eq!(format_f32(-3.402_823_3e38), " -3.4028233e+38");
    assert_eq!(format_f32(-1.166_310_8e-38), " -1.1663108e-38");
    assert_eq!(format_f32(-4.701_977_1e-38), " -4.7019771e-38");
    assert_eq!(format_f32(1e-45), "          1e-45");

    assert_eq!(format_f32(-3.402_823_466e+38), " -3.4028235e+38");
    assert_eq!(format_f32(f32::NAN), "            NaN");
    assert_eq!(format_f32(f32::INFINITY), "            inf");
    assert_eq!(format_f32(f32::NEG_INFINITY), "           -inf");
    assert_eq!(format_f32(-0.0), "             -0");
    assert_eq!(format_f32(0.0), "              0");
}

#[test]
fn test_format_f64() {
    assert_eq!(format_f64(1.0), "      1.0000000000000000");
    assert_eq!(format_f64(10.0), "      10.000000000000000");
    assert_eq!(
        format_f64(1_000_000_000_000_000.0),
        "      1000000000000000.0"
    );
    assert_eq!(
        format_f64(10_000_000_000_000_000.0),
        "       10000000000000000"
    );
    assert_eq!(
        format_f64(100_000_000_000_000_000.0),
        "  1.0000000000000000e+17"
    );

    assert_eq!(format_f64(-0.1), "    -0.10000000000000001");
    assert_eq!(format_f64(-0.01), "  -1.0000000000000000e-2");

    assert_eq!(
        format_f64(-2.225_073_858_507_201_4e-308),
        "-2.2250738585072014e-308"
    );
    assert_eq!(format_f64(4e-320), "                  4e-320");
    assert_eq!(format_f64(f64::NAN), "                     NaN");
    assert_eq!(format_f64(f64::INFINITY), "                     inf");
    assert_eq!(format_f64(f64::NEG_INFINITY), "                    -inf");
    assert_eq!(format_f64(-0.0), "                      -0");
    assert_eq!(format_f64(0.0), "                       0");
}

#[test]
fn test_format_f16() {
    assert_eq!(format_f16(f16::from_bits(0x8400u16)), "  -6.1035156e-5");
    assert_eq!(format_f16(f16::from_bits(0x8401u16)), "  -6.1094761e-5");
    assert_eq!(format_f16(f16::from_bits(0x8402u16)), "  -6.1154366e-5");
    assert_eq!(format_f16(f16::from_bits(0x8403u16)), "  -6.1213970e-5");

    assert_eq!(format_f16(f16::from_f32(1.0)), "      1.0000000");
    assert_eq!(format_f16(f16::from_f32(10.0)), "      10.000000");
    assert_eq!(format_f16(f16::from_f32(100.0)), "      100.00000");
    assert_eq!(format_f16(f16::from_f32(1000.0)), "      1000.0000");
    assert_eq!(format_f16(f16::from_f32(10000.0)), "      10000.000");

    assert_eq!(format_f16(f16::from_f32(-0.2)), "    -0.19995117");
    assert_eq!(format_f16(f16::from_f32(-0.02)), "  -2.0004272e-2");

    assert_eq!(format_f16(f16::MIN_POSITIVE_SUBNORMAL), "   5.9604645e-8");
    assert_eq!(format_f16(f16::MIN), "     -65504.000");
    assert_eq!(format_f16(f16::NAN), "            NaN");
    assert_eq!(format_f16(f16::INFINITY), "            inf");
    assert_eq!(format_f16(f16::NEG_INFINITY), "           -inf");
    assert_eq!(format_f16(f16::NEG_ZERO), "             -0");
    assert_eq!(format_f16(f16::ZERO), "              0");
}
