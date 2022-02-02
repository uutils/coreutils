// spell-checker:ignore (vars) charf cninetyninehexfloatf decf floatf intf scif strf Cninety

//! formatter for %e %E scientific notation subs
use super::super::format_field::FormatField;
use super::super::formatter::{FormatPrimitive, Formatter, InitialPrefix};
use super::float_common::{get_primitive_dec, primitive_to_str_common, FloatAnalysis};

#[derive(Default)]
pub struct Scif;

impl Scif {
    pub fn new() -> Self {
        Self::default()
    }
}
impl Formatter for Scif {
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
            Some(second_field as usize + 1),
            None,
            false,
        );
        let f = get_primitive_dec(
            initial_prefix,
            &str_in[initial_prefix.offset..],
            &analysis,
            second_field as usize,
            Some(*field.field_char == 'E'),
        );
        Some(f)
    }
    fn primitive_to_str(&self, prim: &FormatPrimitive, field: FormatField) -> String {
        primitive_to_str_common(prim, &field)
    }
}
