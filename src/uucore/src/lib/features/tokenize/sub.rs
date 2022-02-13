// spell-checker:ignore (vars) charf decf floatf intf scif strf Cninety

//! Sub is a token that represents a
//! segment of the format string that is a substitution
//! it is created by Sub's implementation of the Tokenizer trait
//! Subs which have numeric field chars make use of the num_format
//! submodule
use crate::error::{UError, UResult};
use itertools::{put_back_n, PutBackN};
use std::error::Error;
use std::fmt::Display;
use std::iter::Peekable;
use std::process::exit;
use std::slice::Iter;
use std::str::Chars;
// use std::collections::HashSet;

use super::num_format::format_field::{FieldType, FormatField};
use super::num_format::num_format;
use super::token;
use super::unescaped_text::UnescapedText;

const EXIT_ERR: i32 = 1;

#[derive(Debug)]
pub enum SubError {
    InvalidSpec(String),
}

impl Display for SubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::InvalidSpec(s) => write!(f, "%{}: invalid conversion specification", s),
        }
    }
}

impl Error for SubError {}

impl UError for SubError {}

fn convert_asterisk_arg_int(asterisk_arg: &str) -> isize {
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
        orig: &asterisk_arg.to_string(),
        field_type: &field_type,
        field_char: &field_char,
    };
    num_format::num_format(&field_info, Some(&asterisk_arg.to_string()))
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
    prefix_char: char,
}
impl Sub {
    pub fn new(
        min_width: CanAsterisk<Option<isize>>,
        second_field: CanAsterisk<Option<u32>>,
        field_char: char,
        orig: String,
        prefix_char: char,
    ) -> Self {
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
                println!("Invalid field type");
                exit(EXIT_ERR);
            }
        };
        Self {
            min_width,
            second_field,
            field_char,
            field_type,
            orig,
            prefix_char,
        }
    }
}

#[derive(Default)]
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
    fn new() -> Self {
        Self::default()
    }
    fn from_it(
        it: &mut PutBackN<Chars>,
        args: &mut Peekable<Iter<String>>,
    ) -> UResult<Option<Box<dyn token::Token>>> {
        let mut parser = Self::new();
        if parser.sub_vals_retrieved(it)? {
            let t: Box<dyn token::Token> = Self::build_token(parser);
            t.print(args);
            Ok(Some(t))
        } else {
            Ok(None)
        }
    }
    fn build_token(parser: Self) -> Box<dyn token::Token> {
        // not a self method so as to allow move of sub-parser vals.
        // return new Sub struct as token
        let prefix_char = match &parser.min_width_tmp {
            Some(width) if width.starts_with('0') => '0',
            _ => ' ',
        };

        let t: Box<dyn token::Token> = Box::new(Sub::new(
            if parser.min_width_is_asterisk {
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
            parser.text_so_far,
            prefix_char,
        ));
        t
    }
    fn sub_vals_retrieved(&mut self, it: &mut PutBackN<Chars>) -> UResult<bool> {
        if !Self::successfully_eat_prefix(it, &mut self.text_so_far)? {
            return Ok(false);
        }
        // this fn in particular is much longer than it needs to be
        // .could get a lot
        // of code savings just by cleaning it up. shouldn't use a regex
        // though, as we want to mimic the original behavior of printing
        // the field as interpreted up until the error in the field.

        let mut legal_fields = vec![
            // 'a', 'A', //c99 hex float implementation not yet complete
            'b', 'c', 'd', 'e', 'E', 'f', 'F', 'g', 'G', 'i', 'o', 's', 'u', 'x', 'X',
        ];
        let mut specifiers = vec!['h', 'j', 'l', 'L', 't', 'z'];
        legal_fields.sort_unstable();
        specifiers.sort_unstable();

        // divide substitution from %([0-9]+)?(.[0-9+])?([a-zA-Z])
        // into min_width, second_field, field_char
        for ch in it {
            self.text_so_far.push(ch);
            match ch as char {
                '-' | '*' | '0'..='9' => {
                    if !self.past_decimal {
                        if self.min_width_is_asterisk || self.specifiers_found {
                            return Err(SubError::InvalidSpec(self.text_so_far.clone()).into());
                        }
                        if self.min_width_tmp.is_none() {
                            self.min_width_tmp = Some(String::new());
                        }
                        match self.min_width_tmp.as_mut() {
                            Some(x) => {
                                if (ch == '-' || ch == '*') && !x.is_empty() {
                                    return Err(
                                        SubError::InvalidSpec(self.text_so_far.clone()).into()
                                    );
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
                            return Err(SubError::InvalidSpec(self.text_so_far.clone()).into());
                        }
                        if self.second_field_tmp.is_none() {
                            self.second_field_tmp = Some(String::new());
                        }
                        match self.second_field_tmp.as_mut() {
                            Some(x) => {
                                if ch == '*' && !x.is_empty() {
                                    return Err(
                                        SubError::InvalidSpec(self.text_so_far.clone()).into()
                                    );
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
                        return Err(SubError::InvalidSpec(self.text_so_far.clone()).into());
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
                    return Err(SubError::InvalidSpec(self.text_so_far.clone()).into());
                }
            }
        }
        if self.field_char.is_none() {
            return Err(SubError::InvalidSpec(self.text_so_far.clone()).into());
        }
        let field_char_retrieved = self.field_char.unwrap();
        if self.past_decimal && self.second_field_tmp.is_none() {
            self.second_field_tmp = Some(String::from("0"));
        }
        self.validate_field_params(field_char_retrieved)?;
        // if the dot is provided without a second field
        // printf interprets it as 0.
        if let Some(x) = self.second_field_tmp.as_mut() {
            if x.is_empty() {
                self.min_width_tmp = Some(String::from("0"));
            }
        }

        Ok(true)
    }
    fn successfully_eat_prefix(
        it: &mut PutBackN<Chars>,
        text_so_far: &mut String,
    ) -> UResult<bool> {
        // get next two chars,
        // if they're '%%' we're not tokenizing it
        // else put chars back
        let preface = it.next();
        let n_ch = it.next();
        if preface == Some('%') && n_ch != Some('%') {
            match n_ch {
                Some(x) => {
                    it.put_back(x);
                    Ok(true)
                }
                None => {
                    text_so_far.push('%');
                    Err(SubError::InvalidSpec(text_so_far.clone()).into())
                }
            }
        } else {
            if let Some(x) = n_ch {
                it.put_back(x);
            };
            if let Some(x) = preface {
                it.put_back(x);
            };
            Ok(false)
        }
    }
    fn validate_field_params(&self, field_char: char) -> UResult<()> {
        // check for illegal combinations here when possible vs
        // on each application so we check less per application
        // to do: move these checks to Sub::new
        if (field_char == 's' && self.min_width_tmp == Some(String::from("0")))
            || (field_char == 'c'
                && (self.min_width_tmp == Some(String::from("0")) || self.past_decimal))
            || (field_char == 'b'
                && (self.min_width_tmp.is_some()
                    || self.past_decimal
                    || self.second_field_tmp.is_some()))
        {
            // invalid string substitution
            // to do: include information about an invalid
            // string substitution
            return Err(SubError::InvalidSpec(self.text_so_far.clone()).into());
        }
        Ok(())
    }
}

impl token::Tokenizer for Sub {
    fn from_it(
        it: &mut PutBackN<Chars>,
        args: &mut Peekable<Iter<String>>,
    ) -> UResult<Option<Box<dyn token::Token>>> {
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
            // if %b use UnescapedText module's unescape-fn
            // if %c return first char of arg
            FieldType::Strf | FieldType::Charf => {
                match pf_arg {
                    Some(arg_string) => {
                        match *field.field_char {
                            's' => Some(match field.second_field {
                                Some(max) => String::from(&arg_string[..max as usize]),
                                None => arg_string.clone(),
                            }),
                            'b' => {
                                let mut a_it = put_back_n(arg_string.chars());
                                UnescapedText::from_it_core(&mut a_it, true);
                                None
                            }
                            // for 'c': get iter of string vals,
                            // get opt<char> of first val
                            // and map it to opt<String>
                            /* 'c' | */
                            _ => arg_string.chars().next().map(|x| x.to_string()),
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
        if let Some(pre_min_width) = pre_min_width_opt {
            // if have a string, print it, ensuring minimum width is met.
            print!(
                "{}",
                match field.min_width {
                    Some(min_width) => {
                        let diff: isize = min_width.abs() as isize - pre_min_width.len() as isize;
                        if diff > 0 {
                            let mut final_str = String::new();
                            // definitely more efficient ways
                            //  to do this.
                            let pad_before = min_width > 0;
                            if !pad_before {
                                final_str.push_str(&pre_min_width);
                            }
                            for _ in 0..diff {
                                final_str.push(self.prefix_char);
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
                }
            );
        }
    }
}
