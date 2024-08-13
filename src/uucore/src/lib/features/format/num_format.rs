// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Utilities for formatting numbers in various formats

use std::cmp::min;
use std::io::Write;

use super::{
    spec::{CanAsterisk, Spec},
    FormatError,
};

pub trait Formatter {
    type Input;
    fn fmt(&self, writer: impl Write, x: Self::Input) -> std::io::Result<()>;
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

impl Formatter for SignedInt {
    type Input = i64;

    fn fmt(&self, writer: impl Write, x: Self::Input) -> std::io::Result<()> {
        let s = if self.precision > 0 {
            format!("{:0>width$}", x.abs(), width = self.precision)
        } else {
            x.abs().to_string()
        };

        let sign_indicator = get_sign_indicator(self.positive_sign, &x);

        write_output(writer, sign_indicator, s, self.width, self.alignment)
    }

    fn try_from_spec(s: Spec) -> Result<Self, FormatError> {
        let Spec::SignedInt {
            width,
            precision,
            positive_sign,
            alignment,
        } = s
        else {
            return Err(FormatError::WrongSpecType);
        };

        let width = match width {
            Some(CanAsterisk::Fixed(x)) => x,
            None => 0,
            Some(CanAsterisk::Asterisk) => return Err(FormatError::WrongSpecType),
        };

        let precision = match precision {
            Some(CanAsterisk::Fixed(x)) => x,
            None => 0,
            Some(CanAsterisk::Asterisk) => return Err(FormatError::WrongSpecType),
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

impl Formatter for UnsignedInt {
    type Input = u64;

    fn fmt(&self, mut writer: impl Write, x: Self::Input) -> std::io::Result<()> {
        let mut s = match self.variant {
            UnsignedIntVariant::Decimal => format!("{x}"),
            UnsignedIntVariant::Octal(_) => format!("{x:o}"),
            UnsignedIntVariant::Hexadecimal(Case::Lowercase, _) => {
                format!("{x:x}")
            }
            UnsignedIntVariant::Hexadecimal(Case::Uppercase, _) => {
                format!("{x:X}")
            }
        };

        // Zeroes do not get a prefix. An octal value does also not get a
        // prefix if the padded value will not start with a zero.
        let prefix = match (x, self.variant) {
            (1.., UnsignedIntVariant::Hexadecimal(Case::Lowercase, Prefix::Yes)) => "0x",
            (1.., UnsignedIntVariant::Hexadecimal(Case::Uppercase, Prefix::Yes)) => "0X",
            (1.., UnsignedIntVariant::Octal(Prefix::Yes)) if s.len() >= self.precision => "0",
            _ => "",
        };

        s = format!("{prefix}{s:0>width$}", width = self.precision);

        match self.alignment {
            NumberAlignment::Left => write!(writer, "{s:<width$}", width = self.width),
            NumberAlignment::RightSpace => write!(writer, "{s:>width$}", width = self.width),
            NumberAlignment::RightZero => write!(writer, "{s:0>width$}", width = self.width),
        }
    }

    fn try_from_spec(s: Spec) -> Result<Self, FormatError> {
        // A signed int spec might be mapped to an unsigned int spec if no sign is specified
        let s = if let Spec::SignedInt {
            width,
            precision,
            positive_sign: PositiveSign::None,
            alignment,
        } = s
        {
            Spec::UnsignedInt {
                variant: UnsignedIntVariant::Decimal,
                width,
                precision,
                alignment,
            }
        } else {
            s
        };

        let Spec::UnsignedInt {
            variant,
            width,
            precision,
            alignment,
        } = s
        else {
            return Err(FormatError::WrongSpecType);
        };

        let width = match width {
            Some(CanAsterisk::Fixed(x)) => x,
            None => 0,
            Some(CanAsterisk::Asterisk) => return Err(FormatError::WrongSpecType),
        };

        let precision = match precision {
            Some(CanAsterisk::Fixed(x)) => x,
            None => 0,
            Some(CanAsterisk::Asterisk) => return Err(FormatError::WrongSpecType),
        };

        Ok(Self {
            width,
            precision,
            variant,
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
    pub precision: usize,
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
            precision: 6,
        }
    }
}

impl Formatter for Float {
    type Input = f64;

    fn fmt(&self, writer: impl Write, x: Self::Input) -> std::io::Result<()> {
        let mut s = if x.is_finite() {
            match self.variant {
                FloatVariant::Decimal => {
                    format_float_decimal(x, self.precision, self.force_decimal)
                }
                FloatVariant::Scientific => {
                    format_float_scientific(x, self.precision, self.case, self.force_decimal)
                }
                FloatVariant::Shortest => {
                    format_float_shortest(x, self.precision, self.case, self.force_decimal)
                }
                FloatVariant::Hexadecimal => {
                    format_float_hexadecimal(x, self.precision, self.case, self.force_decimal)
                }
            }
        } else {
            format_float_non_finite(x, self.case)
        };

        // The format function will parse `x` together with its sign char,
        // which should be placed in `sign_indicator`. So drop it here
        s = if x < 0. { s[1..].to_string() } else { s };

        let sign_indicator = get_sign_indicator(self.positive_sign, &x);

        write_output(writer, sign_indicator, s, self.width, self.alignment)
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
        } = s
        else {
            return Err(FormatError::WrongSpecType);
        };

        let width = match width {
            Some(CanAsterisk::Fixed(x)) => x,
            None => 0,
            Some(CanAsterisk::Asterisk) => return Err(FormatError::WrongSpecType),
        };

        let precision = match precision {
            Some(CanAsterisk::Fixed(x)) => x,
            None => {
                if matches!(variant, FloatVariant::Shortest) {
                    6
                } else {
                    0
                }
            }
            Some(CanAsterisk::Asterisk) => return Err(FormatError::WrongSpecType),
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

fn get_sign_indicator<T: PartialOrd + Default>(sign: PositiveSign, x: &T) -> String {
    if *x >= T::default() {
        match sign {
            PositiveSign::None => String::new(),
            PositiveSign::Plus => String::from("+"),
            PositiveSign::Space => String::from(" "),
        }
    } else {
        String::from("-")
    }
}

fn format_float_non_finite(f: f64, case: Case) -> String {
    debug_assert!(!f.is_finite());
    let mut s = format!("{f}");
    if case == Case::Uppercase {
        s.make_ascii_uppercase();
    }
    s
}

fn format_float_decimal(f: f64, precision: usize, force_decimal: ForceDecimal) -> String {
    if precision == 0 && force_decimal == ForceDecimal::Yes {
        format!("{f:.0}.")
    } else {
        format!("{f:.*}", precision)
    }
}

fn format_float_scientific(
    f: f64,
    precision: usize,
    case: Case,
    force_decimal: ForceDecimal,
) -> String {
    if f == 0.0 {
        return if force_decimal == ForceDecimal::Yes && precision == 0 {
            "0.e+00".into()
        } else {
            format!("{:.*}e+00", precision, 0.0)
        };
    }

    let mut exponent: i32 = f.log10().floor() as i32;
    let mut normalized = f / 10.0_f64.powi(exponent);

    // If the normalized value will be rounded to a value greater than 10
    // we need to correct.
    if (normalized * 10_f64.powi(precision as i32)).round() / 10_f64.powi(precision as i32) >= 10.0
    {
        normalized /= 10.0;
        exponent += 1;
    }

    let additional_dot = if precision == 0 && ForceDecimal::Yes == force_decimal {
        "."
    } else {
        ""
    };

    let exp_char = match case {
        Case::Lowercase => 'e',
        Case::Uppercase => 'E',
    };

    format!(
        "{normalized:.*}{additional_dot}{exp_char}{exponent:+03}",
        precision
    )
}

fn format_float_shortest(
    f: f64,
    precision: usize,
    case: Case,
    force_decimal: ForceDecimal,
) -> String {
    // Precision here is about how many digits should be displayed
    // instead of how many digits for the fractional part, this means that if
    // we pass this to rust's format string, it's always gonna be one less.
    let precision = precision.saturating_sub(1);

    if f == 0.0 {
        return match (force_decimal, precision) {
            (ForceDecimal::Yes, 0) => "0.".into(),
            (ForceDecimal::Yes, _) => {
                format!("{:.*}", precision, 0.0)
            }
            (ForceDecimal::No, _) => "0".into(),
        };
    }

    let mut exponent = f.log10().floor() as i32;
    if f != 0.0 && exponent <= -4 || exponent > precision as i32 {
        // Scientific-ish notation (with a few differences)
        let mut normalized = f / 10.0_f64.powi(exponent);

        // If the normalized value will be rounded to a value greater than 10
        // we need to correct.
        if (normalized * 10_f64.powi(precision as i32)).round() / 10_f64.powi(precision as i32)
            >= 10.0
        {
            normalized /= 10.0;
            exponent += 1;
        }

        let additional_dot = if precision == 0 && ForceDecimal::Yes == force_decimal {
            "."
        } else {
            ""
        };

        let mut normalized = format!("{normalized:.*}", precision);

        if force_decimal == ForceDecimal::No {
            strip_fractional_zeroes_and_dot(&mut normalized);
        }

        let exp_char = match case {
            Case::Lowercase => 'e',
            Case::Uppercase => 'E',
        };

        format!("{normalized}{additional_dot}{exp_char}{exponent:+03}")
    } else {
        // Decimal-ish notation with a few differences:
        //  - The precision works differently and specifies the total number
        //    of digits instead of the digits in the fractional part.
        //  - If we don't force the decimal, `.` and trailing `0` in the fractional part
        //    are trimmed.
        let decimal_places = (precision as i32 - exponent) as usize;
        let mut formatted = if decimal_places == 0 && force_decimal == ForceDecimal::Yes {
            format!("{f:.0}.")
        } else {
            format!("{f:.*}", decimal_places)
        };

        if force_decimal == ForceDecimal::No {
            strip_fractional_zeroes_and_dot(&mut formatted);
        }

        formatted
    }
}

fn format_float_hexadecimal(
    f: f64,
    precision: usize,
    case: Case,
    force_decimal: ForceDecimal,
) -> String {
    let (first_digit, mantissa, exponent) = if f == 0.0 {
        (0, 0, 0)
    } else {
        let bits = f.to_bits();
        let exponent_bits = ((bits >> 52) & 0x7fff) as i64;
        let exponent = exponent_bits - 1023;
        let mantissa = bits & 0xf_ffff_ffff_ffff;
        (1, mantissa, exponent)
    };

    let mut s = match (precision, force_decimal) {
        (0, ForceDecimal::No) => format!("0x{first_digit}p{exponent:+x}"),
        (0, ForceDecimal::Yes) => format!("0x{first_digit}.p{exponent:+x}"),
        _ => format!("0x{first_digit}.{mantissa:0>13x}p{exponent:+x}"),
    };

    if case == Case::Uppercase {
        s.make_ascii_uppercase();
    }

    s
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
    mut s: String,
    width: usize,
    alignment: NumberAlignment,
) -> std::io::Result<()> {
    // Take length of `sign_indicator`, which could be 0 or 1, into consideration when padding
    // by storing remaining_width indicating the actual width needed.
    // Using min() because self.width could be 0, 0usize - 1usize should be avoided
    let remaining_width = width - min(width, sign_indicator.len());
    match alignment {
        NumberAlignment::Left => write!(
            writer,
            "{sign_indicator}{s:<width$}",
            width = remaining_width
        ),
        NumberAlignment::RightSpace => {
            let is_sign = sign_indicator.starts_with('-') || sign_indicator.starts_with('+'); // When sign_indicator is in ['-', '+']
            if is_sign && remaining_width > 0 {
                // Make sure sign_indicator is just next to number, e.g. "% +5.1f" 1 ==> $ +1.0
                s = sign_indicator + s.as_str();
                write!(writer, "{s:>width$}", width = remaining_width + 1) // Since we now add sign_indicator and s together, plus 1
            } else {
                write!(
                    writer,
                    "{sign_indicator}{s:>width$}",
                    width = remaining_width
                )
            }
        }
        NumberAlignment::RightZero => {
            write!(
                writer,
                "{sign_indicator}{s:0>width$}",
                width = remaining_width
            )
        }
    }
}

#[cfg(test)]
mod test {
    use crate::format::num_format::{Case, ForceDecimal};

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
    fn decimal_float() {
        use super::format_float_decimal;
        let f = |x| format_float_decimal(x, 6, ForceDecimal::No);
        assert_eq!(f(0.0), "0.000000");
        assert_eq!(f(1.0), "1.000000");
        assert_eq!(f(100.0), "100.000000");
        assert_eq!(f(123456.789), "123456.789000");
        assert_eq!(f(12.3456789), "12.345679");
        assert_eq!(f(1000000.0), "1000000.000000");
        assert_eq!(f(99999999.0), "99999999.000000");
        assert_eq!(f(1.9999995), "1.999999");
        assert_eq!(f(1.9999996), "2.000000");
    }

    #[test]
    fn scientific_float() {
        use super::format_float_scientific;
        let f = |x| format_float_scientific(x, 6, Case::Lowercase, ForceDecimal::No);
        assert_eq!(f(0.0), "0.000000e+00");
        assert_eq!(f(1.0), "1.000000e+00");
        assert_eq!(f(100.0), "1.000000e+02");
        assert_eq!(f(123456.789), "1.234568e+05");
        assert_eq!(f(12.3456789), "1.234568e+01");
        assert_eq!(f(1000000.0), "1.000000e+06");
        assert_eq!(f(99999999.0), "1.000000e+08");
    }

    #[test]
    fn scientific_float_zero_precision() {
        use super::format_float_scientific;

        let f = |x| format_float_scientific(x, 0, Case::Lowercase, ForceDecimal::No);
        assert_eq!(f(0.0), "0e+00");
        assert_eq!(f(1.0), "1e+00");
        assert_eq!(f(100.0), "1e+02");
        assert_eq!(f(123456.789), "1e+05");
        assert_eq!(f(12.3456789), "1e+01");
        assert_eq!(f(1000000.0), "1e+06");
        assert_eq!(f(99999999.0), "1e+08");

        let f = |x| format_float_scientific(x, 0, Case::Lowercase, ForceDecimal::Yes);
        assert_eq!(f(0.0), "0.e+00");
        assert_eq!(f(1.0), "1.e+00");
        assert_eq!(f(100.0), "1.e+02");
        assert_eq!(f(123456.789), "1.e+05");
        assert_eq!(f(12.3456789), "1.e+01");
        assert_eq!(f(1000000.0), "1.e+06");
        assert_eq!(f(99999999.0), "1.e+08");
    }

    #[test]
    fn shortest_float() {
        use super::format_float_shortest;
        let f = |x| format_float_shortest(x, 6, Case::Lowercase, ForceDecimal::No);
        assert_eq!(f(0.0), "0");
        assert_eq!(f(1.0), "1");
        assert_eq!(f(100.0), "100");
        assert_eq!(f(123456.789), "123457");
        assert_eq!(f(12.3456789), "12.3457");
        assert_eq!(f(1000000.0), "1e+06");
        assert_eq!(f(99999999.0), "1e+08");
    }

    #[test]
    fn shortest_float_force_decimal() {
        use super::format_float_shortest;
        let f = |x| format_float_shortest(x, 6, Case::Lowercase, ForceDecimal::Yes);
        assert_eq!(f(0.0), "0.00000");
        assert_eq!(f(1.0), "1.00000");
        assert_eq!(f(100.0), "100.000");
        assert_eq!(f(123456.789), "123457.");
        assert_eq!(f(12.3456789), "12.3457");
        assert_eq!(f(1000000.0), "1.00000e+06");
        assert_eq!(f(99999999.0), "1.00000e+08");
    }

    #[test]
    fn shortest_float_force_decimal_zero_precision() {
        use super::format_float_shortest;
        let f = |x| format_float_shortest(x, 0, Case::Lowercase, ForceDecimal::No);
        assert_eq!(f(0.0), "0");
        assert_eq!(f(1.0), "1");
        assert_eq!(f(100.0), "1e+02");
        assert_eq!(f(123456.789), "1e+05");
        assert_eq!(f(12.3456789), "1e+01");
        assert_eq!(f(1000000.0), "1e+06");
        assert_eq!(f(99999999.0), "1e+08");

        let f = |x| format_float_shortest(x, 0, Case::Lowercase, ForceDecimal::Yes);
        assert_eq!(f(0.0), "0.");
        assert_eq!(f(1.0), "1.");
        assert_eq!(f(100.0), "1.e+02");
        assert_eq!(f(123456.789), "1.e+05");
        assert_eq!(f(12.3456789), "1.e+01");
        assert_eq!(f(1000000.0), "1.e+06");
        assert_eq!(f(99999999.0), "1.e+08");
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
}
