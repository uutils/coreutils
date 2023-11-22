// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use os_display::Quotable;

use crate::{error::set_exit_code, show_warning};

/// An argument for formatting
///
/// Each of these variants is only accepted by their respective directives. For
/// example, [`FormatArgument::Char`] requires a `%c` directive.
///
/// The [`FormatArgument::Unparsed`] variant contains a string that can be
/// parsed into other types. This is used by the `printf` utility.
#[derive(Clone, Debug)]
pub enum FormatArgument {
    Char(char),
    String(String),
    UnsignedInt(u64),
    SignedInt(i64),
    Float(f64),
    /// Special argument that gets coerced into the other variants
    Unparsed(String),
}

pub trait ArgumentIter<'a>: Iterator<Item = &'a FormatArgument> {
    fn get_char(&mut self) -> char;
    fn get_i64(&mut self) -> i64;
    fn get_u64(&mut self) -> u64;
    fn get_f64(&mut self) -> f64;
    fn get_str(&mut self) -> &'a str;
}

impl<'a, T: Iterator<Item = &'a FormatArgument>> ArgumentIter<'a> for T {
    fn get_char(&mut self) -> char {
        let Some(next) = self.next() else {
            return '\0';
        };
        match next {
            FormatArgument::Char(c) => *c,
            FormatArgument::Unparsed(s) => {
                let mut chars = s.chars();
                let Some(c) = chars.next() else {
                    return '\0';
                };
                let None = chars.next() else {
                    return '\0';
                };
                c
            }
            _ => '\0',
        }
    }

    fn get_u64(&mut self) -> u64 {
        let Some(next) = self.next() else {
            return 0;
        };
        match next {
            FormatArgument::UnsignedInt(n) => *n,
            FormatArgument::Unparsed(s) => {
                let opt = if let Some(s) = s.strip_prefix("0x") {
                    u64::from_str_radix(s, 16).ok()
                } else if let Some(s) = s.strip_prefix('0') {
                    u64::from_str_radix(s, 8).ok()
                } else if let Some(s) = s.strip_prefix('\'') {
                    s.chars().next().map(|c| c as u64)
                } else {
                    s.parse().ok()
                };
                match opt {
                    Some(n) => n,
                    None => {
                        show_warning!("{}: expected a numeric value", s.quote());
                        set_exit_code(1);
                        0
                    }
                }
            }
            _ => 0,
        }
    }

    fn get_i64(&mut self) -> i64 {
        let Some(next) = self.next() else {
            return 0;
        };
        match next {
            FormatArgument::SignedInt(n) => *n,
            FormatArgument::Unparsed(s) => {
                // For hex, we parse `u64` because we do not allow another
                // minus sign. We might need to do more precise parsing here.
                let opt = if let Some(s) = s.strip_prefix("-0x") {
                    u64::from_str_radix(s, 16).ok().map(|x| -(x as i64))
                } else if let Some(s) = s.strip_prefix("0x") {
                    u64::from_str_radix(s, 16).ok().map(|x| x as i64)
                } else if s.starts_with("-0") || s.starts_with('0') {
                    i64::from_str_radix(s, 8).ok()
                } else if let Some(s) = s.strip_prefix('\'') {
                    s.chars().next().map(|x| x as i64)
                } else {
                    s.parse().ok()
                };
                match opt {
                    Some(n) => n,
                    None => {
                        show_warning!("{}: expected a numeric value", s.quote());
                        set_exit_code(1);
                        0
                    }
                }
            }
            _ => 0,
        }
    }

    fn get_f64(&mut self) -> f64 {
        let Some(next) = self.next() else {
            return 0.0;
        };
        match next {
            FormatArgument::Float(n) => *n,
            FormatArgument::Unparsed(s) => {
                let opt = if s.starts_with("0x") || s.starts_with("-0x") {
                    unimplemented!("Hexadecimal floats are unimplemented!")
                } else if let Some(s) = s.strip_prefix('\'') {
                    s.chars().next().map(|x| x as u64 as f64)
                } else {
                    s.parse().ok()
                };
                match opt {
                    Some(n) => n,
                    None => {
                        show_warning!("{}: expected a numeric value", s.quote());
                        set_exit_code(1);
                        0.0
                    }
                }
            }
            _ => 0.0,
        }
    }

    fn get_str(&mut self) -> &'a str {
        match self.next() {
            Some(FormatArgument::Unparsed(s) | FormatArgument::String(s)) => s,
            _ => "",
        }
    }
}
