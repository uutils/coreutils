// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::io::{stdout, Read, Write};

use uucore::display::Quotable;
use uucore::encoding::{wrap_print, Data, EncodeError, Format};
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};
use uucore::format_usage;

use std::fs::File;
use std::io::{BufReader, Stdin};
use std::path::Path;

use clap::{crate_version, Arg, ArgAction, Command};

pub static BASE_CMD_PARSE_ERROR: i32 = 1;

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
    pub fn from(options: &clap::ArgMatches) -> UResult<Self> {
        let file: Option<String> = match options.get_many::<String>(options::FILE) {
            Some(mut values) => {
                let name = values.next().unwrap();
                if let Some(extra_op) = values.next() {
                    return Err(UUsageError::new(
                        BASE_CMD_PARSE_ERROR,
                        format!("extra operand {}", extra_op.quote(),),
                    ));
                }

                if name == "-" {
                    None
                } else {
                    if !Path::exists(Path::new(name)) {
                        return Err(USimpleError::new(
                            BASE_CMD_PARSE_ERROR,
                            format!("{}: No such file or directory", name.maybe_quote()),
                        ));
                    }
                    Some(name.clone())
                }
            }
            None => None,
        };

        let cols = options
            .get_one::<String>(options::WRAP)
            .map(|num| {
                num.parse::<usize>().map_err(|_| {
                    USimpleError::new(
                        BASE_CMD_PARSE_ERROR,
                        format!("invalid wrap size: {}", num.quote()),
                    )
                })
            })
            .transpose()?;

        Ok(Self {
            decode: options.get_flag(options::DECODE),
            ignore_garbage: options.get_flag(options::IGNORE_GARBAGE),
            wrap_cols: cols,
            to_read: file,
        })
    }
}

pub fn parse_base_cmd_args(
    args: impl uucore::Args,
    about: &'static str,
    usage: &str,
) -> UResult<Config> {
    let command = base_app(about, usage);
    Config::from(&command.try_get_matches_from(args)?)
}

pub fn base_app(about: &'static str, usage: &str) -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(about)
        .override_usage(format_usage(usage))
        .infer_long_args(true)
        // Format arguments.
        .arg(
            Arg::new(options::DECODE)
                .short('d')
                .long(options::DECODE)
                .help("decode data")
                .action(ArgAction::SetTrue)
                .overrides_with(options::DECODE),
        )
        .arg(
            Arg::new(options::IGNORE_GARBAGE)
                .short('i')
                .long(options::IGNORE_GARBAGE)
                .help("when decoding, ignore non-alphabetic characters")
                .action(ArgAction::SetTrue)
                .overrides_with(options::IGNORE_GARBAGE),
        )
        .arg(
            Arg::new(options::WRAP)
                .short('w')
                .long(options::WRAP)
                .value_name("COLS")
                .help("wrap encoded lines after COLS character (default 76, 0 to disable wrapping)")
                .overrides_with(options::WRAP),
        )
        // "multiple" arguments are used to check whether there is more than one
        // file passed in.
        .arg(
            Arg::new(options::FILE)
                .index(1)
                .action(clap::ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
}

pub fn get_input<'a>(config: &Config, stdin_ref: &'a Stdin) -> UResult<Box<dyn Read + 'a>> {
    match &config.to_read {
        Some(name) => {
            let file_buf =
                File::open(Path::new(name)).map_err_context(|| name.maybe_quote().to_string())?;
            Ok(Box::new(BufReader::new(file_buf))) // as Box<dyn Read>
        }
        None => {
            Ok(Box::new(stdin_ref.lock())) // as Box<dyn Read>
        }
    }
}

pub fn handle_input<R: Read>(
    input: &mut R,
    format: Format,
    line_wrap: Option<usize>,
    ignore_garbage: bool,
    decode: bool,
) -> UResult<()> {
    let mut data = Data::new(input, format).ignore_garbage(ignore_garbage);
    if let Some(wrap) = line_wrap {
        data = data.line_wrap(wrap);
    }

    if decode {
        match data.decode() {
            Ok(s) => {
                // Silent the warning as we want to the error message
                #[allow(clippy::question_mark)]
                if stdout().write_all(&s).is_err() {
                    // on windows console, writing invalid utf8 returns an error
                    return Err(USimpleError::new(1, "error: cannot write non-utf8 data"));
                }
                Ok(())
            }
            Err(_) => Err(USimpleError::new(1, "error: invalid input")),
        }
    } else {
        match data.encode() {
            Ok(s) => {
                wrap_print(&data, &s);
                Ok(())
            }
            Err(EncodeError::InvalidInput) => Err(USimpleError::new(1, "error: invalid input")),
            Err(_) => Err(USimpleError::new(
                1,
                "error: invalid input (length must be multiple of 4 characters)",
            )),
        }
    }
}
