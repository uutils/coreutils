#![crate_name = "uu_echo"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Derek Chiang <derekchiang93@gmail.com>
 * (c) Christopher Brown <ccbrown112@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[macro_use]
extern crate uucore;

use std::iter::Peekable;
use std::str::Chars;

const SYNTAX: &str = "[OPTIONS]... [STRING]...";
const SUMMARY: &str = "display a line of text";
const HELP: &str = r#"
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

fn parse_code(input: &mut Peekable<Chars>, base: u32, max_digits: u32, bits_per_digit: u32) -> Option<char> {
    let mut ret = 0x80000000;
    for _ in 0..max_digits {
        match input.peek().and_then(|c| c.to_digit(base)) {
            Some(n) => ret = (ret << bits_per_digit) | n,
            None => break,
        }
        input.next();
    }
    std::char::from_u32(ret)
}

fn print_escaped(input: &str, should_stop: &mut bool) {
    let mut iter = input.chars().peekable();
    while let Some(mut c) = iter.next() {
        if c == '\\' {
            if let Some(next) = iter.next() {
                c = match next {
                    '\\' => '\\',
                    'a' => '\x07',
                    'b' => '\x08',
                    'c' => {
                        *should_stop = true;
                        break
                    },
                    'e' => '\x1b',
                    'f' => '\x0c',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    'v' => '\x0b',
                    'x' => parse_code(&mut iter, 16, 2, 4).unwrap_or_else(|| {
                        print!("\\");
                        next
                    }),
                    '0' => parse_code(&mut iter, 8, 3, 3).unwrap_or_else(|| {
                        print!("\\");
                        next
                    }),
                    _ => {
                        print!("\\");
                        next
                    },
                };
            }
        }
        print!("{}", c);
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let matches = new_coreopts!(SYNTAX, SUMMARY, HELP)
        .optflag("n", "", "do not output the trailing newline")
        .optflag("e", "", "enable interpretation of backslash escapes")
        .optflag("E", "", "disable interpretation of backslash escapes (default)")
        .parse(args);

    let no_newline = matches.opt_present("n");
    let escaped = matches.opt_present("e");

    for (i, input) in matches.free.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        if escaped {
            let mut should_stop = false;
            print_escaped(&input, &mut should_stop);
            if should_stop {
                break;
            }
        } else {
            print!("{}", input);
        }
    }

    if !no_newline {
        println!();
    }

    0
}
