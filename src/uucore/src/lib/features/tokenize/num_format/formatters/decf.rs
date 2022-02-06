// spell-checker:ignore (vars) charf decf floatf intf scif strf Cninety

//! formatter for %g %G decimal subs
use super::super::format_field::FormatField;
use super::super::formatter::{FormatPrimitive, Formatter, InitialPrefix};
use super::float_common::{get_primitive_dec, primitive_to_str_common, FloatAnalysis};

const SIGNIFICANT_FIGURES: usize = 6;

fn round_to_significance(input: &str, significant_figures: usize) -> u32 {
    if significant_figures < input.len() {
        let digits = &input[..significant_figures + 1];
        let float_representation = digits.parse::<f32>().unwrap();
        (float_representation / 10.0).round() as u32
    } else {
        input.parse::<u32>().unwrap_or(0)
    }
}

fn round(mut format: FormatPrimitive) -> FormatPrimitive {
    let mut significant_figures = SIGNIFICANT_FIGURES;

    if format.pre_decimal.is_some() {
        let input = format.pre_decimal.as_ref().unwrap();
        let rounded = round_to_significance(input, significant_figures);
        let mut rounded_str = rounded.to_string();
        significant_figures -= rounded_str.len();

        if significant_figures == 0 {
            if let Some(digits) = &format.post_decimal {
                if digits.chars().next().unwrap_or('0') >= '5' {
                    let rounded = rounded + 1;
                    rounded_str = rounded.to_string();
                }
            }
        }
        format.pre_decimal = Some(rounded_str);
    }

    if significant_figures == 0 {
        format.post_decimal = Some(String::new());
    } else if let Some(input) = format.post_decimal {
        let leading_zeroes = input.len() - input.trim_start_matches('0').len();

        let rounded_str = if leading_zeroes <= significant_figures {
            let mut post_decimal = String::with_capacity(significant_figures);
            for _ in 0..leading_zeroes {
                post_decimal.push('0');
            }

            significant_figures -= leading_zeroes;
            let rounded = round_to_significance(&input[leading_zeroes..], significant_figures);
            post_decimal.push_str(&rounded.to_string());
            post_decimal
        } else {
            input[..significant_figures].to_string()
        };
        format.post_decimal = Some(rounded_str);
    }
    format
}

fn truncate(mut format: FormatPrimitive) -> FormatPrimitive {
    if let Some(ref post_dec) = format.post_decimal {
        let trimmed = post_dec.trim_end_matches('0');

        if trimmed.is_empty() {
            format.post_decimal = Some("".into());
            if format.suffix == Some("e+00".into()) {
                format.suffix = Some("".into());
            }
        } else if trimmed.len() != post_dec.len() {
            format.post_decimal = Some(trimmed.to_owned());
        }
    }
    format
}

fn is_float_magnitude(suffix: &Option<String>) -> bool {
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

        if is_float_magnitude(&f_dec.suffix) {
            f_dec = get_primitive_dec(
                initial_prefix,
                &str_in[initial_prefix.offset..],
                &analysis,
                second_field as usize,
                None,
            );
        }

        f_dec = truncate(round(f_dec));
        Some(f_dec)
    }
    fn primitive_to_str(&self, prim: &FormatPrimitive, field: FormatField) -> String {
        primitive_to_str_common(prim, &field)
    }
}
