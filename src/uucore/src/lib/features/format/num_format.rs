// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore bigdecimal prec cppreference
//! Utilities for formatting numbers in various formats

use bigdecimal::BigDecimal;
use bigdecimal::num_bigint::ToBigInt;
use num_traits::Signed;
use num_traits::Zero;
use std::cmp::min;
use std::io::Write;

use super::{
    ExtendedBigDecimal, FormatError,
    spec::{CanAsterisk, Spec},
};

pub trait Formatter<T> {
    fn fmt(&self, writer: impl Write, x: T) -> std::io::Result<()>;
    fn try_from_spec(s: Spec) -> Result<Self, FormatError>
    where
        Self: Sized;
}

#[derive(Clone, Copy, Debug)]
pub enum UnsignedIntVariant {
    Decimal,
    Octal(Prefix),
    Hexadecimal(Case, Prefix),
}

#[derive(Clone, Copy, Debug)]
pub enum FloatVariant {
    Decimal,
    Scientific,
    Shortest,
    Hexadecimal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Case {
    Lowercase,
    Uppercase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prefix {
    No,
    Yes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForceDecimal {
    No,
    Yes,
}

#[derive(Clone, Copy, Debug)]
pub enum PositiveSign {
    None,
    Plus,
    Space,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberAlignment {
    Left,
    RightSpace,
    RightZero,
}

pub struct SignedInt {
    pub width: usize,
    pub precision: usize,
    pub positive_sign: PositiveSign,
    pub alignment: NumberAlignment,
}

impl Formatter<i64> for SignedInt {
    fn fmt(&self, writer: impl Write, x: i64) -> std::io::Result<()> {
        // -i64::MIN is actually 1 larger than i64::MAX, so we need to cast to i128 first.
        let abs = (x as i128).abs();
        let s = if self.precision > 0 {
            format!("{abs:0>width$}", width = self.precision)
        } else {
            abs.to_string()
        };

        let sign_indicator = get_sign_indicator(self.positive_sign, x.is_negative());

        write_output(writer, sign_indicator, s, self.width, self.alignment)
    }

    fn try_from_spec(s: Spec) -> Result<Self, FormatError> {
        let Spec::SignedInt {
            width,
            precision,
            positive_sign,
            alignment,
            position: _position,
        } = s
        else {
            return Err(FormatError::WrongSpecType);
        };

        let width = match width {
            Some(CanAsterisk::Fixed(x)) => x,
            None => 0,
            Some(CanAsterisk::Asterisk(_)) => return Err(FormatError::WrongSpecType),
        };

        let precision = match precision {
            Some(CanAsterisk::Fixed(x)) => x,
            None => 0,
            Some(CanAsterisk::Asterisk(_)) => return Err(FormatError::WrongSpecType),
        };

        Ok(Self {
            width,
            precision,
            positive_sign,
            alignment,
        })
    }
}

pub struct UnsignedInt {
    pub variant: UnsignedIntVariant,
    pub width: usize,
    pub precision: usize,
    pub alignment: NumberAlignment,
}

impl Formatter<u64> for UnsignedInt {
    fn fmt(&self, writer: impl Write, x: u64) -> std::io::Result<()> {
        let mut s = match self.variant {
            UnsignedIntVariant::Decimal => format!("{x}"),
            UnsignedIntVariant::Octal(_) => format!("{x:o}"),
            UnsignedIntVariant::Hexadecimal(case, _) => match case {
                Case::Lowercase => format!("{x:x}"),
                Case::Uppercase => format!("{x:X}"),
            },
        };

        // Zeroes do not get a prefix. An octal value does also not get a
        // prefix if the padded value does not start with a zero.
        let prefix = match (x, self.variant) {
            (1.., UnsignedIntVariant::Hexadecimal(Case::Lowercase, Prefix::Yes)) => "0x",
            (1.., UnsignedIntVariant::Hexadecimal(Case::Uppercase, Prefix::Yes)) => "0X",
            (1.., UnsignedIntVariant::Octal(Prefix::Yes)) if s.len() >= self.precision => "0",
            _ => "",
        };

        s = format!("{prefix}{s:0>width$}", width = self.precision);
        write_output(writer, String::new(), s, self.width, self.alignment)
    }

    fn try_from_spec(s: Spec) -> Result<Self, FormatError> {
        // A signed int spec might be mapped to an unsigned int spec if no sign is specified
        let s = if let Spec::SignedInt {
            width,
            precision,
            positive_sign: PositiveSign::None,
            alignment,
            position,
        } = s
        {
            Spec::UnsignedInt {
                variant: UnsignedIntVariant::Decimal,
                width,
                precision,
                alignment,
                position,
            }
        } else {
            s
        };

        let Spec::UnsignedInt {
            variant,
            width,
            precision,
            alignment,
            position: _position,
        } = s
        else {
            return Err(FormatError::WrongSpecType);
        };

        let width = match width {
            Some(CanAsterisk::Fixed(x)) => x,
            None => 0,
            Some(CanAsterisk::Asterisk(_)) => return Err(FormatError::WrongSpecType),
        };

        let precision = match precision {
            Some(CanAsterisk::Fixed(x)) => x,
            None => 0,
            Some(CanAsterisk::Asterisk(_)) => return Err(FormatError::WrongSpecType),
        };

        Ok(Self {
            variant,
            width,
            precision,
            alignment,
        })
    }
}

pub struct Float {
    pub variant: FloatVariant,
    pub case: Case,
    pub force_decimal: ForceDecimal,
    pub width: usize,
    pub positive_sign: PositiveSign,
    pub alignment: NumberAlignment,
    // For float, the default precision depends on the format, usually 6,
    // but something architecture-specific for %a. Set this to None to
    // use the default.
    pub precision: Option<usize>,
}

impl Default for Float {
    fn default() -> Self {
        Self {
            variant: FloatVariant::Decimal,
            case: Case::Lowercase,
            force_decimal: ForceDecimal::No,
            width: 0,
            positive_sign: PositiveSign::None,
            alignment: NumberAlignment::Left,
            precision: None,
        }
    }
}

impl Formatter<&ExtendedBigDecimal> for Float {
    fn fmt(&self, writer: impl Write, e: &ExtendedBigDecimal) -> std::io::Result<()> {
        /* TODO: Might be nice to implement Signed trait for ExtendedBigDecimal (for abs)
         * at some point, but that requires implementing a _lot_ of traits.
         * Note that "negative" would be the output of "is_sign_negative" on a f64:
         * it returns true on `-0.0`.
         */
        let (abs, negative) = match e {
            ExtendedBigDecimal::BigDecimal(bd) => {
                (ExtendedBigDecimal::BigDecimal(bd.abs()), bd.is_negative())
            }
            ExtendedBigDecimal::MinusZero => (ExtendedBigDecimal::zero(), true),
            ExtendedBigDecimal::Infinity => (ExtendedBigDecimal::Infinity, false),
            ExtendedBigDecimal::MinusInfinity => (ExtendedBigDecimal::Infinity, true),
            ExtendedBigDecimal::Nan => (ExtendedBigDecimal::Nan, false),
            ExtendedBigDecimal::MinusNan => (ExtendedBigDecimal::Nan, true),
        };

        let mut alignment = self.alignment;

        let s = match abs {
            ExtendedBigDecimal::BigDecimal(bd) => match self.variant {
                FloatVariant::Decimal => {
                    format_float_decimal(&bd, self.precision, self.force_decimal)
                }
                FloatVariant::Scientific => {
                    format_float_scientific(&bd, self.precision, self.case, self.force_decimal)
                }
                FloatVariant::Shortest => {
                    format_float_shortest(&bd, self.precision, self.case, self.force_decimal)
                }
                FloatVariant::Hexadecimal => {
                    format_float_hexadecimal(&bd, self.precision, self.case, self.force_decimal)
                }
            },
            _ => {
                // Pad non-finite numbers with spaces, not zeros.
                if alignment == NumberAlignment::RightZero {
                    alignment = NumberAlignment::RightSpace;
                }
                format_float_non_finite(&abs, self.case)
            }
        };
        let sign_indicator = get_sign_indicator(self.positive_sign, negative);

        write_output(writer, sign_indicator, s, self.width, alignment)
    }

    fn try_from_spec(s: Spec) -> Result<Self, FormatError>
    where
        Self: Sized,
    {
        let Spec::Float {
            variant,
            case,
            force_decimal,
            width,
            positive_sign,
            alignment,
            precision,
            position: _position,
        } = s
        else {
            return Err(FormatError::WrongSpecType);
        };

        let width = match width {
            Some(CanAsterisk::Fixed(x)) => x,
            None => 0,
            Some(CanAsterisk::Asterisk(_)) => return Err(FormatError::WrongSpecType),
        };

        let precision = match precision {
            Some(CanAsterisk::Fixed(x)) => Some(x),
            None => None,
            Some(CanAsterisk::Asterisk(_)) => return Err(FormatError::WrongSpecType),
        };

        Ok(Self {
            variant,
            case,
            force_decimal,
            width,
            positive_sign,
            alignment,
            precision,
        })
    }
}

fn get_sign_indicator(sign: PositiveSign, negative: bool) -> String {
    if negative {
        String::from("-")
    } else {
        match sign {
            PositiveSign::None => String::new(),
            PositiveSign::Plus => String::from("+"),
            PositiveSign::Space => String::from(" "),
        }
    }
}

fn format_float_non_finite(e: &ExtendedBigDecimal, case: Case) -> String {
    let mut s = match e {
        ExtendedBigDecimal::Infinity => String::from("inf"),
        ExtendedBigDecimal::Nan => String::from("nan"),
        _ => {
            debug_assert!(false);
            String::from("INVALID")
        }
    };

    if case == Case::Uppercase {
        s.make_ascii_uppercase();
    }
    s
}

fn format_float_decimal(
    bd: &BigDecimal,
    precision: Option<usize>,
    force_decimal: ForceDecimal,
) -> String {
    debug_assert!(!bd.is_negative());
    let precision = precision.unwrap_or(6); // Default %f precision (C standard)
    if precision == 0 {
        let (bi, scale) = bd.as_bigint_and_scale();
        if scale == 0 && force_decimal != ForceDecimal::Yes {
            // Optimization when printing integers.
            return bi.to_str_radix(10);
        } else if force_decimal == ForceDecimal::Yes {
            return format!("{bd:.0}.");
        }
    }
    format!("{bd:.precision$}")
}

/// Converts a `&BigDecimal` to a scientific-like `X.XX * 10^e`.
/// - The returned `String` contains the digits `XXX`, _without_ the separating
///   `.` (the caller must add that to get a valid scientific format number).
/// - `e` is an integer exponent.
fn bd_to_string_exp_with_prec(bd: &BigDecimal, precision: usize) -> (String, i64) {
    // TODO: A lot of time is spent in `with_prec` computing the exact number
    // of digits, it might be possible to save computation time by doing a rough
    // division followed by arithmetics on `digits` to round if necessary (using
    // `fast_inc`).

    // Round bd to precision digits (including the leading digit)
    // Note that `with_prec` will produce an extra digit if rounding overflows
    // (e.g. 9995.with_prec(3) => 1000 * 10^1, but we want 100 * 10^2), we compensate
    // for that later.
    let bd_round = bd.with_prec(precision as u64);

    // Convert to the form XXX * 10^-p (XXX is precision digit long)
    let (frac, mut p) = bd_round.as_bigint_and_exponent();

    let mut digits = frac.to_str_radix(10);

    // In the unlikely case we had an overflow, correct for that.
    if digits.len() == precision + 1 {
        debug_assert!(&digits[precision..] == "0");
        digits.truncate(precision);
        p -= 1;
    }

    // If we end up with scientific formatting, we would convert XXX to X.XX:
    // that divides by 10^(precision-1), so add that to the exponent.
    let exponent = -p + precision as i64 - 1;

    (digits, exponent)
}

fn format_float_scientific(
    bd: &BigDecimal,
    precision: Option<usize>,
    case: Case,
    force_decimal: ForceDecimal,
) -> String {
    debug_assert!(!bd.is_negative());
    let precision = precision.unwrap_or(6); // Default %e precision (C standard)
    let exp_char = match case {
        Case::Lowercase => 'e',
        Case::Uppercase => 'E',
    };

    if BigDecimal::zero().eq(bd) {
        return if force_decimal == ForceDecimal::Yes && precision == 0 {
            format!("0.{exp_char}+00")
        } else {
            format!("{:.*}{exp_char}+00", precision, 0.0)
        };
    }

    let (digits, exponent) = bd_to_string_exp_with_prec(bd, precision + 1);

    // TODO: Optimizations in format_float_shortest can be made here as well
    let (first_digit, remaining_digits) = digits.split_at(1);

    let dot =
        if !remaining_digits.is_empty() || (precision == 0 && ForceDecimal::Yes == force_decimal) {
            "."
        } else {
            ""
        };

    format!("{first_digit}{dot}{remaining_digits}{exp_char}{exponent:+03}")
}

fn format_float_shortest(
    bd: &BigDecimal,
    precision: Option<usize>,
    case: Case,
    force_decimal: ForceDecimal,
) -> String {
    debug_assert!(!bd.is_negative());
    let precision = precision.unwrap_or(6); // Default %g precision (C standard)

    // Note: Precision here is how many digits should be displayed in total,
    // instead of how many digits in the fractional part.

    // Precision 0 is equivalent to precision 1.
    let precision = precision.max(1);

    if BigDecimal::zero().eq(bd) {
        return match (force_decimal, precision) {
            (ForceDecimal::Yes, 1) => "0.".into(),
            (ForceDecimal::Yes, _) => {
                format!("{:.*}", precision - 1, 0.0)
            }
            (ForceDecimal::No, _) => "0".into(),
        };
    }

    let mut output = String::with_capacity(precision);
    let (digits, exponent) = bd_to_string_exp_with_prec(bd, precision);

    if exponent < -4 || exponent >= precision as i64 {
        // Scientific-ish notation (with a few differences)

        // Scale down "XXX" to "X.XX"
        let (first_digit, remaining_digits) = digits.split_at(1);

        // Always add the dot, we might trim it later.
        output.push_str(first_digit);
        output.push('.');
        output.push_str(remaining_digits);

        if force_decimal == ForceDecimal::No {
            strip_fractional_zeroes_and_dot(&mut output);
        }

        output.push(match case {
            Case::Lowercase => 'e',
            Case::Uppercase => 'E',
        });

        // Format the exponent
        let exponent_abs = exponent.abs();
        output.push(if exponent < 0 { '-' } else { '+' });
        if exponent_abs < 10 {
            output.push('0');
        }
        output.push_str(&exponent_abs.to_string());
    } else {
        // Decimal-ish notation with a few differences:
        //  - The precision works differently and specifies the total number
        //    of digits instead of the digits in the fractional part.
        //  - If we don't force the decimal, `.` and trailing `0` in the fractional part
        //    are trimmed.
        if exponent < 0 {
            // Small number, prepend some "0.00" string
            output.push_str("0.");
            output.extend(std::iter::repeat_n('0', -exponent as usize - 1));
            output.push_str(&digits);
        } else {
            // exponent >= 0, slot in a dot at the right spot
            let (first_digits, remaining_digits) = digits.split_at(exponent as usize + 1);

            // Always add `.` even if it's trailing, we might trim it later
            output.push_str(first_digits);
            output.push('.');
            output.push_str(remaining_digits);
        }

        if force_decimal == ForceDecimal::No {
            strip_fractional_zeroes_and_dot(&mut output);
        }
    }

    output
}

fn format_float_hexadecimal(
    bd: &BigDecimal,
    precision: Option<usize>,
    case: Case,
    force_decimal: ForceDecimal,
) -> String {
    debug_assert!(!bd.is_negative());
    // Default precision for %a is supposed to be sufficient to represent the
    // exact value. This is platform specific, GNU coreutils uses a `long double`,
    // which can be equivalent to a f64, f128, or an x86(-64) specific "f80".
    // We have arbitrary precision in base 10, so we can't always represent
    // the value exactly (e.g. 0.1 is c.ccccc...).
    //
    // Note that this is the maximum precision, trailing 0's are trimmed when
    // printing.
    //
    // Emulate x86(-64) behavior, where 64 bits at _most_ are printed in total,
    // that's 16 hex digits, including 1 before the decimal point (so 15 after).
    //
    // TODO: Make this configurable? e.g. arm64 value would be 28 (f128),
    // arm value 13 (f64).
    let max_precision = precision.unwrap_or(15);

    let (prefix, exp_char) = match case {
        Case::Lowercase => ("0x", 'p'),
        Case::Uppercase => ("0X", 'P'),
    };

    if BigDecimal::zero().eq(bd) {
        // To print 0, we don't ever need any digits after the decimal point, so default to
        // that if precision is not specified.
        return if force_decimal == ForceDecimal::Yes && precision.unwrap_or(0) == 0 {
            format!("0x0.{exp_char}+0")
        } else {
            format!("0x{:.*}{exp_char}+0", precision.unwrap_or(0), 0.0)
        };
    }

    // Convert to the form frac10 * 10^exp
    let (frac10, p) = bd.as_bigint_and_exponent();
    // We cast this to u32 below, but we probably do not care about exponents
    // that would overflow u32. We should probably detect this and fail
    // gracefully though.
    let exp10 = -p;

    // We want something that looks like this: frac2 * 2^exp2,
    // without losing precision.
    // frac10 * 10^exp10 = (frac10 * 5^exp10) * 2^exp10 = frac2 * 2^exp2

    // TODO: this is most accurate, but frac2 will grow a lot for large
    // precision or exponent, and formatting will get very slow.
    // The precision can't technically be a very large number (up to 32-bit int),
    // but we can trim some of the lower digits, if we want to only keep what a
    // `long double` (80-bit or 128-bit at most) implementation would be able to
    // display.
    // The exponent is less of a problem if we matched `long double` implementation,
    // as a 80/128-bit floats only covers a 15-bit exponent.

    let (mut frac2, mut exp2) = if exp10 >= 0 {
        // Positive exponent. 5^exp10 is an integer, so we can just multiply.
        (frac10 * 5.to_bigint().unwrap().pow(exp10 as u32), exp10)
    } else {
        // Negative exponent: We're going to need to divide by 5^-exp10,
        // so we first shift left by some margin to make sure we do not lose digits.

        // We want to make sure we have at least precision+1 hex digits to start with.
        // Then, dividing by 5^-exp10 loses at most -exp10*3 binary digits
        // (since 5^-exp10 < 8^-exp10), so we add that, and another bit for
        // rounding.
        let margin =
            ((max_precision + 1) as i64 * 4 - frac10.bits() as i64).max(0) + -exp10 * 3 + 1;

        // frac10 * 10^exp10 = frac10 * 2^margin * 10^exp10 * 2^-margin =
        // (frac10 * 2^margin * 5^exp10) * 2^exp10 * 2^-margin =
        // (frac10 * 2^margin / 5^-exp10) * 2^(exp10-margin)
        (
            (frac10 << margin) / 5.to_bigint().unwrap().pow(-exp10 as u32),
            exp10 - margin,
        )
    };

    // Emulate x86(-64) behavior, we display 4 binary digits before the decimal point,
    // so the value will always be between 0x8 and 0xf.
    // TODO: Make this configurable? e.g. arm64 only displays 1 digit.
    const BEFORE_BITS: usize = 4;
    let wanted_bits = (BEFORE_BITS + max_precision * 4) as u64;
    let bits = frac2.bits();

    exp2 += bits as i64 - wanted_bits as i64;
    if bits > wanted_bits {
        // Shift almost all the way, round up if needed, then finish shifting.
        frac2 >>= bits - wanted_bits - 1;
        let add = frac2.bit(0);
        frac2 >>= 1;

        if add {
            frac2 += 0x1;
            if frac2.bits() > wanted_bits {
                // We overflowed, drop one more hex digit.
                // Note: Yes, the leading hex digit will now contain only 1 binary digit,
                // but that emulates coreutils behavior on x86(-64).
                frac2 >>= 4;
                exp2 += 4;
            }
        }
    } else {
        frac2 <<= wanted_bits - bits;
    }

    // Convert "XXX" to "X.XX": that divides by 16^precision = 2^(4*precision), so add that to the exponent.
    let mut digits = frac2.to_str_radix(16);
    if case == Case::Uppercase {
        digits.make_ascii_uppercase();
    }
    let (first_digit, remaining_digits) = digits.split_at(1);
    let exponent = exp2 + (4 * max_precision) as i64;

    let mut remaining_digits = remaining_digits.to_string();
    if precision.is_none() {
        // Trim trailing zeros
        strip_fractional_zeroes(&mut remaining_digits);
    }

    let dot = if !remaining_digits.is_empty()
        || (precision.unwrap_or(0) == 0 && ForceDecimal::Yes == force_decimal)
    {
        "."
    } else {
        ""
    };

    format!("{prefix}{first_digit}{dot}{remaining_digits}{exp_char}{exponent:+}")
}

fn strip_fractional_zeroes(s: &mut String) {
    let mut trim_to = s.len();
    for (pos, c) in s.char_indices().rev() {
        if pos + c.len_utf8() == trim_to {
            if c == '0' {
                trim_to = pos;
            } else {
                break;
            }
        }
    }
    s.truncate(trim_to);
}

fn strip_fractional_zeroes_and_dot(s: &mut String) {
    let mut trim_to = s.len();
    for (pos, c) in s.char_indices().rev() {
        if pos + c.len_utf8() == trim_to && (c == '0' || c == '.') {
            trim_to = pos;
        }
        if c == '.' {
            s.truncate(trim_to);
            break;
        }
    }
}

fn write_output(
    mut writer: impl Write,
    sign_indicator: String,
    s: String,
    width: usize,
    alignment: NumberAlignment,
) -> std::io::Result<()> {
    if width == 0 {
        writer.write_all(sign_indicator.as_bytes())?;
        writer.write_all(s.as_bytes())?;
        return Ok(());
    }
    // Take length of `sign_indicator`, which could be 0 or 1, into consideration when padding
    // by storing remaining_width indicating the actual width needed.
    // Using min() because self.width could be 0, 0usize - 1usize should be avoided
    let remaining_width = width - min(width, sign_indicator.len());

    // Check if the width is too large for formatting
    super::check_width(remaining_width)?;

    match alignment {
        NumberAlignment::Left => write!(writer, "{sign_indicator}{s:<remaining_width$}"),
        NumberAlignment::RightSpace => {
            let is_sign = sign_indicator.starts_with('-') || sign_indicator.starts_with('+'); // When sign_indicator is in ['-', '+']
            if is_sign && remaining_width > 0 {
                // Make sure sign_indicator is just next to number, e.g. "% +5.1f" 1 ==> $ +1.0
                let s = sign_indicator + s.as_str();
                write!(writer, "{s:>width$}", width = remaining_width + 1) // Since we now add sign_indicator and s together, plus 1
            } else {
                write!(writer, "{sign_indicator}{s:>remaining_width$}")
            }
        }
        NumberAlignment::RightZero => {
            // Add the padding after "0x" for hexadecimals
            let (prefix, rest) = if s.len() >= 2 && s[..2].eq_ignore_ascii_case("0x") {
                (&s[..2], &s[2..])
            } else {
                ("", s.as_str())
            };
            let remaining_width = remaining_width.saturating_sub(prefix.len());
            write!(writer, "{sign_indicator}{prefix}{rest:0>remaining_width$}")
        }
    }
}

#[cfg(test)]
mod test {
    use bigdecimal::BigDecimal;
    use num_traits::FromPrimitive;
    use std::str::FromStr;

    use crate::format::{
        ExtendedBigDecimal, Format,
        num_format::{Case, Float, ForceDecimal, UnsignedInt},
    };

    use super::{Formatter, SignedInt};

    #[test]
    fn unsigned_octal() {
        use super::{Formatter, NumberAlignment, Prefix, UnsignedInt, UnsignedIntVariant};
        let f = |x| {
            let mut s = Vec::new();
            UnsignedInt {
                variant: UnsignedIntVariant::Octal(Prefix::Yes),
                width: 0,
                precision: 0,
                alignment: NumberAlignment::Left,
            }
            .fmt(&mut s, x)
            .unwrap();
            String::from_utf8(s).unwrap()
        };

        assert_eq!(f(0), "0");
        assert_eq!(f(5), "05");
        assert_eq!(f(8), "010");
    }

    #[test]
    fn non_finite_float() {
        use super::format_float_non_finite;
        let f = |x| format_float_non_finite(x, Case::Lowercase);
        assert_eq!(f(&ExtendedBigDecimal::Nan), "nan");
        assert_eq!(f(&ExtendedBigDecimal::Infinity), "inf");

        let f = |x| format_float_non_finite(x, Case::Uppercase);
        assert_eq!(f(&ExtendedBigDecimal::Nan), "NAN");
        assert_eq!(f(&ExtendedBigDecimal::Infinity), "INF");
    }

    #[test]
    fn decimal_float() {
        use super::format_float_decimal;
        let f =
            |x| format_float_decimal(&BigDecimal::from_f64(x).unwrap(), Some(6), ForceDecimal::No);
        assert_eq!(f(0.0), "0.000000");
        assert_eq!(f(1.0), "1.000000");
        assert_eq!(f(100.0), "100.000000");
        assert_eq!(f(123_456.789), "123456.789000");
        assert_eq!(f(12.345_678_9), "12.345679");
        assert_eq!(f(1_000_000.0), "1000000.000000");
        assert_eq!(f(99_999_999.0), "99999999.000000");
        assert_eq!(f(1.999_999_5), "1.999999");
        assert_eq!(f(1.999_999_6), "2.000000");

        let f = |x| {
            format_float_decimal(
                &BigDecimal::from_f64(x).unwrap(),
                Some(0),
                ForceDecimal::Yes,
            )
        };
        assert_eq!(f(100.0), "100.");

        // Test arbitrary precision: long inputs that would not fit in a f64, print 24 digits after decimal point.
        let f = |x| {
            format_float_decimal(
                &BigDecimal::from_str(x).unwrap(),
                Some(24),
                ForceDecimal::No,
            )
        };
        assert_eq!(f("0.12345678901234567890"), "0.123456789012345678900000");
        assert_eq!(
            f("1234567890.12345678901234567890"),
            "1234567890.123456789012345678900000"
        );
    }

    #[test]
    fn decimal_float_zero() {
        use super::format_float_decimal;
        let f = |digits, scale| {
            format_float_decimal(
                &BigDecimal::from_bigint(digits, scale),
                Some(6),
                ForceDecimal::No,
            )
        };
        assert_eq!(f(0.into(), 0), "0.000000");
        assert_eq!(f(0.into(), -10), "0.000000");
        assert_eq!(f(0.into(), 10), "0.000000");
    }

    #[test]
    fn scientific_float() {
        use super::format_float_scientific;
        let f = |x| {
            format_float_scientific(
                &BigDecimal::from_f64(x).unwrap(),
                None,
                Case::Lowercase,
                ForceDecimal::No,
            )
        };
        assert_eq!(f(0.0), "0.000000e+00");
        assert_eq!(f(1.0), "1.000000e+00");
        assert_eq!(f(100.0), "1.000000e+02");
        assert_eq!(f(123_456.789), "1.234568e+05");
        assert_eq!(f(12.345_678_9), "1.234568e+01");
        assert_eq!(f(1_000_000.0), "1.000000e+06");
        assert_eq!(f(99_999_999.0), "1.000000e+08");

        let f = |x| {
            format_float_scientific(
                &BigDecimal::from_f64(x).unwrap(),
                Some(6),
                Case::Uppercase,
                ForceDecimal::No,
            )
        };
        assert_eq!(f(0.0), "0.000000E+00");
        assert_eq!(f(123_456.789), "1.234568E+05");

        // Test "0e10"/"0e-10". From cppreference.com: "If the value is ​0​, the exponent is also ​0​."
        let f = |digits, scale| {
            format_float_scientific(
                &BigDecimal::from_bigint(digits, scale),
                Some(6),
                Case::Lowercase,
                ForceDecimal::No,
            )
        };
        assert_eq!(f(0.into(), 0), "0.000000e+00");
        assert_eq!(f(0.into(), -10), "0.000000e+00");
        assert_eq!(f(0.into(), 10), "0.000000e+00");
    }

    #[test]
    fn scientific_float_zero_precision() {
        use super::format_float_scientific;

        let f = |x| {
            format_float_scientific(
                &BigDecimal::from_f64(x).unwrap(),
                Some(0),
                Case::Lowercase,
                ForceDecimal::No,
            )
        };
        assert_eq!(f(0.0), "0e+00");
        assert_eq!(f(1.0), "1e+00");
        assert_eq!(f(100.0), "1e+02");
        assert_eq!(f(123_456.789), "1e+05");
        assert_eq!(f(12.345_678_9), "1e+01");
        assert_eq!(f(1_000_000.0), "1e+06");
        assert_eq!(f(99_999_999.0), "1e+08");

        let f = |x| {
            format_float_scientific(
                &BigDecimal::from_f64(x).unwrap(),
                Some(0),
                Case::Lowercase,
                ForceDecimal::Yes,
            )
        };
        assert_eq!(f(0.0), "0.e+00");
        assert_eq!(f(1.0), "1.e+00");
        assert_eq!(f(100.0), "1.e+02");
        assert_eq!(f(123_456.789), "1.e+05");
        assert_eq!(f(12.345_678_9), "1.e+01");
        assert_eq!(f(1_000_000.0), "1.e+06");
        assert_eq!(f(99_999_999.0), "1.e+08");
    }

    #[test]
    fn shortest_float() {
        use super::format_float_shortest;
        let f = |x| {
            format_float_shortest(
                &BigDecimal::from_f64(x).unwrap(),
                None,
                Case::Lowercase,
                ForceDecimal::No,
            )
        };
        assert_eq!(f(0.0), "0");
        assert_eq!(f(0.00001), "1e-05");
        assert_eq!(f(0.0001), "0.0001");
        assert_eq!(f(1.0), "1");
        assert_eq!(f(100.0), "100");
        assert_eq!(f(123_456.789), "123457");
        assert_eq!(f(12.345_678_9), "12.3457");
        assert_eq!(f(1_000_000.0), "1e+06");
        assert_eq!(f(99_999_999.0), "1e+08");
    }

    #[test]
    fn shortest_float_force_decimal() {
        use super::format_float_shortest;
        let f = |x| {
            format_float_shortest(
                &BigDecimal::from_f64(x).unwrap(),
                None,
                Case::Lowercase,
                ForceDecimal::Yes,
            )
        };
        assert_eq!(f(0.0), "0.00000");
        assert_eq!(f(0.00001), "1.00000e-05");
        assert_eq!(f(0.0001), "0.000100000");
        assert_eq!(f(1.0), "1.00000");
        assert_eq!(f(100.0), "100.000");
        assert_eq!(f(123_456.789), "123457.");
        assert_eq!(f(12.345_678_9), "12.3457");
        assert_eq!(f(1_000_000.0), "1.00000e+06");
        assert_eq!(f(99_999_999.0), "1.00000e+08");
    }

    #[test]
    fn shortest_float_force_decimal_zero_precision() {
        use super::format_float_shortest;
        let f = |x| {
            format_float_shortest(
                &BigDecimal::from_f64(x).unwrap(),
                Some(0),
                Case::Lowercase,
                ForceDecimal::No,
            )
        };
        assert_eq!(f(0.0), "0");
        assert_eq!(f(0.00001), "1e-05");
        assert_eq!(f(0.0001), "0.0001");
        assert_eq!(f(1.0), "1");
        assert_eq!(f(10.0), "1e+01");
        assert_eq!(f(100.0), "1e+02");
        assert_eq!(f(123_456.789), "1e+05");
        assert_eq!(f(12.345_678_9), "1e+01");
        assert_eq!(f(1_000_000.0), "1e+06");
        assert_eq!(f(99_999_999.0), "1e+08");

        let f = |x| {
            format_float_shortest(
                &BigDecimal::from_f64(x).unwrap(),
                Some(0),
                Case::Lowercase,
                ForceDecimal::Yes,
            )
        };
        assert_eq!(f(0.0), "0.");
        assert_eq!(f(0.00001), "1.e-05");
        assert_eq!(f(0.0001), "0.0001");
        assert_eq!(f(1.0), "1.");
        assert_eq!(f(10.0), "1.e+01");
        assert_eq!(f(100.0), "1.e+02");
        assert_eq!(f(123_456.789), "1.e+05");
        assert_eq!(f(12.345_678_9), "1.e+01");
        assert_eq!(f(1_000_000.0), "1.e+06");
        assert_eq!(f(99_999_999.0), "1.e+08");
    }

    #[test]
    fn hexadecimal_float() {
        // It's important to create the BigDecimal from a string: going through a f64
        // will lose some precision.

        use super::format_float_hexadecimal;
        let f = |x| {
            format_float_hexadecimal(
                &BigDecimal::from_str(x).unwrap(),
                Some(6),
                Case::Lowercase,
                ForceDecimal::No,
            )
        };
        assert_eq!(f("0"), "0x0.000000p+0");
        assert_eq!(f("0.00001"), "0xa.7c5ac4p-20");
        assert_eq!(f("0.125"), "0x8.000000p-6");
        assert_eq!(f("256.0"), "0x8.000000p+5");
        assert_eq!(f("65536.0"), "0x8.000000p+13");
        assert_eq!(f("1.9999999999"), "0x1.000000p+1"); // Corner case: leading hex digit only contains 1 binary digit

        let f = |x| {
            format_float_hexadecimal(
                &BigDecimal::from_str(x).unwrap(),
                Some(0),
                Case::Lowercase,
                ForceDecimal::No,
            )
        };
        assert_eq!(f("0"), "0x0p+0");
        assert_eq!(f("0.125"), "0x8p-6");
        assert_eq!(f("256.0"), "0x8p+5");

        let f = |x| {
            format_float_hexadecimal(
                &BigDecimal::from_str(x).unwrap(),
                Some(0),
                Case::Lowercase,
                ForceDecimal::Yes,
            )
        };
        assert_eq!(f("0"), "0x0.p+0");
        assert_eq!(f("0.125"), "0x8.p-6");
        assert_eq!(f("256.0"), "0x8.p+5");

        // Default precision, maximum 13 digits (x86-64 behavior)
        let f = |x| {
            format_float_hexadecimal(
                &BigDecimal::from_str(x).unwrap(),
                None,
                Case::Lowercase,
                ForceDecimal::No,
            )
        };
        assert_eq!(f("0"), "0x0p+0");
        assert_eq!(f("0.00001"), "0xa.7c5ac471b478423p-20");
        assert_eq!(f("0.125"), "0x8p-6");
        assert_eq!(f("4.25"), "0x8.8p-1");
        assert_eq!(f("17.203125"), "0x8.9ap+1");
        assert_eq!(f("256.0"), "0x8p+5");
        assert_eq!(f("1000.01"), "0xf.a00a3d70a3d70a4p+6");
        assert_eq!(f("65536.0"), "0x8p+13");

        let f = |x| {
            format_float_hexadecimal(
                &BigDecimal::from_str(x).unwrap(),
                None,
                Case::Lowercase,
                ForceDecimal::Yes,
            )
        };
        assert_eq!(f("0"), "0x0.p+0");
        assert_eq!(f("0.125"), "0x8.p-6");
        assert_eq!(f("4.25"), "0x8.8p-1");
        assert_eq!(f("256.0"), "0x8.p+5");

        let f = |x| {
            format_float_hexadecimal(
                &BigDecimal::from_str(x).unwrap(),
                Some(6),
                Case::Uppercase,
                ForceDecimal::No,
            )
        };
        assert_eq!(f("0.00001"), "0XA.7C5AC4P-20");
        assert_eq!(f("0.125"), "0X8.000000P-6");

        // Test "0e10"/"0e-10". From cppreference.com: "If the value is ​0​, the exponent is also ​0​."
        let f = |digits, scale| {
            format_float_hexadecimal(
                &BigDecimal::from_bigint(digits, scale),
                Some(6),
                Case::Lowercase,
                ForceDecimal::No,
            )
        };
        assert_eq!(f(0.into(), 0), "0x0.000000p+0");
        assert_eq!(f(0.into(), -10), "0x0.000000p+0");
        assert_eq!(f(0.into(), 10), "0x0.000000p+0");
    }

    #[test]
    fn strip_insignificant_end() {
        use super::strip_fractional_zeroes_and_dot;
        let f = |s| {
            let mut s = String::from(s);
            strip_fractional_zeroes_and_dot(&mut s);
            s
        };
        assert_eq!(&f("1000"), "1000");
        assert_eq!(&f("1000."), "1000");
        assert_eq!(&f("1000.02030"), "1000.0203");
        assert_eq!(&f("1000.00000"), "1000");
    }

    #[test]
    fn shortest_float_abs_value_less_than_one() {
        use super::format_float_shortest;
        let f = |x| {
            format_float_shortest(
                &BigDecimal::from_f64(x).unwrap(),
                None,
                Case::Lowercase,
                ForceDecimal::No,
            )
        };
        assert_eq!(f(0.1171875), "0.117188");
        assert_eq!(f(0.01171875), "0.0117188");
        assert_eq!(f(0.001171875), "0.00117187");
        assert_eq!(f(0.0001171875), "0.000117187");
        assert_eq!(f(0.001171875001), "0.00117188");
    }

    #[test]
    fn shortest_float_switch_decimal_scientific() {
        use super::format_float_shortest;
        let f = |x| {
            format_float_shortest(
                &BigDecimal::from_f64(x).unwrap(),
                None,
                Case::Lowercase,
                ForceDecimal::No,
            )
        };
        assert_eq!(f(0.001), "0.001");
        assert_eq!(f(0.0001), "0.0001");
        assert_eq!(f(0.00001), "1e-05");
        assert_eq!(f(0.000001), "1e-06");
    }

    /// Wrapper function to get a string out of Format.fmt()
    fn fmt<U, T>(format: &Format<U, T>, n: T) -> String
    where
        U: Formatter<T>,
    {
        let mut v = Vec::<u8>::new();
        format.fmt(&mut v, n).unwrap();
        String::from_utf8_lossy(&v).to_string()
    }

    // Some end-to-end tests, `printf` will also test some of those but it's easier to add more
    // tests here. We mostly focus on padding, negative numbers, and format specifiers that are not
    // covered above.
    #[test]
    fn format_signed_int() {
        let format = Format::<SignedInt, i64>::parse("%d").unwrap();
        assert_eq!(fmt(&format, 123i64), "123");
        assert_eq!(fmt(&format, -123i64), "-123");
        assert_eq!(fmt(&format, i64::MAX), "9223372036854775807");
        assert_eq!(fmt(&format, i64::MIN), "-9223372036854775808");

        let format = Format::<SignedInt, i64>::parse("%i").unwrap();
        assert_eq!(fmt(&format, 123i64), "123");
        assert_eq!(fmt(&format, -123i64), "-123");

        let format = Format::<SignedInt, i64>::parse("%6d").unwrap();
        assert_eq!(fmt(&format, 123i64), "   123");
        assert_eq!(fmt(&format, -123i64), "  -123");

        let format = Format::<SignedInt, i64>::parse("%06d").unwrap();
        assert_eq!(fmt(&format, 123i64), "000123");
        assert_eq!(fmt(&format, -123i64), "-00123");

        let format = Format::<SignedInt, i64>::parse("%+6d").unwrap();
        assert_eq!(fmt(&format, 123i64), "  +123");
        assert_eq!(fmt(&format, -123i64), "  -123");

        let format = Format::<SignedInt, i64>::parse("% d").unwrap();
        assert_eq!(fmt(&format, 123i64), " 123");
        assert_eq!(fmt(&format, -123i64), "-123");
    }

    #[test]
    #[ignore = "Need issue #7509 to be fixed"]
    fn format_signed_int_precision_zero() {
        let format = Format::<SignedInt, i64>::parse("%.0d").unwrap();
        assert_eq!(fmt(&format, 123i64), "123");
        // From cppreference.com: "If both the converted value and the precision are ​0​ the conversion results in no characters."
        assert_eq!(fmt(&format, 0i64), "");
    }

    #[test]
    fn format_unsigned_int() {
        let f = |fmt_str: &str, n: u64| {
            let format = Format::<UnsignedInt, u64>::parse(fmt_str).unwrap();
            fmt(&format, n)
        };

        assert_eq!(f("%u", 123u64), "123");
        assert_eq!(f("%o", 123u64), "173");
        assert_eq!(f("%#o", 123u64), "0173");
        assert_eq!(f("%6x", 123u64), "    7b");
        assert_eq!(f("%#6x", 123u64), "  0x7b");
        assert_eq!(f("%06X", 123u64), "00007B");
        assert_eq!(f("%+6u", 123u64), "   123"); // '+' is ignored for unsigned numbers.
        assert_eq!(f("% u", 123u64), "123"); // ' ' is ignored for unsigned numbers.
        assert_eq!(f("%#x", 0), "0"); // No prefix for 0
    }

    #[test]
    #[ignore = "Need issues #7509 and #7510 to be fixed"]
    fn format_unsigned_int_broken() {
        // TODO: Merge this back into format_unsigned_int.
        let f = |fmt_str: &str, n: u64| {
            let format = Format::<UnsignedInt, u64>::parse(fmt_str).unwrap();
            fmt(&format, n)
        };

        // #7509
        assert_eq!(f("%.0o", 0), "");
        assert_eq!(f("%#0o", 0), "0"); // Already correct, but probably an accident.
        assert_eq!(f("%.0x", 0), "");
        // #7510
        assert_eq!(f("%#06x", 123u64), "0x007b");
    }

    #[test]
    fn format_float_decimal() {
        let format = Format::<Float, &ExtendedBigDecimal>::parse("%f").unwrap();
        assert_eq!(fmt(&format, &123.0.into()), "123.000000");
        assert_eq!(fmt(&format, &(-123.0).into()), "-123.000000");
        assert_eq!(fmt(&format, &123.15e-8.into()), "0.000001");
        assert_eq!(fmt(&format, &(-123.15e8).into()), "-12315000000.000000");
        let zero_exp = |exp| ExtendedBigDecimal::BigDecimal(BigDecimal::from_bigint(0.into(), exp));
        // We've had issues with "0e10"/"0e-10" formatting, and our current workaround is in Format.fmt function.
        assert_eq!(fmt(&format, &zero_exp(0)), "0.000000");
        assert_eq!(fmt(&format, &zero_exp(10)), "0.000000");
        assert_eq!(fmt(&format, &zero_exp(-10)), "0.000000");

        let format = Format::<Float, &ExtendedBigDecimal>::parse("%12f").unwrap();
        assert_eq!(fmt(&format, &123.0.into()), "  123.000000");
        assert_eq!(fmt(&format, &(-123.0).into()), " -123.000000");
        assert_eq!(fmt(&format, &123.15e-8.into()), "    0.000001");
        assert_eq!(fmt(&format, &(-123.15e8).into()), "-12315000000.000000");
        assert_eq!(
            fmt(&format, &(ExtendedBigDecimal::Infinity)),
            "         inf"
        );
        assert_eq!(
            fmt(&format, &(ExtendedBigDecimal::MinusInfinity)),
            "        -inf"
        );
        assert_eq!(fmt(&format, &(ExtendedBigDecimal::Nan)), "         nan");
        assert_eq!(
            fmt(&format, &(ExtendedBigDecimal::MinusNan)),
            "        -nan"
        );

        let format = Format::<Float, &ExtendedBigDecimal>::parse("%+#.0f").unwrap();
        assert_eq!(fmt(&format, &123.0.into()), "+123.");
        assert_eq!(fmt(&format, &(-123.0).into()), "-123.");
        assert_eq!(fmt(&format, &123.15e-8.into()), "+0.");
        assert_eq!(fmt(&format, &(-123.15e8).into()), "-12315000000.");
        assert_eq!(fmt(&format, &(ExtendedBigDecimal::Infinity)), "+inf");
        assert_eq!(fmt(&format, &(ExtendedBigDecimal::Nan)), "+nan");
        assert_eq!(fmt(&format, &(ExtendedBigDecimal::MinusZero)), "-0.");

        let format = Format::<Float, &ExtendedBigDecimal>::parse("%#06.0f").unwrap();
        assert_eq!(fmt(&format, &123.0.into()), "00123.");
        assert_eq!(fmt(&format, &(-123.0).into()), "-0123.");
        assert_eq!(fmt(&format, &123.15e-8.into()), "00000.");
        assert_eq!(fmt(&format, &(-123.15e8).into()), "-12315000000.");
        assert_eq!(fmt(&format, &(ExtendedBigDecimal::Infinity)), "   inf");
        assert_eq!(fmt(&format, &(ExtendedBigDecimal::MinusInfinity)), "  -inf");
        assert_eq!(fmt(&format, &(ExtendedBigDecimal::Nan)), "   nan");
        assert_eq!(fmt(&format, &(ExtendedBigDecimal::MinusNan)), "  -nan");
    }

    #[test]
    fn format_float_others() {
        let f = |fmt_str: &str, n: &ExtendedBigDecimal| {
            let format = Format::<Float, &ExtendedBigDecimal>::parse(fmt_str).unwrap();
            fmt(&format, n)
        };

        assert_eq!(f("%e", &(-123.0).into()), "-1.230000e+02");
        assert_eq!(f("%#09.e", &(-100.0).into()), "-001.e+02");
        assert_eq!(f("%# 9.E", &100.0.into()), "   1.E+02");
        assert_eq!(f("% 12.2A", &(-100.0).into()), "  -0XC.80P+3");
        assert_eq!(f("%012.2a", &(-100.0).into()), "-0x00c.80p+3");
        assert_eq!(f("%012.2A", &(-100.0).into()), "-0X00C.80P+3");
    }
}
