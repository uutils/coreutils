#![crate_name = "base64"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordy Dickinson <jordy.dickinson@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern crate rustc_serialize as serialize;
extern crate getopts;
extern crate libc;
#[macro_use] extern crate log;

use std::ascii::AsciiExt;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Read, stdin, stdout, Write};
use std::path::Path;

use getopts::Options;
use serialize::base64;
use serialize::base64::{FromBase64, ToBase64};

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "base64";

pub type FileOrStdReader = BufReader<Box<Read+'static>>;

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();
    opts.optflag("d", "decode", "decode data");
    opts.optflag("i", "ignore-garbage", "when decoding, ignore non-alphabetic characters");
    opts.optopt("w", "wrap",
            "wrap encoded lines after COLS character (default 76, 0 to disable wrapping)", "COLS"
        );
    opts.optflag("h", "help", "display this help text and exit");
    opts.optflag("V", "version", "output version information and exit");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => {
            crash!(1, "error: {}", e);
        }
    };

    let progname = args[0].clone();
    let usage = opts.usage("Base64 encode or decode FILE, or standard input, to standard output.");
    let mode = if matches.opt_present("help") {
        Mode::Help
    } else if matches.opt_present("version") {
        Mode::Version
    } else if matches.opt_present("decode") {
        Mode::Decode
    } else {
        Mode::Encode
    };
    let ignore_garbage = matches.opt_present("ignore-garbage");
    let line_wrap = match matches.opt_str("wrap") {
        Some(s) => match s.parse() {
            Ok(s) => s,
            Err(e)=> {
                crash!(1, "error: Argument to option 'wrap' improperly formatted: {}", e);
            }
        },
        None => 76
    };
    let mut stdin_buf;
    let mut file_buf;
    let mut input = if matches.free.is_empty() || &matches.free[0][..] == "-" {
        stdin_buf = stdin();
        BufReader::new(Box::new(stdin_buf) as Box<Read+'static>)
    } else {
        let path = Path::new(&matches.free[0][..]);
        file_buf = safe_unwrap!(File::open(&path));
        BufReader::new(Box::new(file_buf) as Box<Read+'static>)
    };

    match mode {
        Mode::Decode  => decode(&mut input, ignore_garbage),
        Mode::Encode  => encode(&mut input, line_wrap),
        Mode::Help    => help(&progname[..], &usage[..]),
        Mode::Version => version()
    }

    0
}

fn decode(input: &mut FileOrStdReader, ignore_garbage: bool) {
    let mut to_decode = String::new();
    input.read_to_string(&mut to_decode).unwrap();

    if ignore_garbage {
        let mut clean = String::new();
        clean.extend(to_decode.chars().filter(|&c| {
            if !c.is_ascii() {
                false
            } else {
                c >= 'a' && c <= 'z' ||
                c >= 'A' && c <= 'Z' ||
                c >= '0' && c <= '9' ||
                c == '+' || c == '/'
            }
        }));
        to_decode = clean;
    }

    match to_decode[..].from_base64() {
        Ok(bytes) => {
            let mut out = stdout();

            match out.write_all(&bytes[..]) {
                Ok(_) => {}
                Err(f) => { crash!(1, "{}", f); }
            }
            match out.flush() {
                Ok(_) => {}
                Err(f) => { crash!(1, "{}", f); }
            }
        }
        Err(s) => {
            crash!(1, "error: {} ({:?})", s.description(), s);
        }
    }
}

fn encode(input: &mut FileOrStdReader, line_wrap: usize) {
    let b64_conf = base64::Config {
        char_set: base64::Standard,
        newline: base64::Newline::LF,
        pad: true,
        line_length: match line_wrap {
            0 => None,
            _ => Some(line_wrap)
        }
    };
    let mut to_encode: Vec<u8> = vec!();
    input.read_to_end(&mut to_encode).unwrap();
    let encoded = to_encode.to_base64(b64_conf);

    println!("{}", &encoded[..]);
}

fn help(progname: &str, usage: &str) {
    println!("Usage: {} [OPTION]... [FILE]", progname);
    println!("");
    println!("{}", usage);

    let msg = "With no FILE, or when FILE is -, read standard input.\n\n\
        The data are encoded as described for the base64 alphabet in RFC \
        3548. When\ndecoding, the input may contain newlines in addition \
        to the bytes of the formal\nbase64 alphabet. Use --ignore-garbage \
        to attempt to recover from any other\nnon-alphabet bytes in the \
        encoded stream.";

    println!("{}", msg);
}

fn version() {
    println!("base64 1.0.0");
}

enum Mode {
    Decode,
    Encode,
    Help,
    Version
}
