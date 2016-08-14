#![crate_name = "uu_base64"]

// This file is part of the uutils coreutils package.
//
// (c) Jordy Dickinson <jordy.dickinson@gmail.com>
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.
//


#[macro_use]
extern crate uucore;
use uucore::encoding::{Data, Format, wrap_print};

use std::fs::File;
use std::io::{BufReader, Read, stdin, Write};
use std::path::Path;

static SYNTAX: &'static str = "[OPTION]... [FILE]"; 
static SUMMARY: &'static str = "Base64 encode or decode FILE, or standard input, to standard output."; 
static LONG_HELP: &'static str = "
 With no FILE, or when FILE is -, read standard input.

 The data are encoded as described for the base64 alphabet in RFC
 3548. When decoding, the input may contain newlines in addition
 to the bytes of the formal base64 alphabet. Use --ignore-garbage
 to attempt to recover from any other non-alphabet bytes in the
 encoded stream.
";

pub fn uumain(args: Vec<String>) -> i32 {
    let matches = new_coreopts!(SYNTAX, SUMMARY, LONG_HELP)
        .optflag("d", "decode", "decode data")
        .optflag("i",
                 "ignore-garbage",
                 "when decoding, ignore non-alphabetic characters")
        .optopt("w",
                "wrap",
                "wrap encoded lines after COLS character (default 76, 0 to disable wrapping)",
                "COLS")
        .parse(args);

    let line_wrap = match matches.opt_str("wrap") {
        Some(s) => {
            match s.parse() {
                Ok(n) => n,
                Err(e) => {
                    crash!(1, "invalid wrap size: ‘{}’: {}", s, e);
                }
            }
        }
        None => 76,
    };

    if matches.free.len() > 1 {
        disp_err!("extra operand ‘{}’", matches.free[0]);
        return 1;
    }

    let input = if matches.free.is_empty() || &matches.free[0][..] == "-" {
        BufReader::new(Box::new(stdin()) as Box<Read>)
    } else {
        let path = Path::new(matches.free[0].as_str());
        let file_buf = safe_unwrap!(File::open(&path));
        BufReader::new(Box::new(file_buf) as Box<Read>)
    };

    let mut data = Data::new(input, Format::Base64)
        .line_wrap(line_wrap)
        .ignore_garbage(matches.opt_present("ignore-garbage"));

    if !matches.opt_present("decode") {
        wrap_print(line_wrap, data.encode());
    } else {
        match data.decode() {
            Ok(s) => print!("{}", String::from_utf8(s).unwrap()),
            Err(_) => crash!(1, "invalid input"),
        }
    }

    0
}
