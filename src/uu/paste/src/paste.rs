// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) delim

use clap::{crate_version, Arg, ArgAction, Command};
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, Read, Write};
use std::iter::Cycle;
use std::path::Path;
use std::slice::Iter;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::line_ending::LineEnding;
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("paste.md");
const USAGE: &str = help_usage!("paste.md");

mod options {
    pub const DELIMITER: &str = "delimiters";
    pub const SERIAL: &str = "serial";
    pub const FILE: &str = "file";
    pub const ZERO_TERMINATED: &str = "zero-terminated";
}

// Wraps BufReader and stdin
fn read_until<R: Read>(
    reader: Option<&mut BufReader<R>>,
    byte: u8,
    buf: &mut Vec<u8>,
) -> std::io::Result<usize> {
    match reader {
        Some(reader) => reader.read_until(byte, buf),
        None => stdin().lock().read_until(byte, buf),
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let serial = matches.get_flag(options::SERIAL);
    let delimiters = matches.get_one::<String>(options::DELIMITER).unwrap();
    let files = matches
        .get_many::<String>(options::FILE)
        .unwrap()
        .cloned()
        .collect();
    let line_ending = LineEnding::from_zero_flag(matches.get_flag(options::ZERO_TERMINATED));

    paste(files, serial, delimiters, line_ending)
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::SERIAL)
                .long(options::SERIAL)
                .short('s')
                .help("paste one file at a time instead of in parallel")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DELIMITER)
                .long(options::DELIMITER)
                .short('d')
                .help("reuse characters from LIST instead of TABs")
                .value_name("LIST")
                .default_value("\t")
                .hide_default_value(true),
        )
        .arg(
            Arg::new(options::FILE)
                .value_name("FILE")
                .action(ArgAction::Append)
                .default_value("-")
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::ZERO_TERMINATED)
                .long(options::ZERO_TERMINATED)
                .short('z')
                .help("line delimiter is NUL, not newline")
                .action(ArgAction::SetTrue),
        )
}

#[allow(clippy::cognitive_complexity)]
fn paste(
    filenames: Vec<String>,
    serial: bool,
    delimiters: &str,
    line_ending: LineEnding,
) -> UResult<()> {
    let mut files = Vec::with_capacity(filenames.len());
    for name in filenames {
        let file = if name == "-" {
            None
        } else {
            let path = Path::new(&name);
            // TODO
            // Is `map_err_context` correct here?
            let file = File::open(path).map_err_context(String::new)?;
            Some(BufReader::new(file))
        };
        files.push(file);
    }

    if delimiters.ends_with('\\') && !delimiters.ends_with("\\\\") {
        return Err(USimpleError::new(
            1,
            format!("delimiter list ends with an unescaped backslash: {delimiters}"),
        ));
    }

    let unescaped_and_encoded_delimiters = parse_delimiters(delimiters);

    let mut delimiter_state = match unescaped_and_encoded_delimiters.as_ref() {
        [] => DelimiterState::NoDelimiters,
        [only_delimiter] => DelimiterState::OneDelimiter {
            delimiter: only_delimiter,
        },
        [first_delimiter, ..] => DelimiterState::MultipleDelimiters {
            current_delimiter: first_delimiter,
            delimiters: &unescaped_and_encoded_delimiters,
            delimiters_iter: unescaped_and_encoded_delimiters.iter().cycle(),
        },
    };

    let mut stdout = stdout().lock();

    let mut output = Vec::new();

    if serial {
        for file in &mut files {
            output.clear();

            loop {
                delimiter_state.advance_to_next_delimiter();

                match read_until(file.as_mut(), line_ending as u8, &mut output) {
                    Ok(0) => break,
                    Ok(_) => {
                        if output.ends_with(&[line_ending as u8]) {
                            output.pop();
                        }

                        delimiter_state.write_delimiter(&mut output);
                    }
                    // TODO
                    // Is `map_err_context` correct here?
                    Err(e) => return Err(e.map_err_context(String::new)),
                }
            }

            delimiter_state.remove_trailing_delimiter(&mut output);

            // TODO
            // Should the output be converted to UTF-8?
            write!(
                stdout,
                "{}{}",
                String::from_utf8_lossy(&output),
                line_ending
            )?;
        }
    } else {
        let mut eof = vec![false; files.len()];

        loop {
            output.clear();

            let mut eof_count = 0;

            for (i, file) in files.iter_mut().enumerate() {
                delimiter_state.advance_to_next_delimiter();

                if eof[i] {
                    eof_count += 1;
                } else {
                    match read_until(file.as_mut(), line_ending as u8, &mut output) {
                        Ok(0) => {
                            eof[i] = true;
                            eof_count += 1;
                        }
                        Ok(_) => {
                            if output.ends_with(&[line_ending as u8]) {
                                output.pop();
                            }
                        }
                        // TODO
                        // Is `map_err_context` correct here?
                        Err(e) => return Err(e.map_err_context(String::new)),
                    }
                }

                delimiter_state.write_delimiter(&mut output);
            }

            if files.len() == eof_count {
                break;
            }

            // Quote:
            //     When the -s option is not specified:
            //     [...]
            //     The delimiter shall be reset to the first element of list after each file operand is processed.
            // https://pubs.opengroup.org/onlinepubs/9799919799/utilities/paste.html
            delimiter_state.reset_to_first_delimiter();

            delimiter_state.remove_trailing_delimiter(&mut output);

            // TODO
            // Should the output be converted to UTF-8?
            write!(
                stdout,
                "{}{}",
                String::from_utf8_lossy(&output),
                line_ending
            )?;
        }
    }

    Ok(())
}

/// Unescape all special characters
fn unescape(s: &str) -> String {
    s.replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\\\", "\\")
}

fn parse_delimiters(delimiters: &str) -> Box<[Box<[u8]>]> {
    let delimiters_unescaped = unescape(delimiters).chars().collect::<Vec<_>>();

    let delimiters_unescaped_len = delimiters_unescaped.len();

    if delimiters_unescaped_len > 0 {
        let mut vec = Vec::<Box<[u8]>>::with_capacity(delimiters_unescaped_len);

        // a buffer of length four is large enough to encode any char
        let mut buffer = [0; 4];

        for delimiter in delimiters_unescaped {
            let delimiter_encoded = delimiter.encode_utf8(&mut buffer);

            vec.push(Box::from(delimiter_encoded.as_bytes()));
        }

        vec.into_boxed_slice()
    } else {
        Box::new([])
    }
}

enum DelimiterState<'a> {
    NoDelimiters,
    OneDelimiter {
        delimiter: &'a [u8],
    },
    MultipleDelimiters {
        current_delimiter: &'a [u8],
        delimiters: &'a [Box<[u8]>],
        delimiters_iter: Cycle<Iter<'a, Box<[u8]>>>,
    },
}

impl<'a> DelimiterState<'a> {
    /// If there are multiple delimiters, advance the iterator over the delimiter list.
    /// This is a no-op unless there are multiple delimiters.
    fn advance_to_next_delimiter(&mut self) {
        if let DelimiterState::MultipleDelimiters {
            current_delimiter,
            delimiters_iter,
            ..
        } = self
        {
            // Unwrap because "delimiters_encoded_iter" is a cycle iter and was created from a non-empty slice
            *current_delimiter = delimiters_iter.next().unwrap();
        }
    }

    /// This should only be used to return to the start of the delimiter list after a file has been processed.
    /// This should only be used when the "serial" option is disabled.
    /// This is a no-op unless there are multiple delimiters.
    fn reset_to_first_delimiter(&mut self) {
        if let DelimiterState::MultipleDelimiters {
            delimiters_iter,
            delimiters,
            ..
        } = self
        {
            *delimiters_iter = delimiters.iter().cycle();
        }
    }

    /// Remove the trailing delimiter.
    /// If there are no delimiters, this is a no-op.
    fn remove_trailing_delimiter(&mut self, output: &mut Vec<u8>) {
        let delimiter_length = match self {
            DelimiterState::OneDelimiter { delimiter } => delimiter.len(),
            DelimiterState::MultipleDelimiters {
                current_delimiter, ..
            } => current_delimiter.len(),
            _ => {
                return;
            }
        };

        let output_len = output.len();

        if let Some(output_without_delimiter_length) = output_len.checked_sub(delimiter_length) {
            output.truncate(output_without_delimiter_length);
        } else {
            // This branch is NOT unreachable, must be skipped
            // "output" should be empty in this case
            assert!(output_len == 0);
        }
    }

    /// Append the current delimiter to `output`.
    /// If there are no delimiters, this is a no-op.
    fn write_delimiter(&mut self, output: &mut Vec<u8>) {
        match self {
            DelimiterState::OneDelimiter { delimiter } => {
                output.extend_from_slice(delimiter);
            }
            DelimiterState::MultipleDelimiters {
                current_delimiter, ..
            } => {
                output.extend_from_slice(current_delimiter);
            }
            _ => {}
        }
    }
}
