use std::io::Write;

use super::FormatError;

pub trait Formatter {
    type Input;
    fn fmt(&self, writer: impl Write, x: Self::Input) -> Result<(), FormatError>;
}

#[derive(Clone, Copy)]
pub enum UnsignedIntVariant {
    Decimal,
    Octal(Prefix),
    Hexadecimal(Case, Prefix),
}

#[derive(Clone, Copy)]

pub enum FloatVariant {
    Decimal,
    Scientific,
    Shortest,
    Hexadecimal,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Case {
    Lowercase,
    Uppercase,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Prefix {
    No,
    Yes,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ForceDecimal {
    No,
    Yes,
}

#[derive(Clone, Copy)]
pub enum PositiveSign {
    None,
    Plus,
    Space,
}

#[derive(Clone, Copy)]
pub enum NumberAlignment {
    Left,
    RightSpace,
    RightZero,
}

pub struct SignedInt {
    pub width: usize,
    pub positive_sign: PositiveSign,
    pub alignment: NumberAlignment,
}

impl Formatter for SignedInt {
    type Input = i64;

    fn fmt(&self, mut writer: impl Write, x: Self::Input) -> Result<(), FormatError> {
        if x >= 0 {
            match self.positive_sign {
                PositiveSign::None => Ok(()),
                PositiveSign::Plus => write!(writer, "+"),
                PositiveSign::Space => write!(writer, " "),
            }
            .map_err(FormatError::IoError)?;
        }

        match self.alignment {
            NumberAlignment::Left => write!(writer, "{x:<width$}", width = self.width),
            NumberAlignment::RightSpace => write!(writer, "{x:>width$}", width = self.width),
            NumberAlignment::RightZero => write!(writer, "{x:0>width$}", width = self.width),
        }
        .map_err(FormatError::IoError)
    }
}

pub struct UnsignedInt {
    pub variant: UnsignedIntVariant,
    pub width: usize,
    pub alignment: NumberAlignment,
}

impl Formatter for UnsignedInt {
    type Input = u64;

    fn fmt(&self, mut writer: impl Write, x: Self::Input) -> Result<(), FormatError> {
        let s = match self.variant {
            UnsignedIntVariant::Decimal => format!("{x}"),
            UnsignedIntVariant::Octal(Prefix::No) => format!("{x:o}"),
            UnsignedIntVariant::Octal(Prefix::Yes) => format!("{x:#o}"),
            UnsignedIntVariant::Hexadecimal(Case::Lowercase, Prefix::No) => {
                format!("{x:x}")
            }
            UnsignedIntVariant::Hexadecimal(Case::Lowercase, Prefix::Yes) => {
                format!("{x:#x}")
            }
            UnsignedIntVariant::Hexadecimal(Case::Uppercase, Prefix::No) => {
                format!("{x:X}")
            }
            UnsignedIntVariant::Hexadecimal(Case::Uppercase, Prefix::Yes) => {
                format!("{x:#X}")
            }
        };

        match self.alignment {
            NumberAlignment::Left => write!(writer, "{s:<width$}", width = self.width),
            NumberAlignment::RightSpace => write!(writer, "{s:>width$}", width = self.width),
            NumberAlignment::RightZero => write!(writer, "{s:0>width$}", width = self.width),
        }
        .map_err(FormatError::IoError)
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
            precision: 2,
        }
    }
}

impl Formatter for Float {
    type Input = f64;

    fn fmt(&self, mut writer: impl Write, x: Self::Input) -> Result<(), FormatError> {
        if x.is_sign_positive() {
            match self.positive_sign {
                PositiveSign::None => Ok(()),
                PositiveSign::Plus => write!(writer, "+"),
                PositiveSign::Space => write!(writer, " "),
            }
            .map_err(FormatError::IoError)?;
        }

        let s = match self.variant {
            FloatVariant::Decimal => {
                format_float_decimal(x, self.precision, self.case, self.force_decimal)
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
        };

        match self.alignment {
            NumberAlignment::Left => write!(writer, "{s:<width$}", width = self.width),
            NumberAlignment::RightSpace => write!(writer, "{s:>width$}", width = self.width),
            NumberAlignment::RightZero => write!(writer, "{s:0>width$}", width = self.width),
        }
        .map_err(FormatError::IoError)
    }
}

fn format_float_nonfinite(f: f64, case: Case) -> String {
    debug_assert!(!f.is_finite());
    let mut s = format!("{f}");
    if case == Case::Uppercase {
        s.make_ascii_uppercase();
    }
    return s;
}

fn format_float_decimal(
    f: f64,
    precision: usize,
    case: Case,
    force_decimal: ForceDecimal,
) -> String {
    if !f.is_finite() {
        return format_float_nonfinite(f, case);
    }

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
    // If the float is NaN, -Nan, Inf or -Inf, format like any other float
    if !f.is_finite() {
        return format_float_nonfinite(f, case);
    }

    let exponent: i32 = f.log10().floor() as i32;
    let normalized = f / 10.0_f64.powi(exponent);

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

// TODO: This could be optimized. It's not terribly important though.
fn format_float_shortest(
    f: f64,
    precision: usize,
    case: Case,
    force_decimal: ForceDecimal,
) -> String {
    let a = format_float_decimal(f, precision, case, force_decimal);
    let b = format_float_scientific(f, precision, case, force_decimal);

    if a.len() > b.len() {
        b
    } else {
        a
    }
}

fn format_float_hexadecimal(
    f: f64,
    precision: usize,
    case: Case,
    force_decimal: ForceDecimal,
) -> String {
    if !f.is_finite() {
        return format_float_nonfinite(f, case);
    }

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

    return s;
}
