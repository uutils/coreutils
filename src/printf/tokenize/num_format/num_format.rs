//! handles creating printed output for numeric substitutions

use std::env;
use std::vec::Vec;
use cli;
use super::format_field::{FormatField, FieldType};
use super::formatter::{Formatter, FormatPrimitive, InPrefix, Base};
use super::formatters::intf::Intf;
use super::formatters::floatf::Floatf;
use super::formatters::cninetyninehexfloatf::CninetyNineHexFloatf;
use super::formatters::scif::Scif;
use super::formatters::decf::Decf;

pub fn warn_expected_numeric(pf_arg: &String) {
    // important: keep println here not print
    cli::err_msg(&format!("{}: expected a numeric value", pf_arg));
}

// when character constant arguments have excess characters
// issue a warning when POSIXLY_CORRECT is not set
fn warn_char_constant_ign(remaining_bytes: Vec<u8>) {
    match env::var("POSIXLY_CORRECT") {
        Ok(_) => {}
        Err(e) => {
            match e {
                env::VarError::NotPresent => {
                    cli::err_msg(&format!("warning: {:?}: character(s) following character \
                                           constant have been ignored",
                                          &*remaining_bytes));
                }
                _ => {}
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
            if let Some(qchar) = byte_it.next() {
                match qchar {
                    C_S_QUOTE | C_D_QUOTE => {
                        return Some(match byte_it.next() {
                            Some(second_byte) => {
                                let mut ignored: Vec<u8> = Vec::new();
                                while let Some(cont) = byte_it.next() {
                                    ignored.push(cont);
                                }
                                if ignored.len() > 0 {
                                    warn_char_constant_ign(ignored);
                                }
                                second_byte as u8
                            }
                            // no byte after quote
                            None => {
                                let so_far = (qchar as u8 as char).to_string();
                                warn_expected_numeric(&so_far);
                                0 as u8
                            }
                        });
                    }
                    // first byte is not quote
                    _ => {
                        return None;
                    }
                    // no first byte
                }
            } else {
                Some(0 as u8)
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
fn get_inprefix(str_in: &String, field_type: &FieldType) -> InPrefix {
    let mut str_it = str_in.chars();
    let mut ret = InPrefix {
        radix_in: Base::Ten,
        sign: 1,
        offset: 0,
    };
    let mut topchar = str_it.next().clone();
    // skip spaces and ensure topchar is the first non-space char
    // (or None if none exists)
    loop {
        match topchar {
            Some(' ') => {
                ret.offset += 1;
                topchar = str_it.next();
            }
            _ => {
                break;
            }
        }
    }
    // parse sign
    match topchar {
        Some('+') => {
            ret.offset += 1;
            topchar = str_it.next();
        }
        Some('-') => {
            ret.sign = -1;
            ret.offset += 1;
            topchar = str_it.next();
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
    if Some('0') == topchar {
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
                e @ '0'...'9' => {
                    ret.offset += 1;
                    match *field_type {
                        FieldType::Intf => {
                            ret.radix_in = Base::Octal;
                        }
                        _ => {}                        
                    }
                    if e == '0' {
                        do_clean_lead_zeroes = true;
                    }
                }
                _ => {}
            }
            if do_clean_lead_zeroes {
                let mut first = true;
                while let Some(ch_zero) = str_it.next() {
                    // see notes on offset above:
                    // this is why the offset for octals and decimals
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


    let fchar = field.field_char.clone();

    // num format mainly operates by further delegating to one of
    // several Formatter structs depending on the field
    // see formatter.rs for more details

    // to do switch to static dispatch
    let fmtr: Box<Formatter> = match *field.field_type {
        FieldType::Intf => Box::new(Intf::new()),
        FieldType::Floatf => Box::new(Floatf::new()),
        FieldType::CninetyNineHexFloatf => Box::new(CninetyNineHexFloatf::new()),
        FieldType::Scif => Box::new(Scif::new()),
        FieldType::Decf => Box::new(Decf::new()),
        _ => {
            panic!("asked to do num format with non-num fieldtype");
        }
    };
    let prim_opt=
        // if we can get an assumed value from looking at the first
        // few characters, use that value to create the FormatPrimitive
        if let Some(provided_num) = get_provided(in_str_opt) {
            let mut tmp : FormatPrimitive = Default::default();
            match fchar {
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
                    let inprefix = get_inprefix(
                        &as_str,
                        &field.field_type
                    );                    
                    tmp=fmtr.get_primitive(field, &inprefix, &as_str)
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
            let inprefix = get_inprefix(
                in_str,
                &field.field_type
            );
            // then get the FormatPrimitive from the Formatter
            fmtr.get_primitive(field, &inprefix, in_str)
        };
    // if we have a formatPrimitive, print its results
    // according to the field-char appropriate Formatter
    if let Some(prim) = prim_opt {
        Some(fmtr.primitive_to_str(&prim, field.clone()))
    } else {
        None
    }
}
