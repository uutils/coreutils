// spell-checker:ignore (vars) charf decf floatf intf scif strf Cninety

//! formatter for %g %G decimal subs
use super::super::format_field::FormatField;
use super::super::formatter::{FormatPrimitive, Formatter, InitialPrefix};
use super::float_common::{get_primitive_dec, primitive_to_str_common, FloatAnalysis};

const SIGNIFICANT_FIGURES: usize = 6;

// Parse a numeric string as the nearest integer with a given significance.
// This is a helper function for round().
// Examples:
//  round_to_significance("456", 1) == 500
//  round_to_significance("456", 2) == 460
//  round_to_significance("456", 9) == 456
fn round_to_significance(input: &str, significant_figures: usize) -> u32 {
    if significant_figures < input.len() {
        // If the input has too many digits, use a float intermediary
        // to round it before converting to an integer. Otherwise,
        // converting straight to integer will truncate.
        // There might be a cleaner way to do this...
        let digits = &input[..significant_figures + 1];
        let float_representation = digits.parse::<f32>().unwrap();
        (float_representation / 10.0).round() as u32
    } else {
        input.parse::<u32>().unwrap_or(0)
    }
}

// Removing trailing zeroes, expressing the result as an integer where
// possible. This is a helper function for round().
fn truncate(mut format: FormatPrimitive) -> FormatPrimitive {
    if let Some(ref post_dec) = format.post_decimal {
        let trimmed = post_dec.trim_end_matches('0');

        if trimmed.is_empty() {
            // If there are no nonzero digits after the decimal point,
            // use integer formatting by clearing post_decimal and suffix.
            format.post_decimal = Some("".into());
            if format.suffix == Some("e+00".into()) {
                format.suffix = Some("".into());
            }
        } else if trimmed.len() != post_dec.len() {
            // Otherwise, update the format to remove only the trailing
            // zeroes (e.g. "4.50" becomes "4.5", not "4"). If there were
            // no trailing zeroes, do nothing.
            format.post_decimal = Some(trimmed.to_owned());
        }
    }
    format
}

// Round a format to six significant figures and remove trailing zeroes.
fn round(mut format: FormatPrimitive) -> FormatPrimitive {
    let mut significant_digits_remaining = SIGNIFICANT_FIGURES;

    // First, take as many significant digits as possible from pre_decimal,
    if format.pre_decimal.is_some() {
        let input = format.pre_decimal.as_ref().unwrap();
        let rounded = round_to_significance(input, significant_digits_remaining);
        let mut rounded_str = rounded.to_string();
        significant_digits_remaining -= rounded_str.len();

        // If the pre_decimal has exactly enough significant digits,
        // round the input to the nearest integer. If the first
        // post_decimal digit is 5 or higher, round up by incrementing
        // the pre_decimal number. Otherwise, use the pre_decimal as-is.
        if significant_digits_remaining == 0 {
            if let Some(digits) = &format.post_decimal {
                if digits.chars().next().unwrap_or('0') >= '5' {
                    let rounded = rounded + 1;
                    rounded_str = rounded.to_string();
                }
            }
        }
        format.pre_decimal = Some(rounded_str);
    }

    // If no significant digits remain, or there's no post_decimal to
    // round, return the rounded pre_decimal value with no post_decimal.
    // Otherwise, round the post_decimal to the remaining significance.
    if significant_digits_remaining == 0 {
        format.post_decimal = Some(String::new());
    } else if let Some(input) = format.post_decimal {
        let leading_zeroes = input.len() - input.trim_start_matches('0').len();
        let digits = &input[leading_zeroes..];

        // In the post_decimal, leading zeroes are significant. "01.0010"
        // has one significant digit in pre_decimal, and 3 from post_decimal.
        let mut post_decimal_str = String::with_capacity(significant_digits_remaining);
        for _ in 0..leading_zeroes {
            post_decimal_str.push('0');
        }

        if leading_zeroes < significant_digits_remaining {
            // After significant leading zeroes, round the remaining digits
            // to any remaining significance.
            let rounded = round_to_significance(digits, significant_digits_remaining);
            post_decimal_str.push_str(&rounded.to_string());
        } else if leading_zeroes == significant_digits_remaining
            && digits.chars().next().unwrap_or('0') >= '5'
        {
            // If necessary, round up the post_decimal ("1.000009" should
            // round to 1.00001, instead of truncating after the last
            // significant leading zero).
            post_decimal_str.pop();
            post_decimal_str.push('1');
        } else {
            // If the rounded post_decimal is entirely zeroes, discard
            // it and use integer formatting instead.
            post_decimal_str = "".into();
        }

        format.post_decimal = Some(post_decimal_str);
    }
    truncate(format)
}

// Given an exponent used in scientific notation, return whether the
// number is small enough to be expressed as a decimal instead. "Small
// enough" is based only on the number's magnitude, not the length of
// any string representation.
fn should_represent_as_decimal(suffix: &Option<String>) -> bool {
    match suffix {
        Some(exponent) => {
            if exponent.chars().nth(1) == Some('-') {
                exponent < &"e-05".into()
            } else {
                exponent < &"e+06".into()
            }
        }
        None => true,
    }
}

pub struct Decf;

impl Decf {
    pub fn new() -> Self {
        Self
    }
}
impl Formatter for Decf {
    fn get_primitive(
        &self,
        field: &FormatField,
        initial_prefix: &InitialPrefix,
        str_in: &str,
    ) -> Option<FormatPrimitive> {
        let second_field = field.second_field.unwrap_or(6) + 1;
        // default to scif interpretation so as to not truncate input vals
        // (that would be displayed in scif) based on relation to decimal place
        let analysis = FloatAnalysis::analyze(
            str_in,
            initial_prefix,
            Some(second_field as usize + 1),
            None,
            false,
        );
        let mut f_dec = get_primitive_dec(
            initial_prefix,
            &str_in[initial_prefix.offset..],
            &analysis,
            second_field as usize,
            Some(*field.field_char == 'G'),
        );

        if should_represent_as_decimal(&f_dec.suffix) {
            // Use decimal formatting instead of scientific notation
            // if the input's magnitude is small.
            f_dec = get_primitive_dec(
                initial_prefix,
                &str_in[initial_prefix.offset..],
                &analysis,
                second_field as usize,
                None,
            );
        }

        Some(round(f_dec))
    }
    fn primitive_to_str(&self, prim: &FormatPrimitive, field: FormatField) -> String {
        primitive_to_str_common(prim, &field)
    }
}
