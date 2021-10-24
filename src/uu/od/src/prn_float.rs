use half::f16;
use std::f32;
use std::f64;
use std::num::FpCategory;

use crate::formatteriteminfo::*;

pub static FORMAT_ITEM_F16: FormatterItemInfo = FormatterItemInfo {
    byte_size: 2,
    print_width: 10,
    formatter: FormatWriter::FloatWriter(format_item_flo16),
};

pub static FORMAT_ITEM_F32: FormatterItemInfo = FormatterItemInfo {
    byte_size: 4,
    print_width: 15,
    formatter: FormatWriter::FloatWriter(format_item_flo32),
};

pub static FORMAT_ITEM_F64: FormatterItemInfo = FormatterItemInfo {
    byte_size: 8,
    print_width: 25,
    formatter: FormatWriter::FloatWriter(format_item_flo64),
};

pub fn format_item_flo16(f: f64) -> String {
    format!(" {}", format_flo16(f16::from_f64(f)))
}

pub fn format_item_flo32(f: f64) -> String {
    format!(" {}", format_flo32(f as f32))
}

pub fn format_item_flo64(f: f64) -> String {
    format!(" {}", format_flo64(f))
}

fn format_flo16(f: f16) -> String {
    format_float(f64::from(f), 9, 4)
}

// formats float with 8 significant digits, eg 12345678 or -1.2345678e+12
// always returns a string of 14 characters
fn format_flo32(f: f32) -> String {
    let width: usize = 14;
    let precision: usize = 8;

    if f.classify() == FpCategory::Subnormal {
        // subnormal numbers will be normal as f64, so will print with a wrong precision
        format!("{:width$e}", f, width = width) // subnormal numbers
    } else {
        format_float(f64::from(f), width, precision)
    }
}

fn format_flo64(f: f64) -> String {
    format_float(f, 24, 17)
}

fn format_float(f: f64, width: usize, precision: usize) -> String {
    if !f.is_normal() {
        if f == -0.0 && f.is_sign_negative() {
            return format!("{:>width$}", "-0", width = width);
        }
        if f == 0.0 || !f.is_finite() {
            return format!("{:width$}", f, width = width);
        }
        return format!("{:width$e}", f, width = width); // subnormal numbers
    }

    let mut l = f.abs().log10().floor() as i32;

    let r = 10f64.powi(l);
    if (f > 0.0 && r > f) || (f < 0.0 && -r < f) {
        // fix precision error
        l -= 1;
    }

    if l >= 0 && l <= (precision as i32 - 1) {
        format!(
            "{:width$.dec$}",
            f,
            width = width,
            dec = (precision - 1) - l as usize
        )
    } else if l == -1 {
        format!("{:width$.dec$}", f, width = width, dec = precision)
    } else {
        format!("{:width$.dec$e}", f, width = width, dec = precision - 1)
    }
}

#[test]
#[allow(clippy::excessive_precision)]
fn test_format_flo32() {
    assert_eq!(format_flo32(1.0), "     1.0000000");
    assert_eq!(format_flo32(9.9999990), "     9.9999990");
    assert_eq!(format_flo32(10.0), "     10.000000");
    assert_eq!(format_flo32(99.999977), "     99.999977");
    assert_eq!(format_flo32(99.999992), "     99.999992");
    assert_eq!(format_flo32(100.0), "     100.00000");
    assert_eq!(format_flo32(999.99994), "     999.99994");
    assert_eq!(format_flo32(1000.0), "     1000.0000");
    assert_eq!(format_flo32(9999.9990), "     9999.9990");
    assert_eq!(format_flo32(10000.0), "     10000.000");
    assert_eq!(format_flo32(99999.992), "     99999.992");
    assert_eq!(format_flo32(100000.0), "     100000.00");
    assert_eq!(format_flo32(999999.94), "     999999.94");
    assert_eq!(format_flo32(1000000.0), "     1000000.0");
    assert_eq!(format_flo32(9999999.0), "     9999999.0");
    assert_eq!(format_flo32(10000000.0), "      10000000");
    assert_eq!(format_flo32(99999992.0), "      99999992");
    assert_eq!(format_flo32(100000000.0), "   1.0000000e8");
    assert_eq!(format_flo32(9.9999994e8), "   9.9999994e8");
    assert_eq!(format_flo32(1.0e9), "   1.0000000e9");
    assert_eq!(format_flo32(9.9999990e9), "   9.9999990e9");
    assert_eq!(format_flo32(1.0e10), "  1.0000000e10");

    assert_eq!(format_flo32(0.1), "    0.10000000");
    assert_eq!(format_flo32(0.99999994), "    0.99999994");
    assert_eq!(format_flo32(0.010000001), "  1.0000001e-2");
    assert_eq!(format_flo32(0.099999994), "  9.9999994e-2");
    assert_eq!(format_flo32(0.001), "  1.0000000e-3");
    assert_eq!(format_flo32(0.0099999998), "  9.9999998e-3");

    assert_eq!(format_flo32(-1.0), "    -1.0000000");
    assert_eq!(format_flo32(-9.9999990), "    -9.9999990");
    assert_eq!(format_flo32(-10.0), "    -10.000000");
    assert_eq!(format_flo32(-99.999977), "    -99.999977");
    assert_eq!(format_flo32(-99.999992), "    -99.999992");
    assert_eq!(format_flo32(-100.0), "    -100.00000");
    assert_eq!(format_flo32(-999.99994), "    -999.99994");
    assert_eq!(format_flo32(-1000.0), "    -1000.0000");
    assert_eq!(format_flo32(-9999.9990), "    -9999.9990");
    assert_eq!(format_flo32(-10000.0), "    -10000.000");
    assert_eq!(format_flo32(-99999.992), "    -99999.992");
    assert_eq!(format_flo32(-100000.0), "    -100000.00");
    assert_eq!(format_flo32(-999999.94), "    -999999.94");
    assert_eq!(format_flo32(-1000000.0), "    -1000000.0");
    assert_eq!(format_flo32(-9999999.0), "    -9999999.0");
    assert_eq!(format_flo32(-10000000.0), "     -10000000");
    assert_eq!(format_flo32(-99999992.0), "     -99999992");
    assert_eq!(format_flo32(-100000000.0), "  -1.0000000e8");
    assert_eq!(format_flo32(-9.9999994e8), "  -9.9999994e8");
    assert_eq!(format_flo32(-1.0e9), "  -1.0000000e9");
    assert_eq!(format_flo32(-9.9999990e9), "  -9.9999990e9");
    assert_eq!(format_flo32(-1.0e10), " -1.0000000e10");

    assert_eq!(format_flo32(-0.1), "   -0.10000000");
    assert_eq!(format_flo32(-0.99999994), "   -0.99999994");
    assert_eq!(format_flo32(-0.010000001), " -1.0000001e-2");
    assert_eq!(format_flo32(-0.099999994), " -9.9999994e-2");
    assert_eq!(format_flo32(-0.001), " -1.0000000e-3");
    assert_eq!(format_flo32(-0.0099999998), " -9.9999998e-3");

    assert_eq!(format_flo32(3.4028233e38), "  3.4028233e38");
    assert_eq!(format_flo32(-3.4028233e38), " -3.4028233e38");
    assert_eq!(format_flo32(-1.1663108e-38), "-1.1663108e-38");
    assert_eq!(format_flo32(-4.7019771e-38), "-4.7019771e-38");
    assert_eq!(format_flo32(1e-45), "         1e-45");

    assert_eq!(format_flo32(-3.402823466e+38), " -3.4028235e38");
    assert_eq!(format_flo32(f32::NAN), "           NaN");
    assert_eq!(format_flo32(f32::INFINITY), "           inf");
    assert_eq!(format_flo32(f32::NEG_INFINITY), "          -inf");
    assert_eq!(format_flo32(-0.0), "            -0");
    assert_eq!(format_flo32(0.0), "             0");
}

#[test]
fn test_format_flo64() {
    assert_eq!(format_flo64(1.0), "      1.0000000000000000");
    assert_eq!(format_flo64(10.0), "      10.000000000000000");
    assert_eq!(format_flo64(1000000000000000.0), "      1000000000000000.0");
    assert_eq!(
        format_flo64(10000000000000000.0),
        "       10000000000000000"
    );
    assert_eq!(
        format_flo64(100000000000000000.0),
        "   1.0000000000000000e17"
    );

    assert_eq!(format_flo64(-0.1), "    -0.10000000000000001");
    assert_eq!(format_flo64(-0.01), "  -1.0000000000000000e-2");

    assert_eq!(
        format_flo64(-2.2250738585072014e-308),
        "-2.2250738585072014e-308"
    );
    assert_eq!(format_flo64(4e-320), "                  4e-320");
    assert_eq!(format_flo64(f64::NAN), "                     NaN");
    assert_eq!(format_flo64(f64::INFINITY), "                     inf");
    assert_eq!(format_flo64(f64::NEG_INFINITY), "                    -inf");
    assert_eq!(format_flo64(-0.0), "                      -0");
    assert_eq!(format_flo64(0.0), "                       0");
}

#[test]
fn test_format_flo16() {
    assert_eq!(format_flo16(f16::from_bits(0x8400u16)), "-6.104e-5");
    assert_eq!(format_flo16(f16::from_bits(0x8401u16)), "-6.109e-5");
    assert_eq!(format_flo16(f16::from_bits(0x8402u16)), "-6.115e-5");
    assert_eq!(format_flo16(f16::from_bits(0x8403u16)), "-6.121e-5");

    assert_eq!(format_flo16(f16::from_f32(1.0)), "    1.000");
    assert_eq!(format_flo16(f16::from_f32(10.0)), "    10.00");
    assert_eq!(format_flo16(f16::from_f32(100.0)), "    100.0");
    assert_eq!(format_flo16(f16::from_f32(1000.0)), "     1000");
    assert_eq!(format_flo16(f16::from_f32(10000.0)), "  1.000e4");

    assert_eq!(format_flo16(f16::from_f32(-0.2)), "  -0.2000");
    assert_eq!(format_flo16(f16::from_f32(-0.02)), "-2.000e-2");

    assert_eq!(format_flo16(f16::MIN_POSITIVE_SUBNORMAL), " 5.960e-8");
    assert_eq!(format_flo16(f16::MIN), " -6.550e4");
    assert_eq!(format_flo16(f16::NAN), "      NaN");
    assert_eq!(format_flo16(f16::INFINITY), "      inf");
    assert_eq!(format_flo16(f16::NEG_INFINITY), "     -inf");
    assert_eq!(format_flo16(f16::NEG_ZERO), "       -0");
    assert_eq!(format_flo16(f16::ZERO), "        0");
}
