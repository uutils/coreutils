// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

#[macro_use]
extern crate uucore;
use uucore::encoding::Format;
use uucore::InvalidEncodingHandling;

use std::fs::File;
use std::io::{stdin, BufReader};
use std::path::Path;

use clap::{App, Arg};

mod base_common;

static SUMMARY: &str = "Base32 encode or decode FILE, or standard input, to standard output.";
static LONG_HELP: &str = "
 With no FILE, or when FILE is -, read standard input.

 The data are encoded as described for the base32 alphabet in RFC
 4648. When decoding, the input may contain newlines in addition
 to the bytes of the formal base32 alphabet. Use --ignore-garbage
 to attempt to recover from any other non-alphabet bytes in the
 encoded stream.
";
static VERSION: &str = env!("CARGO_PKG_VERSION");

fn get_usage() -> String {
    format!("{0} [OPTION]... [FILE]", executable!())
}

pub mod options {
    pub static DECODE: &str = "decode";
    pub static WRAP: &str = "wrap";
    pub static IGNORE_GARBAGE: &str = "ignore-garbage";
    pub static FILE: &str = "file";
}

struct Config {
    decode: bool,
    ignore_garbage: bool,
    wrap_cols: Option<usize>,
    to_read: Option<String>,
}

impl Config {
    fn from(options: clap::ArgMatches) -> Config {
        let file: Option<String> = match options.values_of(options::FILE) {
            Some(mut values) => {
                let name = values.next().unwrap();
                if values.len() != 0 {
                    crash!(3, "extra operand ‘{}’", name);
                }

                if name == "-" {
                    None
                } else {
                    if !Path::exists(Path::new(name)) {
                        crash!(2, "{}: No such file or directory", name);
                    }
                    Some(name.to_owned())
                }
            }
            None => None,
        };

        let cols = match options.value_of(options::WRAP) {
            Some(num) => match num.parse::<usize>() {
                Ok(n) => Some(n),
                Err(e) => {
                    crash!(1, "invalid wrap size: ‘{}’: {}", num, e);
                }
            },
            None => None,
        };

        Config {
            decode: options.is_present(options::DECODE),
            ignore_garbage: options.is_present(options::IGNORE_GARBAGE),
            wrap_cols: cols,
            to_read: file,
        }
    }
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    execute(
        args.collect_str(InvalidEncodingHandling::ConvertLossy)
            .accept_any(),
        SUMMARY,
        LONG_HELP,
        &get_usage(),
        Format::Base32,
    )
}

fn execute(args: Vec<String>, _summary: &str, long_help: &str, usage: &str, format: Format) -> i32 {
    let app = App::new(executable!())
        .version(VERSION)
        .usage(usage)
        .about(long_help)
        // Format arguments.
        .arg(
            Arg::with_name(options::DECODE)
                .short("d")
                .long(options::DECODE)
                .help("decode data"),
        )
        .arg(
            Arg::with_name(options::IGNORE_GARBAGE)
                .short("i")
                .long(options::IGNORE_GARBAGE)
                .help("when decoding, ignore non-alphabetic characters"),
        )
        .arg(
            Arg::with_name(options::WRAP)
                .short("w")
                .long(options::WRAP)
                .takes_value(true)
                .help(
                    "wrap encoded lines after COLS character (default 76, 0 to disable wrapping)",
                ),
        )
        // "multiple" arguments are used to check whether there is more than one
        // file passed in.
        .arg(Arg::with_name(options::FILE).index(1).multiple(true));

    let config: Config = Config::from(app.get_matches_from(args));
    match config.to_read {
        // Read from file.
        Some(name) => {
            let file_buf = safe_unwrap!(File::open(Path::new(&name)));
            let mut input = BufReader::new(file_buf);
            base_common::handle_input(
                &mut input,
                format,
                config.wrap_cols,
                config.ignore_garbage,
                config.decode,
            );
        }
        // stdin
        None => {
            base_common::handle_input(
                &mut stdin().lock(),
                format,
                config.wrap_cols,
                config.ignore_garbage,
                config.decode,
            );
        }
    };

    0
}
