//! Sub is a token that represents a
//! segment of the format string that is a substitution
//! it is created by Sub's implementation of the Tokenizer trait
//! Subs which have numeric field chars make use of the num_format
//! submodule
use std::slice::Iter;
use std::iter::Peekable;
use std::str::Chars;
use std::process::exit;
use cli;
use itertools::{PutBackN, put_back_n};
use super::token;
use super::unescaped_text::UnescapedText;
use super::num_format::format_field::{FormatField, FieldType};
use super::num_format::num_format;
// use std::collections::HashSet;

fn err_conv(sofar: &String) {
    cli::err_msg(&format!("%{}: invalid conversion specification", sofar));
    exit(cli::EXIT_ERR);
}

fn convert_asterisk_arg_int(asterisk_arg: &String) -> isize {
    // this is a costly way to parse the
    // args used for asterisk values into integers
    // from various bases. Actually doing it correctly
    // (going through the pipeline to intf, but returning
    // the integer instead of writing it to string and then
    // back) is on the refactoring TODO
    let field_type = FieldType::Intf;
    let field_char = 'i';
    let field_info = FormatField {
        min_width: Some(0),
        second_field: Some(0),
        orig: asterisk_arg,
        field_type: &field_type,
        field_char: &field_char,
    };
    num_format::num_format(&field_info, Some(asterisk_arg))
        .unwrap()
        .parse::<isize>()
        .unwrap()
}

pub enum CanAsterisk<T> {
    Fixed(T),
    Asterisk,
}

// Sub is a tokenizer which creates tokens
// for substitution segments of a format string
pub struct Sub {
    min_width: CanAsterisk<Option<isize>>,
    second_field: CanAsterisk<Option<u32>>,
    field_char: char,
    field_type: FieldType,
    orig: String,
}
impl Sub {
    pub fn new(min_width: CanAsterisk<Option<isize>>,
               second_field: CanAsterisk<Option<u32>>,
               field_char: char,
               orig: String)
               -> Sub {
        // for more dry printing, field characters are grouped
        // in initialization of token.
        let field_type = match field_char {
            's' | 'b' => FieldType::Strf,
            'd' | 'i' | 'u' | 'o' | 'x' | 'X' => FieldType::Intf,
            'f' | 'F' => FieldType::Floatf,
            'a' | 'A' => FieldType::CninetyNineHexFloatf,
            'e' | 'E' => FieldType::Scif,
            'g' | 'G' => FieldType::Decf,
            'c' => FieldType::Charf,
            _ => {
                // should be unreachable.
                println!("Invalid fieldtype");
                exit(cli::EXIT_ERR);
            }
        };
        Sub {
            min_width: min_width,
            second_field: second_field,
            field_char: field_char,
            field_type: field_type,
            orig: orig,
        }
    }
}

struct SubParser {
    min_width_tmp: Option<String>,
    min_width_is_asterisk: bool,
    past_decimal: bool,
    second_field_tmp: Option<String>,
    second_field_is_asterisk: bool,
    specifiers_found: bool,
    field_char: Option<char>,
    text_so_far: String,
}

impl SubParser {
    fn new() -> SubParser {
        SubParser {
            min_width_tmp: None,
            min_width_is_asterisk: false,
            past_decimal: false,
            second_field_tmp: None,
            second_field_is_asterisk: false,
            specifiers_found: false,
            field_char: None,
            text_so_far: String::new(),
        }
    }
    fn from_it(it: &mut PutBackN<Chars>,
               args: &mut Peekable<Iter<String>>)
               -> Option<Box<token::Token>> {
        let mut parser = SubParser::new();
        if parser.sub_vals_retrieved(it) {
            let t: Box<token::Token> = SubParser::build_token(parser);
            t.print(args);
            Some(t)
        } else {
            None
        }
    }
    fn build_token(parser: SubParser) -> Box<token::Token> {
        // not a self method so as to allow move of subparser vals.
        // return new Sub struct as token
        let t: Box<token::Token> = Box::new(Sub::new(if parser.min_width_is_asterisk {
                                                         CanAsterisk::Asterisk
                                                     } else {
                                                         CanAsterisk::Fixed(parser.min_width_tmp.map(|x| x.parse::<isize>().unwrap()))
                                                     },
                                                     if parser.second_field_is_asterisk {
                                                         CanAsterisk::Asterisk
                                                     } else {
                                                         CanAsterisk::Fixed(parser.second_field_tmp.map(|x| x.parse::<u32>().unwrap()))
                                                     },
                                                     parser.field_char.unwrap(),
                                                     parser.text_so_far));
        t
    }
    fn sub_vals_retrieved(&mut self, it: &mut PutBackN<Chars>) -> bool {

        if !SubParser::successfully_eat_prefix(it, &mut self.text_so_far) {
            return false;
        }
        // this fn in particular is much longer than it needs to be
        // .could get a lot
        // of code savings just by cleaning it up. shouldn't use a regex
        // though, as we want to mimic the original behavior of printing
        // the field as interpreted up until the error in the field.

        let mut legal_fields = vec![// 'a', 'A', //c99 hex float implementation not yet complete
                                    'b',
                                    'c',
                                    'd',
                                    'e',
                                    'E',
                                    'f',
                                    'F',
                                    'g',
                                    'G',
                                    'i',
                                    'o',
                                    's',
                                    'u',
                                    'x',
                                    'X'];
        let mut specifiers = vec!['h', 'j', 'l', 'L', 't', 'z'];
        legal_fields.sort();
        specifiers.sort();

        // divide substitution from %([0-9]+)?(.[0-9+])?([a-zA-Z])
        // into min_width, second_field, field_char
        while let Some(ch) = it.next() {
            self.text_so_far.push(ch);
            match ch as char {
                '-' | '*' | '0'...'9' => {
                    if !self.past_decimal {
                        if self.min_width_is_asterisk || self.specifiers_found {
                            err_conv(&self.text_so_far);
                        }
                        if self.min_width_tmp.is_none() {
                            self.min_width_tmp = Some(String::new());
                        }
                        match self.min_width_tmp.as_mut() {
                            Some(x) => {
                                if (ch == '-' || ch == '*') && x.len() > 0 {
                                    err_conv(&self.text_so_far);
                                }
                                if ch == '*' {
                                    self.min_width_is_asterisk = true;
                                }
                                x.push(ch);
                            }
                            None => {
                                panic!("should be unreachable");
                            } 
                        }
                    } else {
                        // second field should never have a
                        // negative value
                        if self.second_field_is_asterisk || ch == '-' || self.specifiers_found {
                            err_conv(&self.text_so_far);
                        }
                        if self.second_field_tmp.is_none() {
                            self.second_field_tmp = Some(String::new());
                        }
                        match self.second_field_tmp.as_mut() {
                            Some(x) => {
                                if ch == '*' && x.len() > 0 {
                                    err_conv(&self.text_so_far);
                                }
                                if ch == '*' {
                                    self.second_field_is_asterisk = true;
                                }
                                x.push(ch);
                            }
                            None => {
                                panic!("should be unreachable");
                            } 
                        }
                    }
                }
                '.' => {
                    if !self.past_decimal {
                        self.past_decimal = true;
                    } else {
                        err_conv(&self.text_so_far);
                    }
                }
                x if legal_fields.binary_search(&x).is_ok() => {
                    self.field_char = Some(ch);
                    self.text_so_far.push(ch);
                    break;
                }
                x if specifiers.binary_search(&x).is_ok() => {
                    if !self.past_decimal {
                        self.past_decimal = true;
                    }
                    if !self.specifiers_found {
                        self.specifiers_found = true;
                    }
                }
                _ => {
                    err_conv(&self.text_so_far);
                }
            }
        }
        if !self.field_char.is_some() {
            err_conv(&self.text_so_far);
        }
        let field_char_retrieved = self.field_char.unwrap();
        if self.past_decimal && self.second_field_tmp.is_none() {
            self.second_field_tmp = Some(String::from("0"));
        }
        self.validate_field_params(field_char_retrieved);
        // if the dot is provided without a second field
        // printf interprets it as 0.
        match self.second_field_tmp.as_mut() {
            Some(x) => {
                if x.len() == 0 {
                    self.min_width_tmp = Some(String::from("0"));
                }
            }
            _ => {}
        }

        true
    }
    fn successfully_eat_prefix(it: &mut PutBackN<Chars>, text_so_far: &mut String) -> bool {
        // get next two chars,
        // if they're '%%' we're not tokenizing it
        // else put chars back
        let preface = it.next();
        let n_ch = it.next();
        if preface == Some('%') && n_ch != Some('%') {
            match n_ch {
                Some(x) => {
                    it.put_back(x);
                    true
                }
                None => {
                    text_so_far.push('%');
                    err_conv(&text_so_far);
                    false
                }
            }
        } else {
            n_ch.map(|x| it.put_back(x));
            preface.map(|x| it.put_back(x));
            false
        }
    }
    fn validate_field_params(&self, field_char: char) {
        // check for illegal combinations here when possible vs
        // on each application so we check less per application
        // to do: move these checks to Sub::new
        if (field_char == 's' && self.min_width_tmp == Some(String::from("0"))) ||
           (field_char == 'c' &&
            (self.min_width_tmp == Some(String::from("0")) || self.past_decimal)) ||
           (field_char == 'b' &&
            (self.min_width_tmp.is_some() || self.past_decimal ||
             self.second_field_tmp.is_some())) {
            err_conv(&self.text_so_far);
        }
    }
}



impl token::Tokenizer for Sub {
    fn from_it(it: &mut PutBackN<Chars>,
               args: &mut Peekable<Iter<String>>)
               -> Option<Box<token::Token>> {
        SubParser::from_it(it, args)
    }
}
impl token::Token for Sub {
    fn print(&self, pf_args_it: &mut Peekable<Iter<String>>) {
        let field = FormatField {
            min_width: match self.min_width {
                CanAsterisk::Fixed(x) => x,
                CanAsterisk::Asterisk => {
                    match pf_args_it.next() {
                        // temporary, use intf.rs instead
                        Some(x) => Some(convert_asterisk_arg_int(x)),
                        None => Some(0),
                    }
                }
            },
            second_field: match self.second_field {
                CanAsterisk::Fixed(x) => x,
                CanAsterisk::Asterisk => {
                    match pf_args_it.next() {
                        // temporary, use intf.rs instead
                        Some(x) => {
                            let result = convert_asterisk_arg_int(x);
                            if result < 0 {
                                None
                            } else {
                                Some(result as u32)
                            }
                        }
                        None => Some(0),
                    }
                }                
            },
            field_char: &self.field_char,
            field_type: &self.field_type,
            orig: &self.orig,
        };
        let pf_arg = pf_args_it.next();

        // minimum width is handled independently of actual
        // field char
        let pre_min_width_opt: Option<String> = match *field.field_type {
            // if %s just return arg
            // if %b use UnescapedText module's unescaping-fn
            // if %c return first char of arg
            FieldType::Strf | FieldType::Charf => {
                match pf_arg { 
                    Some(arg_string) => {
                        match *field.field_char {
                            's' => {
                                Some(match field.second_field {
                                    Some(max) => String::from(&arg_string[..max as usize]),
                                    None => arg_string.clone(),
                                })
                            }
                            'b' => {
                                let mut a_it = put_back_n(arg_string.chars());
                                UnescapedText::from_it_core(&mut a_it, true);
                                None
                            }
                            // for 'c': get iter of string vals,
                            // get opt<char> of first val
                            // and map it to opt<String>
                            'c' | _ => arg_string.chars().next().map(|x| x.to_string()),
                        }
                    }
                    None => None,
                }
            }
            _ => {
                // non string/char fields are delegated to num_format
                num_format::num_format(&field, pf_arg)
            }
        };
        match pre_min_width_opt {
            // if have a string, print it, ensuring minimum width is met.
            Some(pre_min_width) => {
                print!("{}",
                       match field.min_width {
                           Some(min_width) => {
                               let diff: isize = min_width.abs() as isize -
                                                 pre_min_width.len() as isize;
                               if diff > 0 {
                                   let mut final_str = String::new();
                                   // definitely more efficient ways
                                   //  to do this.
                                   let pad_before = min_width > 0;
                                   if !pad_before {
                                       final_str.push_str(&pre_min_width);
                                   }
                                   for _ in 0..diff {
                                       final_str.push(' ');
                                   }
                                   if pad_before {
                                       final_str.push_str(&pre_min_width);
                                   }
                                   final_str
                               } else {
                                   pre_min_width
                               }
                           }
                           None => pre_min_width,
                       });
            }
            None => {}
        }
    }
}
