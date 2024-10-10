// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use std::cell::{OnceCell, RefCell};
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, Stdin, Write};
use std::iter::Cycle;
use std::rc::Rc;
use std::slice::Iter;
use uucore::error::{UResult, USimpleError};
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
    let unescaped_and_encoded_delimiters = parse_delimiters(delimiters)?;

    let stdin_once_cell = OnceCell::<Rc<RefCell<Stdin>>>::new();

    let mut input_source_vec = Vec::with_capacity(filenames.len());

    for filename in filenames {
        let input_source = match filename.as_str() {
            "-" => InputSource::StandardInput(
                stdin_once_cell
                    .get_or_init(|| Rc::new(RefCell::new(stdin())))
                    .clone(),
            ),
            st => {
                let file = File::open(st)?;

                InputSource::File(BufReader::new(file))
            }
        };

        input_source_vec.push(input_source);
    }

    let mut stdout = stdout().lock();

    let line_ending_byte = u8::from(line_ending);
    let line_ending_byte_array_ref = &[line_ending_byte];

    let input_source_vec_len = input_source_vec.len();

    let mut delimiter_state = DelimiterState::new(&unescaped_and_encoded_delimiters);

    let mut output = Vec::new();

    if serial {
        for input_source in &mut input_source_vec {
            output.clear();

            loop {
                match input_source.read_until(line_ending_byte, &mut output)? {
                    0 => break,
                    _ => {
                        remove_trailing_line_ending_byte(line_ending_byte, &mut output);

                        delimiter_state.write_delimiter(&mut output);
                    }
                }
            }

            delimiter_state.remove_trailing_delimiter(&mut output);

            stdout.write_all(&output)?;
            stdout.write_all(line_ending_byte_array_ref)?;
        }
    } else {
        let mut eof = vec![false; input_source_vec_len];

        loop {
            output.clear();

            let mut eof_count = 0;

            for (i, input_source) in input_source_vec.iter_mut().enumerate() {
                if eof[i] {
                    eof_count += 1;
                } else {
                    match input_source.read_until(line_ending_byte, &mut output)? {
                        0 => {
                            eof[i] = true;
                            eof_count += 1;
                        }
                        _ => {
                            remove_trailing_line_ending_byte(line_ending_byte, &mut output);
                        }
                    }
                }

                delimiter_state.write_delimiter(&mut output);
            }

            if eof_count == input_source_vec_len {
                break;
            }

            delimiter_state.remove_trailing_delimiter(&mut output);

            stdout.write_all(&output)?;
            stdout.write_all(line_ending_byte_array_ref)?;

            // Quote:
            //     When the -s option is not specified:
            //     [...]
            //     The delimiter shall be reset to the first element of list after each file operand is processed.
            // https://pubs.opengroup.org/onlinepubs/9799919799/utilities/paste.html
            delimiter_state.reset_to_first_delimiter();
        }
    }

    Ok(())
}

fn parse_delimiters(delimiters: &str) -> UResult<Box<[Box<[u8]>]>> {
    /// A single backslash char
    const BACKSLASH: char = '\\';

    fn add_one_byte_single_char_delimiter(vec: &mut Vec<Box<[u8]>>, byte: u8) {
        vec.push(Box::new([byte]));
    }

    // a buffer of length four is large enough to encode any char
    let mut buffer = [0; 4];

    let mut add_single_char_delimiter = |vec: &mut Vec<Box<[u8]>>, ch: char| {
        let delimiter_encoded = ch.encode_utf8(&mut buffer);

        vec.push(Box::from(delimiter_encoded.as_bytes()));
    };

    let mut vec = Vec::<Box<[u8]>>::with_capacity(delimiters.len());

    let mut chars = delimiters.chars();

    // Unescape all special characters
    while let Some(char) = chars.next() {
        match char {
            BACKSLASH => match chars.next() {
                // "Empty string (not a null character)"
                // https://pubs.opengroup.org/onlinepubs/9799919799/utilities/paste.html
                Some('0') => {
                    vec.push(Box::<[u8; 0]>::new([]));
                }
                // "\\" to "\" (U+005C)
                Some(BACKSLASH) => {
                    add_one_byte_single_char_delimiter(&mut vec, b'\\');
                }
                // "\n" to U+000A
                Some('n') => {
                    add_one_byte_single_char_delimiter(&mut vec, b'\n');
                }
                // "\t" to U+0009
                Some('t') => {
                    add_one_byte_single_char_delimiter(&mut vec, b'\t');
                }
                Some(other_char) => {
                    // "If any other characters follow the <backslash>, the results are unspecified."
                    // https://pubs.opengroup.org/onlinepubs/9799919799/utilities/paste.html
                    // However, other implementations remove the backslash
                    // See "test_posix_unspecified_delimiter"
                    add_single_char_delimiter(&mut vec, other_char);
                }
                None => {
                    return Err(USimpleError::new(
                        1,
                        format!("delimiter list ends with an unescaped backslash: {delimiters}"),
                    ));
                }
            },
            non_backslash_char => {
                add_single_char_delimiter(&mut vec, non_backslash_char);
            }
        }
    }

    Ok(vec.into_boxed_slice())
}

fn remove_trailing_line_ending_byte(line_ending_byte: u8, output: &mut Vec<u8>) {
    if let Some(&byte) = output.last() {
        if byte == line_ending_byte {
            assert!(output.pop() == Some(line_ending_byte));
        }
    }
}

enum DelimiterState<'a> {
    NoDelimiters,
    OneDelimiter(&'a [u8]),
    MultipleDelimiters {
        current_delimiter: &'a [u8],
        delimiters: &'a [Box<[u8]>],
        delimiters_iterator: Cycle<Iter<'a, Box<[u8]>>>,
    },
}

impl<'a> DelimiterState<'a> {
    fn new(unescaped_and_encoded_delimiters: &'a [Box<[u8]>]) -> DelimiterState<'a> {
        match unescaped_and_encoded_delimiters {
            [] => DelimiterState::NoDelimiters,
            [only_delimiter] => {
                // -d '\0' is equivalent to -d ''
                if only_delimiter.is_empty() {
                    DelimiterState::NoDelimiters
                } else {
                    DelimiterState::OneDelimiter(only_delimiter)
                }
            }
            [first_delimiter, ..] => DelimiterState::MultipleDelimiters {
                current_delimiter: first_delimiter,
                delimiters: unescaped_and_encoded_delimiters,
                delimiters_iterator: unescaped_and_encoded_delimiters.iter().cycle(),
            },
        }
    }

    /// This should only be used to return to the start of the delimiter list after a file has been processed.
    /// This should only be used when the "serial" option is disabled.
    /// This is a no-op unless there are multiple delimiters.
    fn reset_to_first_delimiter(&mut self) {
        if let DelimiterState::MultipleDelimiters {
            delimiters_iterator,
            delimiters,
            ..
        } = self
        {
            *delimiters_iterator = delimiters.iter().cycle();
        }
    }

    /// Remove the trailing delimiter.
    /// If there are no delimiters, this is a no-op.
    fn remove_trailing_delimiter(&mut self, output: &mut Vec<u8>) {
        let delimiter_length = match self {
            DelimiterState::OneDelimiter(only_delimiter) => only_delimiter.len(),
            DelimiterState::MultipleDelimiters {
                current_delimiter, ..
            } => current_delimiter.len(),
            _ => {
                return;
            }
        };

        // `delimiter_length` will be zero if the current delimiter is a "\0" delimiter
        if delimiter_length > 0 {
            let output_len = output.len();

            if let Some(output_without_delimiter_length) = output_len.checked_sub(delimiter_length)
            {
                output.truncate(output_without_delimiter_length);
            } else {
                // This branch is NOT unreachable, must be skipped
                // `output` should be empty in this case
                assert!(output_len == 0);
            }
        }
    }

    /// Append the current delimiter to `output`.
    /// If there are no delimiters, this is a no-op.
    fn write_delimiter(&mut self, output: &mut Vec<u8>) {
        match self {
            DelimiterState::OneDelimiter(only_delimiter) => {
                output.extend_from_slice(only_delimiter);
            }
            DelimiterState::MultipleDelimiters {
                current_delimiter,
                delimiters_iterator,
                ..
            } => {
                // Unwrap because `delimiters_iterator` is a cycle iter and was created from a non-empty slice
                let bo = delimiters_iterator.next().unwrap();

                output.extend_from_slice(bo);

                *current_delimiter = bo;
            }
            _ => {}
        }
    }
}

enum InputSource {
    File(BufReader<File>),
    StandardInput(Rc<RefCell<Stdin>>),
}

impl InputSource {
    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> UResult<usize> {
        let us = match self {
            Self::File(bu) => bu.read_until(byte, buf)?,
            Self::StandardInput(rc) => rc
                .try_borrow()
                .map_err(|bo| USimpleError::new(1, format!("{bo}")))?
                .lock()
                .read_until(byte, buf)?,
        };

        Ok(us)
    }
}
