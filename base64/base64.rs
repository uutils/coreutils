#![crate_name = "base64"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordy Dickinson <jordy.dickinson@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

#![feature(phase)]
#![feature(macro_rules)]

extern crate serialize;
extern crate getopts;
extern crate libc;
#[phase(plugin, link)] extern crate log;

use std::io::{println, File, stdout};
use std::io::stdio::stdin_raw;

use getopts::{
    getopts,
    optflag,
    optopt,
    usage
};
use serialize::base64;
use serialize::base64::{FromBase64, ToBase64};

#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "base64";

pub fn uumain(args: Vec<String>) -> int {
    let opts = [
        optflag("d", "decode", "decode data"),
        optflag("i", "ignore-garbage", "when decoding, ignore non-alphabetic characters"),
        optopt("w", "wrap",
            "wrap encoded lines after COLS character (default 76, 0 to disable wrapping)", "COLS"
        ),
        optflag("h", "help", "display this help text and exit"),
        optflag("V", "version", "output version information and exit")
    ];
    let matches = match getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(e) => {
            error!("error: {}", e);
            fail!()
        }
    };

    let progname = args.get(0).clone();
    let usage = usage("Base64 encode or decode FILE, or standard input, to standard output.", opts);
    let mode = if matches.opt_present("help") {
        Help
    } else if matches.opt_present("version") {
        Version
    } else if matches.opt_present("decode") {
        Decode
    } else {
        Encode
    };
    let ignore_garbage = matches.opt_present("ignore-garbage");
    let line_wrap = match matches.opt_str("wrap") {
        Some(s) => match from_str(s.as_slice()) {
            Some(s) => s,
            None => {
                error!("error: {:s}", "Argument to option 'wrap' improperly formatted.");
                fail!()
            }
        },
        None => 76
    };
    let mut stdin_buf;
    let mut file_buf;
    let input = if matches.free.is_empty() || matches.free.get(0).as_slice() == "-" {
        stdin_buf = stdin_raw();
        &mut stdin_buf as &mut Reader
    } else {
        let path = Path::new(matches.free.get(0).as_slice());
        file_buf = File::open(&path);
        &mut file_buf as &mut Reader
    };

    match mode {
        Decode  => decode(input, ignore_garbage),
        Encode  => encode(input, line_wrap),
        Help    => help(progname.as_slice(), usage.as_slice()),
        Version => version()
    }

    0
}

fn decode(input: &mut Reader, ignore_garbage: bool) {
    let mut to_decode = match input.read_to_string() {
        Ok(m) => m,
        Err(f) => fail!(f)
    };

    let slice =
        if ignore_garbage {
            to_decode.as_slice()
                .trim_chars(|c: char| {
                    let num = match c.to_ascii_opt() {
                        Some(ascii) => ascii.to_byte(),
                        None => return false
                    };
                    !(num >= 'a' as u8 && num <= 'z' as u8 ||
                      num >= 'A' as u8 && num <= 'Z' as u8 ||
                      num >= '0' as u8 && num <= '9' as u8 ||
                      num == '+' as u8 || num == '/' as u8)
                })
        } else {
            to_decode = to_decode.as_slice().replace("\n", "");
            to_decode.as_slice()
        };

    match slice.from_base64() {
        Ok(bytes) => {
            let mut out = stdout();

            match out.write(bytes.as_slice()) {
                Ok(_) => {}
                Err(f) => { crash!(1, "{}", f); }
            }
            match out.flush() {
                Ok(_) => {}
                Err(f) => { crash!(1, "{}", f); }
            }
        }
        Err(s) => {
            error!("error: {}", s);
            fail!()
        }
    }
}

fn encode(input: &mut Reader, line_wrap: uint) {
    let b64_conf = base64::Config {
        char_set: base64::Standard,
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

    // To my knowledge, RFC 3548 does not specify which line endings to use. It
    // seems that rust's base64 algorithm uses CRLF as prescribed by RFC 2045.
    // However, since GNU base64 outputs only LF (presumably because that is
    // the standard UNIX line ending), we strip CRs from the output to maintain
    // compatibility.
    let final = encoded.replace("\r", "");

    println(final.as_slice());
}

fn help(progname: &str, usage: &str) {
    println!("Usage: {:s} [OPTION]... [FILE]", progname);
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
