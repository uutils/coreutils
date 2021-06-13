// spell-checker:ignore (vars) charf decf floatf intf scif strf Cninety

//! formatter for %g %G decimal subs
use super::super::format_field::FormatField;
use super::super::formatter::{FormatPrimitive, Formatter, InitialPrefix};
use super::float_common::{get_primitive_dec, primitive_to_str_common, FloatAnalysis};

fn get_len_fmt_primitive(fmt: &FormatPrimitive) -> usize {
    let mut len = 0;
    if let Some(ref s) = fmt.prefix {
        len += s.len();
    }
    if let Some(ref s) = fmt.pre_decimal {
        len += s.len();
    }
    if let Some(ref s) = fmt.post_decimal {
        len += s.len();
    }
    if let Some(ref s) = fmt.suffix {
        len += s.len();
    }
    len
}

pub struct Decf;

impl Decf {
    pub fn new() -> Decf {
        Decf
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
        let mut f_sci = get_primitive_dec(
            initial_prefix,
            &str_in[initial_prefix.offset..],
            &analysis,
            second_field as usize,
            Some(*field.field_char == 'G'),
        );
        // strip trailing zeroes
        if let Some(ref post_dec) = f_sci.post_decimal {
            let trimmed = post_dec.trim_end_matches('0');
            if trimmed.len() != post_dec.len() {
                f_sci.post_decimal = Some(trimmed.to_owned());
            }
        }
        let f_fl = get_primitive_dec(
            initial_prefix,
            &str_in[initial_prefix.offset..],
            &analysis,
            second_field as usize,
            None,
        );
        Some(
            if get_len_fmt_primitive(&f_fl) >= get_len_fmt_primitive(&f_sci) {
                f_sci
            } else {
                f_fl
            },
        )
    }
    fn primitive_to_str(&self, prim: &FormatPrimitive, field: FormatField) -> String {
        primitive_to_str_common(prim, &field)
    }
}
