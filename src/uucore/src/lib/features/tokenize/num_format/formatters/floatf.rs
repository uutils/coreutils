// spell-checker:ignore (vars) charf decf floatf intf scif strf Cninety
// spell-checker:ignore (ToDO) arrnum

//! formatter for %f %F common-notation floating-point subs
use super::super::format_field::FormatField;
use super::super::formatter::{FormatPrimitive, Formatter, InitialPrefix};
use super::float_common::{get_primitive_dec, primitive_to_str_common, FloatAnalysis};

#[derive(Default)]
pub struct Floatf;
impl Floatf {
    pub fn new() -> Self {
        Self::default()
    }
}
impl Formatter for Floatf {
    fn get_primitive(
        &self,
        field: &FormatField,
        initial_prefix: &InitialPrefix,
        str_in: &str,
    ) -> Option<FormatPrimitive> {
        let second_field = field.second_field.unwrap_or(6) + 1;
        let analysis = FloatAnalysis::analyze(
            str_in,
            initial_prefix,
            None,
            Some(second_field as usize),
            false,
        );
        let f = get_primitive_dec(
            initial_prefix,
            &str_in[initial_prefix.offset..],
            &analysis,
            second_field as usize,
            None,
        );
        Some(f)
    }
    fn primitive_to_str(&self, prim: &FormatPrimitive, field: FormatField) -> String {
        primitive_to_str_common(prim, &field)
    }
}
