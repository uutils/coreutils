// This file is part of the uutils coreutils package.
//
// (c) Jordy Dickinson <jordy.dickinson@gmail.com>
// (c) Jian Zeng <anonymousknight96@gmail.com>
// (c) Alex Lyon <arcterus@mail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

use std::fs::File;
use std::io::{stdin, stdout, BufReader, Read, Write};
use std::path::Path;

use clap::{App, Arg};
use uucore::encoding::{wrap_print, Data, Format};

static VERSION: &str = env!("CARGO_PKG_VERSION");

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
                if values.len() != 1 {
                    crash!(3, "extra operand ‘{}’", values.nth(0).unwrap())
                }
                let name = values.nth(0).unwrap();
                if !Path::exists(Path::new(name)) {
                    crash!(2, "{}: No such file or directory", name);
                }

                if name == "-" {
                    None // stdin
                } else {
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

fn get_usage() -> String {
    format!("{0} [OPTION]... [FILE]", executable!())
}

pub fn execute(args: Vec<String>, _summary: &str, long_help: &str, format: Format) -> i32 {
    let usage = get_usage();
    let app = App::new(executable!())
        .version(VERSION)
        .usage(&usage[..])
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
            handle_input(
                &mut input,
                format,
                config.wrap_cols,
                config.ignore_garbage,
                config.decode,
            );
        }
        // stdin
        None => {
            handle_input(
                &mut stdin().lock(),
                format,
                config.wrap_cols,
                config.ignore_garbage,
                config.decode,
            );
        }
    };

    // let matches = app!(syntax, summary, long_help)
    //     .optflag("d", "decode", "decode data")
    //     .optflag(
    //         "i",
    //         "ignore-garbage",
    //         "when decoding, ignore non-alphabetic characters",
    //     )
    //     .optopt(
    //         "w",
    //         "wrap",
    //         "wrap encoded lines after COLS character (default 76, 0 to disable wrapping)",
    //         "COLS",
    //     )
    //     .parse(args);

    // let line_wrap = matches.opt_str("wrap").map(|s| match s.parse() {
    //     Ok(n) => n,
    //     Err(e) => {
    //         crash!(1, "invalid wrap size: ‘{}’: {}", s, e);
    //     }
    // });
    // let ignore_garbage = matches.opt_present("ignore-garbage");
    // let decode = matches.opt_present("decode");

    // if matches.free.len() > 1 {
    //     show_usage_error!("extra operand ‘{}’", matches.free[0]);
    //     return 1;
    // }

    // if matches.free.is_empty() || &matches.free[0][..] == "-" {
    //     let stdin_raw = stdin();
    //     handle_input(
    //         &mut stdin_raw.lock(),
    //         format,
    //         line_wrap,
    //         ignore_garbage,
    //         decode,
    //     );
    // } else {
    //     let path = Path::new(matches.free[0].as_str());
    //     let file_buf = safe_unwrap!(File::open(&path));
    //     let mut input = BufReader::new(file_buf);
    //
    // };

    0
}

fn handle_input<R: Read>(
    input: &mut R,
    format: Format,
    line_wrap: Option<usize>,
    ignore_garbage: bool,
    decode: bool,
) {
    let mut data = Data::new(input, format).ignore_garbage(ignore_garbage);
    if let Some(wrap) = line_wrap {
        data = data.line_wrap(wrap);
    }

    if !decode {
        let encoded = data.encode();
        wrap_print(&data, encoded);
    } else {
        match data.decode() {
            Ok(s) => {
                if stdout().write_all(&s).is_err() {
                    // on windows console, writing invalid utf8 returns an error
                    crash!(1, "Cannot write non-utf8 data");
                }
            }
            Err(_) => crash!(1, "invalid input"),
        }
    }
}
