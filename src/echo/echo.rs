#![crate_name = "uu_echo"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Derek Chiang <derekchiang93@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[macro_use]
extern crate uucore;

use std::io::Write;
use std::str::from_utf8;

#[allow(dead_code)]
static SYNTAX: &'static str = "[OPTIONS]... [STRING]..."; 
static SUMMARY: &'static str = "display a line of text"; 
static LONG_HELP: &'static str = r#"
 Echo the STRING(s) to standard output.
 If -e is in effect, the following sequences are recognized:

 \\\\      backslash
 \\a      alert (BEL)
 \\b      backspace
 \\c      produce no further output
 \\e      escape
 \\f      form feed
 \\n      new line
 \\r      carriage return
 \\t      horizontal tab
 \\v      vertical tab
 \\0NNN   byte with octal value NNN (1 to 3 digits)
 \\xHH    byte with hexadecimal value HH (1 to 2 digits)
"#; 

#[derive(Clone)]
struct EchoOptions {
    newline: bool,
    escape: bool
}

#[inline(always)]
fn to_char(bytes: &[u8], base: u32) -> char {
    usize::from_str_radix(from_utf8(bytes.as_ref()).unwrap(), base).unwrap() as u8 as char
}

#[inline(always)]
fn isxdigit(c: u8) -> bool {
    match c as char {
        '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' |
        '8' | '9' | 'A' | 'B' | 'C' | 'D' | 'E' | 'F' => true,
        _ => false
    }
}

#[inline(always)]
fn isodigit(c: u8) -> bool {
    match c as char {
        '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' => true,
        _ => false
    }
}

fn convert_str(string: &[u8], index: usize, base: u32) -> (char, usize) {
    let (max_digits, is_legal_digit) : (usize, fn(u8) -> bool) = match base {
        8 => (3, isodigit),
        16 => (2, isxdigit),
        _ => panic!(),
    };

    let mut bytes = vec!();
    for offset in 0usize .. max_digits {
        if string.len() <= index + offset as usize {
            break;
        }
        let c = string[index + offset as usize];
        if is_legal_digit(c) {
            bytes.push(c as u8);
        } else {
            break;
        }
    }

    if bytes.is_empty() {
        (' ', 0)
    } else {
        (to_char(&bytes, base), bytes.len())
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut options = EchoOptions {
        newline: false,
        escape: false
    };

    let matches = new_coreopts!(SYNTAX, SUMMARY, LONG_HELP)
        .optflag("n", "", "do not output the trailing newline")
        .optflag("e", "", "enable interpretation of backslash escapes")
        .optflag("E", "", "disable interpretation of backslash escapes (default)")
        .parse(args);

    options.newline = matches.opt_present("n");
    options.escape = matches.opt_present("e");
    let free = matches.free;
    if !free.is_empty() {
        let string = free.join(" ");
        if options.escape {
            let mut prev_was_slash = false;
            let mut iter = string.chars().enumerate();
            while let Some((index, c)) = iter.next() {
                if !prev_was_slash {
                    if c != '\\' {
                        print!("{}", c);
                    } else {
                        prev_was_slash = true;
                    }
                } else {
                    prev_was_slash = false;
                    match c {
                        '\\' => print!("\\"),
                        'a' => print!("\x07"),
                        'b' => print!("\x08"),
                        'c' => break,
                        'e' => print!("\x1B"),
                        'f' => print!("\x0C"),
                        'n' => print!("\n"),
                        'r' => print!("\r"),
                        't' => print!("\t"),
                        'v' => print!("\x0B"),
                        'x' => {
                            let (c, num_char_used) = convert_str(string.as_bytes(), index + 1, 16);
                            if num_char_used == 0 {
                                print!("\\x");
                            } else {
                                print!("{}", c);
                                for _ in 0 .. num_char_used {
                                    iter.next(); // consume used characters
                                }
                            }
                        },
                        '0' => {
                            let (c, num_char_used) = convert_str(string.as_bytes(), index + 1, 8);
                            if num_char_used == 0 {
                                print!("\0");
                            } else {
                                print!("{}", c);
                                for _ in 0 .. num_char_used {
                                    iter.next(); // consume used characters
                                }
                            }
                        }
                        _ => {
                            let (esc_c, num_char_used) = convert_str(string.as_bytes(), index, 8);
                            if num_char_used == 0 {
                                print!("\\{}", c);
                            } else {
                                print!("{}", esc_c);
                                for _ in 1 .. num_char_used {
                                    iter.next(); // consume used characters
                                }
                            }
                        }
                    }
                }
            }
        } else {
            print!("{}", string);
        }
    }

    if options.newline {
        pipe_flush!();
    } else {
        println!("")
    }

    0
}
