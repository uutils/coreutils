// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.
//

#![crate_name = "uu_base32"]

#[macro_use]
extern crate uucore;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate failure_derive;

use uucore::encoding::{Data, Format, wrap_print};
use uucore::{ProgramInfo, UStatus, Util};

use std::fs::File;
use std::io::{self, BufReader, Read, Write, stdin};
use std::path::Path;

static SYNTAX: &'static str = "[OPTION]... [FILE]";
static SUMMARY: &'static str = "Base32 encode or decode FILE, or standard input, to standard output.";
static LONG_HELP: &'static str = "
 With no FILE, or when FILE is -, read standard input.

 The data are encoded as described for the base32 alphabet in RFC
 4648. When decoding, the input may contain newlines in addition
 to the bytes of the formal base32 alphabet. Use --ignore-garbage
 to attempt to recover from any other non-alphabet bytes in the
 encoded stream.
";

pub const UTILITY: Base32 = Base32;

pub struct Base32;

impl<'a, I: Read, O: Write, E: Write> Util<'a, I, O, E, Error> for Base32 {
    fn uumain(args: Vec<String>, pio: &mut ProgramInfo<I, O, E>) -> Result<i32, Error> {
        let matches = new_coreopts!(SYNTAX, SUMMARY, LONG_HELP)
            .optflag("d", "decode", "decode data")
            .optflag("i",
                     "ignore-garbage",
                     "when decoding, ignore non-alphabetic characters")
            .optopt("w",
                    "wrap",
                    "wrap encoded lines after COLS character (default 76, 0 to disable wrapping)",
                    "COLS")
            .parse(args, pio)?;
    
        if matches.is_none() {
            return Ok(0);
        }
        let matches = matches.unwrap();

        let line_wrap = match matches.opt_str("wrap") {
            Some(s) => {
                match s.parse() {
                    Ok(wrap) => wrap,
                    Err(f) => return Err(Error::ParseWrap(f, s))
                }
            }
            None => 76,
        };

        if matches.free.len() > 1 {
            Err(format_err!("extra operand '{}'", matches.free[0]))?;
        }

        let input = if matches.free.is_empty() || &matches.free[0][..] == "-" {
            BufReader::new(Box::new(stdin()) as Box<Read>)
        } else {
            let path = Path::new(matches.free[0].as_str());
            let file_buf = File::open(&path)?;
            BufReader::new(Box::new(file_buf) as Box<Read>)
        };

        let mut data = Data::new(input, Format::Base32)
            .line_wrap(line_wrap)
            .ignore_garbage(matches.opt_present("ignore-garbage"));

        if !matches.opt_present("decode") {
            wrap_print(line_wrap, data.encode());
        } else {
            match data.decode() {
                Ok(s) => write!(pio, "{}", String::from_utf8(s).unwrap())?,
                Err(_) => Err(format_err!("invalid input"))?,
            }
        }

        Ok(0)
    }
}

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "{}", _0)]
    Io(#[cause] io::Error),

    #[fail(display = "{}", _0)]
    CoreOpts(#[cause] uucore::coreopts::Error),

    #[fail(display = "invalid wrap size: '{}': {}", _1, _0)]
    ParseWrap(#[cause] ::std::num::ParseIntError, String),

    #[fail(display = "{}", _0)]
    General(failure::Error)
}

impl UStatus for Error { }

generate_from_impl!(Error, Io, io::Error);
generate_from_impl!(Error, CoreOpts, uucore::coreopts::Error);
generate_from_impl!(Error, General, failure::Error);

//generate_error_type!(Base32Error, uucore::coreopts::CoreOptionsError, _);