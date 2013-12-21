#[link(name="base64", vers="1.0.0", author="Jordy Dickinson")]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordy Dickinson <jordy.dickinson@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern mod extra;

use std::char;
use std::io::{File, stdin, stdout};
use std::os;
use std::str;

use extra::base64;
use extra::base64::{FromBase64, ToBase64};

fn main() {
    let mut conf = Conf::new(os::args());

    match conf.mode {
        Decode  => decode(&mut conf),
        Encode  => encode(&mut conf),
        Help    => help(&conf),
        Version => version()
    }
}

fn decode(conf: &mut Conf) {
    let mut to_decode = str::from_utf8_owned(conf.input_file.read_to_end());

    to_decode = to_decode.replace("\n", "");

    if conf.ignore_garbage {
        let standard_chars =
            bytes!("ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            "abcdefghijklmnopqrstuvwxyz",
            "0123456789+/").map(|b| char::from_u32(*b as u32).unwrap());

        to_decode = to_decode
            .trim_chars(&|c| !standard_chars.contains(&c))
            .to_owned();
    }

    match to_decode.from_base64() {
        Ok(bytes) => {
            let mut out = stdout();

            out.write(bytes);
            out.flush();
        }
        Err(s) => {
            error!("error: {:s}", s);
            fail!()
        }
    }
}

fn encode(conf: &mut Conf) {
    let b64_conf = base64::Config {
        char_set: base64::Standard,
        pad: true,
        line_length: match conf.line_wrap {
            0 => None,
            _ => Some(conf.line_wrap)
        }
    };
    let to_encode = conf.input_file.read_to_end();
    let mut encoded = to_encode.to_base64(b64_conf);

    // To my knowledge, RFC 3548 does not specify which line endings to use. It
    // seems that rust's base64 algorithm uses CRLF as prescribed by RFC 2045.
    // However, since GNU base64 outputs only LF (presumably because that is
    // the standard UNIX line ending), we strip CRs from the output to maintain
    // compatibility.
    encoded = encoded.replace("\r", "");

    println(encoded);
}

fn help(conf: &Conf) {
    println!("Usage: {:s} [OPTION]... [FILE]", conf.progname);
    println("");
    println(conf.usage);

    let msg = ~"With no FILE, or when FILE is -, read standard input.\n\n\
        The data are encoded as described for the base64 alphabet in RFC \
        3548. When\ndecoding, the input may contain newlines in addition \
        to the bytes of the formal\nbase64 alphabet. Use --ignore-garbage \
        to attempt to recover from any other\nnon-alphabet bytes in the \
        encoded stream.";

    println(msg);
}

fn version() {
    println("base64 1.0.0");
}

struct Conf {
    progname: ~str,
    usage: ~str,
    mode: Mode,
    ignore_garbage: bool,
    line_wrap: uint,
    input_file: ~Reader
}

impl Conf {
    fn new(args: &[~str]) -> Conf {
        // The use statement is here rather than at the top of the file so that
        // the reader is made directly aware that we're using getopts::groups,
        // and not just getopts. Also some names are somewhat vague taken out
        // of context (i.e., "usage").
        use extra::getopts::groups::{
            getopts,
            optflag,
            optopt,
            usage
        };

        let opts = ~[
            optflag("d", "decode", "decode data"),
            optflag("i", "ignore-garbage",
            "when decoding, ignore non-alphabetic characters"),
            optopt("w", "wrap",
            "wrap encoded lines after COLS character (default 76, 0 to \
            disable wrapping)", "COLS"),
            optflag("h", "help", "display this help text and exit"),
            optflag("V", "version", "output version information and exit")
                ];
        let matches = match getopts(args.tail(), opts) {
            Ok(m) => m,
            Err(e) => {
                error!("error: {:s}", e.to_err_msg());
                fail!()
            }
        };

        Conf {
            progname: args[0].clone(),
            usage: usage("Base64 encode or decode FILE, or standard input, to \
                         standard output.", opts),
            mode: if matches.opt_present("help") {
                Help
            } else if matches.opt_present("version") {
                Version
            } else if matches.opt_present("decode") {
                Decode
            } else {
                Encode
            },
            ignore_garbage: matches.opt_present("ignore-garbage"),
            line_wrap: match matches.opt_str("wrap") {
                Some(s) => match from_str(s) {
                    Some(s) => s,
                    None => {
                        error!("error: {:s}", "Argument to option 'wrap' \
                               improperly formatted.");
                        fail!()
                    }
                },
                None => 76
            },
            input_file: if matches.free.is_empty() 
                            || matches.free[0] == ~"-" {
                ~stdin() as ~Reader
            } else {
                let path = Path::new(matches.free[0]);
                ~File::open(&path) as ~Reader
            }
        }
    }
}

enum Mode {
    Decode,
    Encode,
    Help,
    Version
}

