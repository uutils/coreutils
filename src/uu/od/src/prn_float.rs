// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore FLT DBL subnormals

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

/// The width of a `float`, in significant decimal digits, that `printf`'s `%g`
/// uses by default (`FLT_DIG`).
const FLOAT_DIG: usize = 6;
/// The same for a `double` (`DBL_DIG`).
const DOUBLE_DIG: usize = 15;
/// Digits needed to round-trip any `float` through a decimal string.
const FLOAT_MAX_DIG: usize = 9;
/// The same for a `double`.
const DOUBLE_MAX_DIG: usize = 17;

/// The floating point type a value is rendered as.
///
/// Half precision values (`-t f2`, `fH` and `fB`) are widened to `float`, which
/// is lossless, and share its formatting.
#[derive(Clone, Copy)]
enum FloatKind {
    Single,
    Double,
}

impl FloatKind {
    /// Minimum number of significant digits used to decide between fixed and
    /// scientific notation.
    fn min_digits(self) -> usize {
        match self {
            Self::Single => FLOAT_DIG,
            Self::Double => DOUBLE_DIG,
        }
    }

    /// Digits guaranteed to round-trip a value of this type.
    fn max_digits(self) -> usize {
        match self {
            Self::Single => FLOAT_MAX_DIG,
            Self::Double => DOUBLE_MAX_DIG,
        }
    }

    /// Whether `repr` parses back to exactly `value` at this precision.
    fn round_trips(self, repr: &str, value: f64) -> bool {
        match self {
            Self::Single => repr
                .parse::<f32>()
                .is_ok_and(|parsed| parsed == value as f32),
            Self::Double => repr.parse::<f64>().is_ok_and(|parsed| parsed == value),
        }
    }

    /// Rust's shortest round-tripping form, in scientific notation.
    ///
    /// Formatting at the value's own width matters: a `float` widened to `f64`
    /// would render the double's digits — 16 for `0.01f32`, not 1.
    fn shortest_repr(self, value: f64) -> String {
        match self {
            Self::Single => format!("{:e}", value as f32),
            Self::Double => format!("{value:e}"),
        }
    }
}

/// Significant digits in a rendered mantissa, e.g. 3 for `-1.25e2`.
fn count_digits(scientific: &str) -> usize {
    let mantissa = scientific
        .split_once('e')
        .map_or(scientific, |(mantissa, _)| mantissa);
    mantissa.chars().filter(char::is_ascii_digit).count().max(1)
}

/// The fewest significant digits whose correctly rounded decimal reproduces
/// `value` exactly — the precision GNU renders at.
///
/// Rust's shortest form is only a *lower bound* here, not the answer. Rust may
/// pick any decimal in the value's rounding interval, whereas `%g` always emits
/// the correctly rounded one for a given precision, and that one occasionally
/// fails to round-trip where a neighbor would. The f32 nearest 2^-96 is such a
/// case: Rust writes it in eight digits as 1.2621775e-29, but `%.8g` rounds to
/// 1.2621774e-29, which reads back as a different float, so GNU needs nine.
fn significant_digits(value: f64, kind: FloatKind) -> usize {
    let shortest = kind.shortest_repr(value);
    let max = kind.max_digits();
    // Starting at the lower bound means the first candidate is nearly always
    // the answer, so this loop typically runs a single iteration.
    (count_digits(&shortest)..max)
        .find(|digits| kind.round_trips(&format!("{value:.*e}", digits - 1), value))
        .unwrap_or(max)
}

/// Render `value` the way GNU `od` does.
///
/// GNU prints the shortest decimal representation that round-trips, laid out
/// with `printf`'s `%g` rules: scientific notation when the decimal exponent is
/// below -4 or at least the working precision, fixed notation otherwise, with
/// trailing zeros stripped either way. The one wrinkle is that the choice
/// between the two uses at least `FLT_DIG`/`DBL_DIG` digits even when fewer
/// suffice to round-trip, so e.g. a `float` 1e5 stays `100000` while 1e6
/// becomes `1e+06`.
fn format_float(value: f64, kind: FloatKind) -> String {
    if value.is_nan() {
        // GNU keeps the sign of a NaN but not its payload.
        return if value.is_sign_negative() {
            "-nan".into()
        } else {
            "nan".into()
        };
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-inf".into()
        } else {
            "inf".into()
        };
    }
    if value == 0.0 {
        return if value.is_sign_negative() {
            "-0".into()
        } else {
            "0".into()
        };
    }

    let digits = significant_digits(value, kind);

    // Rust's `{:e}` gives us the mantissa and the decimal exponent in one step,
    // and rounds to `digits` significant digits on the way.
    let scientific = format!("{value:.*e}", digits - 1);
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("`{:e}` always emits an exponent");
    let exponent: i32 = exponent.parse().expect("`{:e}` emits a decimal exponent");

    let precision = digits.max(kind.min_digits()) as i32;
    if exponent < -4 || exponent >= precision {
        // `{:e}` writes the exponent bare ("1e-5"); GNU pads it to at least two
        // digits and always signs it ("1e-05").
        let sign = if exponent < 0 { '-' } else { '+' };
        let magnitude = exponent.abs();
        format!("{mantissa}e{sign}{magnitude:02}")
    } else {
        // `digits` counts significant digits; `%f` wants digits after the point.
        let decimals = (digits as i32 - 1 - exponent).max(0) as usize;
        let fixed = format!("{value:.decimals$}");
        // A minimal `digits` never leaves a trailing zero, but `%g` strips them
        // and matching that keeps this robust if the digit count is ever relaxed.
        strip_trailing_zeros(&fixed)
    }
}

/// Drop trailing fractional zeros, and the decimal point if nothing follows it.
fn strip_trailing_zeros(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Right-align a rendered value in the column width `od` reserves for it.
fn pad(repr: &str, width: usize) -> String {
    format!(" {repr:>width$}")
}

pub fn format_item_f16(f: f64) -> String {
    let value = f64::from(f16::from_f64(f));
    pad(
        &format_float(value, FloatKind::Single),
        FORMAT_ITEM_F16.print_width - 1,
    )
}

pub fn format_item_bf16(f: f64) -> String {
    let value = f64::from(bf16::from_f32(f as f32));
    pad(
        &format_float(value, FloatKind::Single),
        FORMAT_ITEM_BF16.print_width - 1,
    )
}

pub fn format_item_f32(f: f64) -> String {
    pad(
        &format_float(f64::from(f as f32), FloatKind::Single),
        FORMAT_ITEM_F32.print_width - 1,
    )
}

pub fn format_item_f64(f: f64) -> String {
    pad(
        &format_float(f, FloatKind::Double),
        FORMAT_ITEM_F64.print_width - 1,
    )
}

pub fn format_item_long_double(f: f64) -> String {
    format!(" {}", format_long_double(f))
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

/// Expectations in these tests were taken from GNU coreutils' `od` (9.7), by
/// feeding it the same values and recording what it printed.
#[cfg(test)]
mod tests {
    use super::*;

    /// `format_float` for a value that reaches `od` as a 32-bit float.
    fn single(value: f32) -> String {
        format_float(f64::from(value), FloatKind::Single)
    }

    fn double(value: f64) -> String {
        format_float(value, FloatKind::Double)
    }

    #[test]
    fn f32_uses_shortest_round_trip_form() {
        assert_eq!(single(1.0), "1");
        assert_eq!(single(2.5), "2.5");
        assert_eq!(single(10.0), "10");
        assert_eq!(single(100.0), "100");
        assert_eq!(single(0.5), "0.5");
        assert_eq!(single(0.25), "0.25");
        assert_eq!(single(0.0625), "0.0625");
        assert_eq!(single(0.1), "0.1");
        assert_eq!(single(std::f32::consts::PI), "3.1415927");
        assert_eq!(single(1_234_567.0), "1234567");
        assert_eq!(single(-1.0), "-1");
        assert_eq!(single(-1_234_567.0), "-1234567");
    }

    /// `%g` switches to scientific notation below 1e-5, not at the first
    /// negative exponent.
    #[test]
    fn f32_small_values_stay_in_fixed_notation() {
        assert_eq!(single(0.01), "0.01");
        assert_eq!(single(0.001), "0.001");
        assert_eq!(single(0.0001), "0.0001");
        assert_eq!(single(-0.01), "-0.01");
        assert_eq!(single(1e-5), "1e-05");
        assert_eq!(single(1e-6), "1e-06");
    }

    /// The fixed/scientific cut-off uses at least `FLT_DIG` (6) digits, so 1e5
    /// stays fixed while 1e6 does not, even though both need one digit.
    #[test]
    fn f32_large_values_switch_at_flt_dig() {
        assert_eq!(single(1e5), "100000");
        assert_eq!(single(1e6), "1e+06");
        assert_eq!(single(1e7), "1e+07");
        assert_eq!(single(123_456_792.0), "1.2345679e+08");
        assert_eq!(single(1e38), "1e+38");
        assert_eq!(single(3.402_823_5e38), "3.4028235e+38");
    }

    /// `%g` emits the *correctly rounded* decimal at each precision, which is
    /// not always the shortest decimal that round-trips. For the f32 nearest
    /// 2^-96, eight digits round-trip as 1.2621775e-29, but `%.8g` rounds to
    /// 1.2621774e-29, which does not — so GNU prints nine digits. Reproducing
    /// GNU means following the rounding, not merely the shortest form.
    #[test]
    fn f32_uses_correctly_rounded_digits_not_merely_shortest() {
        assert_eq!(single(f32::from_bits(0x0F80 << 16)), "1.26217745e-29");
        assert_eq!(single(-f32::from_bits(0x0F80 << 16)), "-1.26217745e-29");
        assert_eq!(single(f32::from_bits(0x6B00 << 16)), "1.54742505e+26");
        assert_eq!(single(f32::from_bits(0x6C80 << 16)), "1.23794004e+27");
    }

    #[test]
    fn f32_subnormals() {
        assert_eq!(single(f32::from_bits(1)), "1e-45");
        assert_eq!(single(f32::from_bits(0x007f_ffff)), "1.1754942e-38");
        assert_eq!(single(1e-38), "1e-38");
    }

    #[test]
    fn f64_uses_shortest_round_trip_form() {
        assert_eq!(double(1.0), "1");
        assert_eq!(double(2.5), "2.5");
        assert_eq!(double(10.0), "10");
        assert_eq!(double(0.1), "0.1");
        assert_eq!(double(0.01), "0.01");
        assert_eq!(double(0.0001), "0.0001");
        assert_eq!(double(1e-5), "1e-05");
        assert_eq!(double(std::f64::consts::PI), "3.141592653589793");
        assert_eq!(double(-1.0), "-1");
        assert_eq!(double(-0.1), "-0.1");
    }

    /// A `double` keeps fixed notation up to `DBL_DIG` (15) digits, further
    /// than a `float` does.
    #[test]
    fn f64_switches_at_dbl_dig() {
        assert_eq!(double(1e6), "1000000");
        assert_eq!(double(1e9), "1000000000");
        assert_eq!(double(1e14), "100000000000000");
        assert_eq!(double(1e15), "1e+15");
        assert_eq!(double(1e16), "1e+16");
        assert_eq!(double(1_234_567_890_123.0), "1234567890123");
        assert_eq!(double(1e308), "1e+308");
    }

    #[test]
    fn f64_subnormals() {
        assert_eq!(double(1e-308), "1e-308");
        assert_eq!(
            double(2.225_073_858_507_201_4e-308),
            "2.2250738585072014e-308"
        );
        assert_eq!(double(4e-320), "4e-320");
        assert_eq!(double(5e-324), "5e-324");
    }

    /// GNU spells these in lower case and keeps the sign of a NaN.
    #[test]
    fn special_values() {
        for kind in [FloatKind::Single, FloatKind::Double] {
            assert_eq!(format_float(0.0, kind), "0");
            assert_eq!(format_float(-0.0, kind), "-0");
            assert_eq!(format_float(f64::INFINITY, kind), "inf");
            assert_eq!(format_float(f64::NEG_INFINITY, kind), "-inf");
            assert_eq!(format_float(f64::NAN, kind), "nan");
            assert_eq!(format_float(-f64::NAN, kind), "-nan");
        }
    }

    /// Each `format_item_*` right-aligns its value in the column width `od`
    /// advertises for that format.
    #[test]
    fn items_are_padded_to_the_advertised_width() {
        let cases = [
            (
                format_item_f16(1.0),
                FORMAT_ITEM_F16.print_width,
                "               1",
            ),
            (
                format_item_bf16(1.0),
                FORMAT_ITEM_BF16.print_width,
                "               1",
            ),
            (
                format_item_f32(1.0),
                FORMAT_ITEM_F32.print_width,
                "               1",
            ),
            (
                format_item_f64(1.0),
                FORMAT_ITEM_F64.print_width,
                "                        1",
            ),
        ];
        for (rendered, width, expected) in cases {
            assert_eq!(rendered.chars().count(), width);
            assert_eq!(rendered, expected);
        }
    }

    /// Half precision widens losslessly to `float` and shares its formatting.
    #[test]
    fn f16_matches_float_formatting() {
        assert_eq!(format_item_f16(1.0).trim(), "1");
        // 0x8400 is the negative half just below the subnormal boundary
        assert_eq!(
            format_item_f16(f64::from(f16::from_bits(0x8400))).trim(),
            "-6.1035156e-05"
        );
        assert_eq!(
            format_item_f16(f64::from(f16::from_f32(0.25))).trim(),
            "0.25"
        );
    }
}
