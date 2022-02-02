// spell-checker:ignore (vars) charf cninetyninehexfloatf decf floatf intf scif strf Cninety

//! handles creating printed output for numeric substitutions

// spell-checker:ignore (vars) charf decf floatf intf scif strf Cninety

use std::env;
use std::vec::Vec;

use crate::display::Quotable;
use crate::{show_error, show_warning};

use super::format_field::{FieldType, FormatField};
use super::formatter::{Base, FormatPrimitive, Formatter, InitialPrefix};
use super::formatters::cninetyninehexfloatf::CninetyNineHexFloatf;
use super::formatters::decf::Decf;
use super::formatters::floatf::Floatf;
use super::formatters::intf::Intf;
use super::formatters::scif::Scif;

pub fn warn_expected_numeric(pf_arg: &str) {
    // important: keep println here not print
    show_error!("{}: expected a numeric value", pf_arg.maybe_quote());
}

// when character constant arguments have excess characters
// issue a warning when POSIXLY_CORRECT is not set
fn warn_char_constant_ign(remaining_bytes: &[u8]) {
    match env::var("POSIXLY_CORRECT") {
        Ok(_) => {}
        Err(e) => {
            if let env::VarError::NotPresent = e {
                show_warning!(
                    "{:?}: character(s) following character \
                     constant have been ignored",
                    &*remaining_bytes
                );
            }
        }
    }
}

// this function looks at the first few
// characters of an argument and returns a value if we can learn
// a value from that (e.g. no argument? return 0, char constant? ret value)
fn get_provided(str_in_opt: Option<&String>) -> Option<u8> {
    const C_S_QUOTE: u8 = 39;
    const C_D_QUOTE: u8 = 34;
    match str_in_opt {
        Some(str_in) => {
            let mut byte_it = str_in.bytes();
            if let Some(ch) = byte_it.next() {
                match ch {
                    C_S_QUOTE | C_D_QUOTE => {
                        Some(match byte_it.next() {
                            Some(second_byte) => {
                                let mut ignored: Vec<u8> = Vec::new();
                                for cont in byte_it {
                                    ignored.push(cont);
                                }
                                if !ignored.is_empty() {
                                    warn_char_constant_ign(&ignored);
                                }
                                second_byte as u8
                            }
                            // no byte after quote
                            None => {
                                let so_far = (ch as u8 as char).to_string();
                                warn_expected_numeric(&so_far);
                                0_u8
                            }
                        })
                    }
                    // first byte is not quote
                    _ => None, // no first byte
                }
            } else {
                Some(0_u8)
            }
        }
        None => Some(0),
    }
}

// takes a string and returns
// a sign,
// a base,
// and an offset for index after all
//  initial spacing, sign, base prefix, and leading zeroes
fn get_initial_prefix(str_in: &str, field_type: &FieldType) -> InitialPrefix {
    let mut str_it = str_in.chars();
    let mut ret = InitialPrefix {
        radix_in: Base::Ten,
        sign: 1,
        offset: 0,
    };
    let mut top_char = str_it.next();
    // skip spaces and ensure top_char is the first non-space char
    // (or None if none exists)
    while let Some(' ') = top_char {
        ret.offset += 1;
        top_char = str_it.next();
    }
    // parse sign
    match top_char {
        Some('+') => {
            ret.offset += 1;
            top_char = str_it.next();
        }
        Some('-') => {
            ret.sign = -1;
            ret.offset += 1;
            top_char = str_it.next();
        }
        _ => {}
    }
    // we want to exit with offset being
    // the index of the first non-zero
    // digit before the decimal point or
    // if there is none, the zero before the
    // decimal point, or, if there is none,
    // the decimal point.

    // while we are determining the offset
    // we will ensure as a convention
    // the offset is always on the first character
    // that we are yet unsure if it is the
    // final offset. If the zero could be before
    // a decimal point we don't move past the zero.
    let mut is_hex = false;
    if Some('0') == top_char {
        if let Some(base) = str_it.next() {
            // lead zeroes can only exist in
            // octal and hex base
            let mut do_clean_lead_zeroes = false;
            match base {
                'x' | 'X' => {
                    is_hex = true;
                    ret.offset += 2;
                    ret.radix_in = Base::Hex;
                    do_clean_lead_zeroes = true;
                }
                e @ '0'..='9' => {
                    ret.offset += 1;
                    if let FieldType::Intf = *field_type {
                        ret.radix_in = Base::Octal;
                    }
                    if e == '0' {
                        do_clean_lead_zeroes = true;
                    }
                }
                _ => {}
            }
            if do_clean_lead_zeroes {
                let mut first = true;
                for ch_zero in str_it {
                    // see notes on offset above:
                    // this is why the offset for octal and decimal numbers
                    // that reach this branch is 1 even though
                    // they have already eaten the characters '00'
                    // this is also why when hex encounters its
                    // first zero it does not move its offset
                    // forward because it does not know for sure
                    // that it's current offset (of that zero)
                    // is not the final offset,
                    // whereas at that point octal knows its
                    // current offset is not the final offset.
                    match ch_zero {
                        '0' => {
                            if !(is_hex && first) {
                                ret.offset += 1;
                            }
                        }
                        // if decimal, keep last zero if one exists
                        // (it's possible for last zero to
                        // not exist at this branch if we're in hex input)
                        '.' => break,
                        // other digit, etc.
                        _ => {
                            if !(is_hex && first) {
                                ret.offset += 1;
                            }
                            break;
                        }
                    }
                    if first {
                        first = false;
                    }
                }
            }
        }
    }
    ret
}

// this is the function a Sub's print will delegate to
// if it is a numeric field, passing the field details
// and an iterator to the argument
pub fn num_format(field: &FormatField, in_str_opt: Option<&String>) -> Option<String> {
    let field_char = field.field_char;

    // num format mainly operates by further delegating to one of
    // several Formatter structs depending on the field
    // see formatter.rs for more details

    // to do switch to static dispatch
    let formatter: Box<dyn Formatter> = match *field.field_type {
        FieldType::Intf => Box::new(Intf::new()),
        FieldType::Floatf => Box::new(Floatf::new()),
        FieldType::CninetyNineHexFloatf => Box::new(CninetyNineHexFloatf::new()),
        FieldType::Scif => Box::new(Scif::new()),
        FieldType::Decf => Box::new(Decf::new()),
        _ => {
            panic!("asked to do num format with non-num field type");
        }
    };
    let prim_opt=
        // if we can get an assumed value from looking at the first
        // few characters, use that value to create the FormatPrimitive
        if let Some(provided_num) = get_provided(in_str_opt) {
            let mut tmp : FormatPrimitive = Default::default();
            match field_char {
                'u' | 'i' | 'd' => {
                    tmp.pre_decimal = Some(
                        format!("{}", provided_num));
                },
                'x' | 'X' => {
                    tmp.pre_decimal = Some(
                        format!("{:x}", provided_num));
                },
                'o' => {
                    tmp.pre_decimal = Some(
                        format!("{:o}", provided_num));
                },
                'e' | 'E' | 'g' | 'G' => {
                    let as_str = format!("{}", provided_num);
                    let initial_prefix = get_initial_prefix(
                        &as_str,
                        field.field_type
                    );
                    tmp=formatter.get_primitive(field, &initial_prefix, &as_str)
                        .expect("err during default provided num");
                },
                _ => {
                    tmp.pre_decimal = Some(
                        format!("{}", provided_num));
                    tmp.post_decimal = Some(String::from("0"));
                }
            }
            Some(tmp)
        } else {
            // otherwise we'll interpret the argument as a number
            // using the appropriate Formatter
            let in_str = in_str_opt.expect(
                "please send the devs this message:
                \n get_provided is failing to ret as Some(0) on no str ");
            // first get information about the beginning of the
            // numeric argument that would be useful for
            // any formatter (int or float)
            let initial_prefix = get_initial_prefix(
                in_str,
                field.field_type
            );
            // then get the FormatPrimitive from the Formatter
            formatter.get_primitive(field, &initial_prefix, in_str)
        };
    // if we have a formatPrimitive, print its results
    // according to the field-char appropriate Formatter
    prim_opt.map(|prim| formatter.primitive_to_str(&prim, field.clone()))
}
