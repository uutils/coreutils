use half::f16;
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

pub static FORMAT_ITEM_BF16: FormatterItemInfo = FormatterItemInfo {
    byte_size: 2,
    print_width: 16,
    formatter: FormatWriter::BFloatWriter(format_item_bf16),
};

pub fn format_item_f16(f: f64) -> String {
    format!(" {}", format_f16(f16::from_f64(f)))
}

pub fn format_item_f32(f: f64) -> String {
    format!(" {}", format_f32(f as f32))
}

pub fn format_item_f64(f: f64) -> String {
    format!(" {}", format_f64(f))
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
    format!(" {}", format_f32(f as f32))
}

fn format_f16(f: f16) -> String {
    format_float(f64::from(f), 15, 8)
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

#[test]
#[allow(clippy::excessive_precision)]
#[allow(clippy::cognitive_complexity)]
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
#[allow(clippy::cognitive_complexity)]
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
#[allow(clippy::cognitive_complexity)]
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
