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

pub use crate::app::get_app;
use crate::app::options;

// Config.
pub struct Config {
    pub decode: bool,
    pub ignore_garbage: bool,
    pub wrap_cols: Option<usize>,
    pub to_read: Option<String>,
}

impl Config {
    fn from(options: clap::ArgMatches) -> Result<Config, String> {
        let file: Option<String> = match options.values_of(options::FILE) {
            Some(mut values) => {
                let name = values.next().unwrap();
                if values.len() != 0 {
                    return Err(format!("extra operand ‘{}’", name));
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
                    .map_err(|e| format!("Invalid wrap size: ‘{}’: {}", num, e))
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
    let app = get_app(name).version(version).about(about).usage(usage);
    let arg_list = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();
    Config::from(app.get_matches_from(arg_list))
}

pub fn get_input<'a>(config: &Config, stdin_ref: &'a Stdin) -> Box<dyn Read + 'a> {
    match &config.to_read {
        Some(name) => {
            let file_buf = safe_unwrap!(File::open(Path::new(name)));
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
        let encoded = data.encode();
        wrap_print(&data, encoded);
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
