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

use std::io::{stdout, Write};
use std::str::from_utf8;

#[allow(dead_code)]
static SYNTAX: &str = "[OPTIONS]... [STRING]...";
static SUMMARY: &str = "display a line of text";
static HELP: &str = r#"
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

enum Base {
    B8 = 8,
    B16 = 16,
}

struct Opts {
    newline: bool,
    escape: bool,
}

fn convert_str(string: &[u8], index: usize, base: Base) -> (char, usize) {
    let (max_digits, is_legal_digit): (usize, fn(u8) -> bool) = match base {
        Base::B8 => (3, |c| (c as char).is_digit(8)),
        Base::B16 => (2, |c| (c as char).is_digit(16)),
    };

    let mut bytes = vec![];
    for offset in 0..max_digits {
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
        (
            usize::from_str_radix(from_utf8(bytes.as_ref()).unwrap(), base as u32).unwrap() as u8
                as char,
            bytes.len(),
        )
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let matches = new_coreopts!(SYNTAX, SUMMARY, HELP)
        .optflag("n", "", "do not output the trailing newline")
        .optflag("e", "", "enable interpretation of backslash escapes")
        .optflag(
            "E",
            "",
            "disable interpretation of backslash escapes (default)",
        )
        .parse(args);

    let options = Opts {
        newline: matches.opt_present("n"),
        escape: matches.opt_present("e"),
    };
    let free = matches.free;
    if !free.is_empty() {
        let string = free.join(" ");
        if options.escape {
            let mut prev_was_slash = false;
            let mut iter = string.chars().enumerate();
            while let Some((mut idx, c)) = iter.next() {
                prev_was_slash = if !prev_was_slash {
                    if c != '\\' {
                        print!("{}", c);
                        false
                    } else {
                        true
                    }
                } else {
                    match c {
                        '\\' => print!("\\"),
                        'n' => print!("\n"),
                        'r' => print!("\r"),
                        't' => print!("\t"),
                        'v' => print!("\x0B"),
                        'a' => print!("\x07"),
                        'b' => print!("\x08"),
                        'c' => break,
                        'e' => print!("\x1B"),
                        'f' => print!("\x0C"),
                        ch => {
                            // 'x' or '0' or _
                            idx = if ch == 'x' || ch == '0' { idx + 1 } else { idx };
                            let base = if ch == 'x' { Base::B16 } else { Base::B8 };
                            match convert_str(string.as_bytes(), idx, base) {
                                (_, 0) => match ch {
                                    'x' => print!("\\x"),
                                    '0' => print!("\0"),
                                    _ => print!("\\{}", c),
                                },
                                (c, num_char_used) => {
                                    print!("{}", c);
                                    let beg = if ch == 'x' || ch == '0' { 0 } else { 1 };
                                    for _ in beg..num_char_used {
                                        iter.next(); // consume used characters
                                    }
                                }
                            }
                        }
                    }
                    false
                }
            }
        } else {
            print!("{}", string);
        }
    }

    if options.newline {
        return_if_err!(1, stdout().flush())
    } else {
        println!()
    }

    0
}
