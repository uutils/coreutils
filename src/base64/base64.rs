#![crate_name = "base64"]
#![feature(collections, core, io, libc, path, rustc_private, std_misc)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordy Dickinson <jordy.dickinson@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern crate serialize;
extern crate getopts;
extern crate libc;
#[macro_use] extern crate log;

use std::ascii::AsciiExt;
use std::error::Error;
use std::old_io::{println, File, stdout};
use std::old_io::stdio::stdin_raw;

use getopts::{
    getopts,
    optflag,
    optopt,
    usage
};
use serialize::base64;
use serialize::base64::{FromBase64, ToBase64};

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "base64";

pub fn uumain(args: Vec<String>) -> isize {
    let opts = [
        optflag("d", "decode", "decode data"),
        optflag("i", "ignore-garbage", "when decoding, ignore non-alphabetic characters"),
        optopt("w", "wrap",
            "wrap encoded lines after COLS character (default 76, 0 to disable wrapping)", "COLS"
        ),
        optflag("h", "help", "display this help text and exit"),
        optflag("V", "version", "output version information and exit")
    ];
    let matches = match getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(e) => {
            crash!(1, "error: {}", e);
        }
    };

    let progname = args[0].clone();
    let usage = usage("Base64 encode or decode FILE, or standard input, to standard output.", &opts);
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
    let input = if matches.free.is_empty() || matches.free[0].as_slice() == "-" {
        stdin_buf = stdin_raw();
        &mut stdin_buf as &mut Reader
    } else {
        let path = Path::new(matches.free[0].as_slice());
        file_buf = File::open(&path);
        &mut file_buf as &mut Reader
    };

    match mode {
        Mode::Decode  => decode(input, ignore_garbage),
        Mode::Encode  => encode(input, line_wrap),
        Mode::Help    => help(progname.as_slice(), usage.as_slice()),
        Mode::Version => version()
    }

    0
}

fn decode(input: &mut Reader, ignore_garbage: bool) {
    let mut to_decode = match input.read_to_string() {
        Ok(m) => m,
        Err(f) => panic!(f)
    };

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

    match to_decode.as_slice().from_base64() {
        Ok(bytes) => {
            let mut out = stdout();

            match out.write_all(bytes.as_slice()) {
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

fn encode(input: &mut Reader, line_wrap: usize) {
    let b64_conf = base64::Config {
        char_set: base64::Standard,
        newline: base64::Newline::LF,
        pad: true,
        line_length: match line_wrap {
            0 => None,
            _ => Some(line_wrap)
        }
    };
    let to_encode = match input.read_to_end() {
        Ok(m) => m,
        Err(err) => crash!(1, "{}", err)
    };
    let encoded = to_encode.as_slice().to_base64(b64_conf);

    println(encoded.as_slice());
}

fn help(progname: &str, usage: &str) {
    println!("Usage: {} [OPTION]... [FILE]", progname);
    println!("");
    println(usage);

    let msg = "With no FILE, or when FILE is -, read standard input.\n\n\
        The data are encoded as described for the base64 alphabet in RFC \
        3548. When\ndecoding, the input may contain newlines in addition \
        to the bytes of the formal\nbase64 alphabet. Use --ignore-garbage \
        to attempt to recover from any other\nnon-alphabet bytes in the \
        encoded stream.";

    println(msg);
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
