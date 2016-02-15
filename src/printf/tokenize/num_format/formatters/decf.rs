//! formatter for %g %G decimal subs
use super::super::format_field::FormatField;
use super::super::formatter::{InPrefix, FormatPrimitive, Formatter};
use super::float_common::{FloatAnalysis, get_primitive_dec, primitive_to_str_common};

fn get_len_fprim(fprim: &FormatPrimitive) -> usize {
    let mut len = 0;
    if let Some(ref s) = fprim.prefix {
        len += s.len();
    }
    if let Some(ref s) = fprim.pre_decimal {
        len += s.len();
    }
    if let Some(ref s) = fprim.post_decimal {
        len += s.len();
    }
    if let Some(ref s) = fprim.suffix {
        len += s.len();
    }
    len
}

pub struct Decf {
    as_num: f64,
}
impl Decf {
    pub fn new() -> Decf {
        Decf { as_num: 0.0 }
    }
}
impl Formatter for Decf {
    fn get_primitive(&self,
                     field: &FormatField,
                     inprefix: &InPrefix,
                     str_in: &str)
                     -> Option<FormatPrimitive> {
        let second_field = field.second_field.unwrap_or(6) + 1;
        // default to scif interp. so as to not truncate input vals
        // (that would be displayed in scif) based on relation to decimal place
        let analysis = FloatAnalysis::analyze(str_in,
                                              inprefix,
                                              Some(second_field as usize + 1),
                                              None,
                                              false);
        let mut f_sci = get_primitive_dec(inprefix,
                                          &str_in[inprefix.offset..],
                                          &analysis,
                                          second_field as usize,
                                          Some(*field.field_char == 'G'));
        // strip trailing zeroes
        match f_sci.post_decimal.clone() {
            Some(ref post_dec) => {
                let mut i = post_dec.len();
                {
                    let mut it = post_dec.chars();
                    while let Some(c) = it.next_back() {
                        if c != '0' {
                            break;
                        }
                        i -= 1;
                    }
                }
                if i != post_dec.len() {
                    f_sci.post_decimal = Some(String::from(&post_dec[0..i]));
                }
            }
            None => {}
        }
        let f_fl = get_primitive_dec(inprefix,
                                     &str_in[inprefix.offset..],
                                     &analysis,
                                     second_field as usize,
                                     None);
        Some(if get_len_fprim(&f_fl) >= get_len_fprim(&f_sci) {
            f_sci
        } else {
            f_fl
        })
    }
    fn primitive_to_str(&self, prim: &FormatPrimitive, field: FormatField) -> String {
        primitive_to_str_common(prim, &field)
    }
}
