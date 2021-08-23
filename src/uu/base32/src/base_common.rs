// This file is part of the uutils coreutils package.
//
// (c) Jordy Dickinson <jordy.dickinson@gmail.com>
// (c) Jian Zeng <anonymousknight96@gmail.com>
// (c) Alex Lyon <arcterus@mail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

use std::io::{stdout, Read, Write};

use uucore::encoding::{wrap_print, Data, Format};
use uucore::InvalidEncodingHandling;

use std::fs::File;
use std::io::{BufReader, Stdin};
use std::path::Path;

use clap::{App, Arg};

// Config.
pub struct Config {
    pub decode: bool,
    pub ignore_garbage: bool,
    pub wrap_cols: Option<usize>,
    pub to_read: Option<String>,
}

pub mod options {
    pub static DECODE: &str = "decode";
    pub static WRAP: &str = "wrap";
    pub static IGNORE_GARBAGE: &str = "ignore-garbage";
    pub static FILE: &str = "file";
}

impl Config {
    pub fn from(app_name: &str, options: &clap::ArgMatches) -> Result<Config, String> {
        let file: Option<String> = match options.values_of(options::FILE) {
            Some(mut values) => {
                let name = values.next().unwrap();
                if let Some(extra_op) = values.next() {
                    return Err(format!(
                        "extra operand '{}'\nTry '{} --help' for more information.",
                        extra_op, app_name
                    ));
                }

                if name == "-" {
                    None
                } else {
                    if !Path::exists(Path::new(name)) {
                        return Err(format!("{}: No such file or directory", name));
                    }
                    Some(name.to_owned())
                }
            }
            None => None,
        };

        let cols = options
            .value_of(options::WRAP)
            .map(|num| {
                num.parse::<usize>()
                    .map_err(|_| format!("invalid wrap size: '{}'", num))
            })
            .transpose()?;

        Ok(Config {
            decode: options.is_present(options::DECODE),
            ignore_garbage: options.is_present(options::IGNORE_GARBAGE),
            wrap_cols: cols,
            to_read: file,
        })
    }
}

pub fn parse_base_cmd_args(
    args: impl uucore::Args,
    name: &str,
    version: &str,
    about: &str,
    usage: &str,
) -> Result<Config, String> {
    let app = base_app(name, version, about).usage(usage);
    let arg_list = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();
    Config::from(name, &app.get_matches_from(arg_list))
}

pub fn base_app<'a>(name: &str, version: &'a str, about: &'a str) -> App<'static, 'a> {
    App::new(name)
        .version(version)
        .about(about)
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
        .arg(Arg::with_name(options::FILE).index(1).multiple(true))
}

pub fn get_input<'a>(config: &Config, stdin_ref: &'a Stdin) -> Box<dyn Read + 'a> {
    match &config.to_read {
        Some(name) => {
            let file_buf = crash_if_err!(1, File::open(Path::new(name)));
            Box::new(BufReader::new(file_buf)) // as Box<dyn Read>
        }
        None => {
            Box::new(stdin_ref.lock()) // as Box<dyn Read>
        }
    }
}

pub fn handle_input<R: Read>(
    input: &mut R,
    format: Format,
    line_wrap: Option<usize>,
    ignore_garbage: bool,
    decode: bool,
    name: &str,
) {
    let mut data = Data::new(input, format).ignore_garbage(ignore_garbage);
    if let Some(wrap) = line_wrap {
        data = data.line_wrap(wrap);
    }

    if !decode {
        match data.encode() {
            Ok(s) => {
                wrap_print(&data, s);
            }
            Err(_) => {
                eprintln!(
                    "{}: error: invalid input (length must be multiple of 4 characters)",
                    name
                );
                exit!(1)
            }
        }
    } else {
        match data.decode() {
            Ok(s) => {
                if stdout().write_all(&s).is_err() {
                    // on windows console, writing invalid utf8 returns an error
                    eprintln!("{}: error: Cannot write non-utf8 data", name);
                    exit!(1)
                }
            }
            Err(_) => {
                eprintln!("{}: error: invalid input", name);
                exit!(1)
            }
        }
    }
}
