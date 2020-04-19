//! formatter for %e %E scientific notation subs
use super::super::format_field::FormatField;
use super::super::formatter::{FormatPrimitive, Formatter, InPrefix};
use super::float_common::{get_primitive_dec, primitive_to_str_common, FloatAnalysis};

pub struct Scif {
    as_num: f64,
}
impl Scif {
    pub fn new() -> Scif {
        Scif { as_num: 0.0 }
    }
}
impl Formatter for Scif {
    fn get_primitive(
        &self,
        field: &FormatField,
        inprefix: &InPrefix,
        str_in: &str,
    ) -> Option<FormatPrimitive> {
        let second_field = field.second_field.unwrap_or(6) + 1;
        let analysis = FloatAnalysis::analyze(
            str_in,
            inprefix,
            Some(second_field as usize + 1),
            None,
            false,
        );
        let f = get_primitive_dec(
            inprefix,
            &str_in[inprefix.offset..],
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
