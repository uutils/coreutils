// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) delim sourcefiles

use bstr::io::BufReadExt;
use clap::{crate_version, Arg, ArgAction, Command};
use std::fs::File;
use std::io::{stdin, stdout, BufReader, BufWriter, IsTerminal, Read, Write};
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{set_exit_code, FromIo, UResult, USimpleError};
use uucore::line_ending::LineEnding;

use self::searcher::Searcher;
use matcher::{ExactMatcher, Matcher, WhitespaceMatcher};
use uucore::ranges::Range;
use uucore::{format_usage, help_about, help_section, help_usage, show_error, show_if_err};

mod matcher;
mod searcher;

const USAGE: &str = help_usage!("cut.md");
const ABOUT: &str = help_about!("cut.md");
const AFTER_HELP: &str = help_section!("after help", "cut.md");

struct Options {
    out_delim: Option<String>,
    line_ending: LineEnding,
}

enum Delimiter {
    Whitespace,
    String(String), // FIXME: use char?
}

struct FieldOptions {
    delimiter: Delimiter,
    out_delimiter: Option<String>,
    only_delimited: bool,
    line_ending: LineEnding,
}

enum Mode {
    Bytes(Vec<Range>, Options),
    Characters(Vec<Range>, Options),
    Fields(Vec<Range>, FieldOptions),
}

fn stdout_writer() -> Box<dyn Write> {
    if std::io::stdout().is_terminal() {
        Box::new(stdout())
    } else {
        Box::new(BufWriter::new(stdout())) as Box<dyn Write>
    }
}

fn list_to_ranges(list: &str, complement: bool) -> Result<Vec<Range>, String> {
    if complement {
        Range::from_list(list).map(|r| uucore::ranges::complement(&r))
    } else {
        Range::from_list(list)
    }
}

fn cut_bytes<R: Read>(reader: R, ranges: &[Range], opts: &Options) -> UResult<()> {
    let newline_char = opts.line_ending.into();
    let mut buf_in = BufReader::new(reader);
    let mut out = stdout_writer();
    let delim = opts
        .out_delim
        .as_ref()
        .map_or("", String::as_str)
        .as_bytes();

    let result = buf_in.for_byte_record(newline_char, |line| {
        let mut print_delim = false;
        for &Range { low, high } in ranges {
            if low > line.len() {
                break;
            }
            if print_delim {
                out.write_all(delim)?;
            } else if opts.out_delim.is_some() {
                print_delim = true;
            }
            // change `low` from 1-indexed value to 0-index value
            let low = low - 1;
            let high = high.min(line.len());
            out.write_all(&line[low..high])?;
        }
        out.write_all(&[newline_char])?;
        Ok(true)
    });

    if let Err(e) = result {
        return Err(USimpleError::new(1, e.to_string()));
    }

    Ok(())
}

// Output delimiter is explicitly specified
fn cut_fields_explicit_out_delim<R: Read, M: Matcher>(
    reader: R,
    matcher: &M,
    ranges: &[Range],
    only_delimited: bool,
    newline_char: u8,
    out_delim: &str,
) -> UResult<()> {
    let mut buf_in = BufReader::new(reader);
    let mut out = stdout_writer();

    let result = buf_in.for_byte_record_with_terminator(newline_char, |line| {
        let mut fields_pos = 1;
        let mut low_idx = 0;
        let mut delim_search = Searcher::new(matcher, line).peekable();
        let mut print_delim = false;

        if delim_search.peek().is_none() {
            if !only_delimited {
                out.write_all(line)?;
                if line[line.len() - 1] != newline_char {
                    out.write_all(&[newline_char])?;
                }
            }

            return Ok(true);
        }

        for &Range { low, high } in ranges {
            if low - fields_pos > 0 {
                // current field is not in the range, so jump to the field corresponding to the
                // beginning of the range if any
                low_idx = match delim_search.nth(low - fields_pos - 1) {
                    Some((_, last)) => last,
                    None => break,
                };
            }

            // at this point, current field is the first in the range
            for _ in 0..=high - low {
                // skip printing delimiter if this is the first matching field for this line
                if print_delim {
                    out.write_all(out_delim.as_bytes())?;
                } else {
                    print_delim = true;
                }

                match delim_search.next() {
                    // print the current field up to the next field delim
                    Some((first, last)) => {
                        let segment = &line[low_idx..first];

                        out.write_all(segment)?;

                        low_idx = last;
                        fields_pos = high + 1;
                    }
                    None => {
                        // this is the last field in the line, so print the rest
                        let segment = &line[low_idx..];

                        out.write_all(segment)?;

                        if line[line.len() - 1] == newline_char {
                            return Ok(true);
                        }
                        break;
                    }
                }
            }
        }

        out.write_all(&[newline_char])?;
        Ok(true)
    });

    if let Err(e) = result {
        return Err(USimpleError::new(1, e.to_string()));
    }

    Ok(())
}

// Output delimiter is the same as input delimiter
fn cut_fields_implicit_out_delim<R: Read, M: Matcher>(
    reader: R,
    matcher: &M,
    ranges: &[Range],
    only_delimited: bool,
    newline_char: u8,
) -> UResult<()> {
    let mut buf_in = BufReader::new(reader);
    let mut out = stdout_writer();

    let result = buf_in.for_byte_record_with_terminator(newline_char, |line| {
        let mut fields_pos = 1;
        let mut low_idx = 0;
        let mut delim_search = Searcher::new(matcher, line).peekable();
        let mut print_delim = false;

        if delim_search.peek().is_none() {
            if !only_delimited {
                out.write_all(line)?;
                if line[line.len() - 1] != newline_char {
                    out.write_all(&[newline_char])?;
                }
            }

            return Ok(true);
        }

        for &Range { low, high } in ranges {
            if low - fields_pos > 0 {
                if let Some((first, last)) = delim_search.nth(low - fields_pos - 1) {
                    low_idx = if print_delim { first } else { last }
                } else {
                    break;
                }
            }

            match delim_search.nth(high - low) {
                Some((first, _)) => {
                    let segment = &line[low_idx..first];

                    out.write_all(segment)?;

                    print_delim = true;
                    low_idx = first;
                    fields_pos = high + 1;
                }
                None => {
                    let segment = &line[low_idx..line.len()];

                    out.write_all(segment)?;

                    if line[line.len() - 1] == newline_char {
                        return Ok(true);
                    }
                    break;
                }
            }
        }
        out.write_all(&[newline_char])?;
        Ok(true)
    });

    if let Err(e) = result {
        return Err(USimpleError::new(1, e.to_string()));
    }

    Ok(())
}

fn cut_fields<R: Read>(reader: R, ranges: &[Range], opts: &FieldOptions) -> UResult<()> {
    let newline_char = opts.line_ending.into();
    match opts.delimiter {
        Delimiter::String(ref delim) => {
            let matcher = ExactMatcher::new(delim.as_bytes());
            match opts.out_delimiter {
                Some(ref out_delim) => cut_fields_explicit_out_delim(
                    reader,
                    &matcher,
                    ranges,
                    opts.only_delimited,
                    newline_char,
                    out_delim,
                ),
                None => cut_fields_implicit_out_delim(
                    reader,
                    &matcher,
                    ranges,
                    opts.only_delimited,
                    newline_char,
                ),
            }
        }
        Delimiter::Whitespace => {
            let matcher = WhitespaceMatcher {};
            let out_delim = opts.out_delimiter.as_deref().unwrap_or("\t");
            cut_fields_explicit_out_delim(
                reader,
                &matcher,
                ranges,
                opts.only_delimited,
                newline_char,
                out_delim,
            )
        }
    }
}

fn cut_files(mut filenames: Vec<String>, mode: &Mode) {
    let mut stdin_read = false;

    if filenames.is_empty() {
        filenames.push("-".to_owned());
    }

    for filename in &filenames {
        if filename == "-" {
            if stdin_read {
                continue;
            }

            show_if_err!(match mode {
                Mode::Bytes(ref ranges, ref opts) => cut_bytes(stdin(), ranges, opts),
                Mode::Characters(ref ranges, ref opts) => cut_bytes(stdin(), ranges, opts),
                Mode::Fields(ref ranges, ref opts) => cut_fields(stdin(), ranges, opts),
            });

            stdin_read = true;
        } else {
            let path = Path::new(&filename[..]);

            if path.is_dir() {
                show_error!("{}: Is a directory", filename.maybe_quote());
                set_exit_code(1);
                continue;
            }

            show_if_err!(File::open(path)
                .map_err_context(|| filename.maybe_quote().to_string())
                .and_then(|file| {
                    match &mode {
                        Mode::Bytes(ranges, opts) | Mode::Characters(ranges, opts) => {
                            cut_bytes(file, ranges, opts)
                        }
                        Mode::Fields(ranges, opts) => cut_fields(file, ranges, opts),
                    }
                }));
        }
    }
}

mod options {
    pub const BYTES: &str = "bytes";
    pub const CHARACTERS: &str = "characters";
    pub const DELIMITER: &str = "delimiter";
    pub const FIELDS: &str = "fields";
    pub const ZERO_TERMINATED: &str = "zero-terminated";
    pub const ONLY_DELIMITED: &str = "only-delimited";
    pub const OUTPUT_DELIMITER: &str = "output-delimiter";
    pub const WHITESPACE_DELIMITED: &str = "whitespace-delimited";
    pub const COMPLEMENT: &str = "complement";
    pub const FILE: &str = "file";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args.collect_ignore();

    let delimiter_is_equal = args.contains(&"-d=".to_string()); // special case
    let matches = uu_app().try_get_matches_from(args)?;

    let complement = matches.get_flag(options::COMPLEMENT);

    // Only one, and only one of cutting mode arguments, i.e. `-b`, `-c`, `-f`,
    // is expected. The number of those arguments is used for parsing a cutting
    // mode and handling the error cases.
    let mode_args_count = [
        matches.indices_of(options::BYTES),
        matches.indices_of(options::CHARACTERS),
        matches.indices_of(options::FIELDS),
    ]
    .into_iter()
    .map(|indices| indices.unwrap_or_default().count())
    .sum();

    let mode_parse = match (
        mode_args_count,
        matches.get_one::<String>(options::BYTES),
        matches.get_one::<String>(options::CHARACTERS),
        matches.get_one::<String>(options::FIELDS),
    ) {
        (1, Some(byte_ranges), None, None) => list_to_ranges(byte_ranges, complement).map(|ranges| {
            Mode::Bytes(
                ranges,
                Options {
                    out_delim: Some(
                        matches
                            .get_one::<String>(options::OUTPUT_DELIMITER)
                            .map(|s| s.as_str())
                            .unwrap_or_default()
                            .to_owned(),
                    ),
                    line_ending: LineEnding::from_zero_flag(matches.get_flag(options::ZERO_TERMINATED)),
                },
            )
        }),
        (1, None, Some(char_ranges), None) => list_to_ranges(char_ranges, complement).map(|ranges| {
            Mode::Characters(
                ranges,
                Options {
                    out_delim: Some(
                        matches
                            .get_one::<String>(options::OUTPUT_DELIMITER)
                            .map(|s| s.as_str())
                            .unwrap_or_default()
                            .to_owned(),
                    ),
                    line_ending: LineEnding::from_zero_flag(matches.get_flag(options::ZERO_TERMINATED)),
                },
            )
        }),
        (1, None, None, Some(field_ranges)) => {
            list_to_ranges(field_ranges, complement).and_then(|ranges| {
                let out_delim = match matches.get_one::<String>(options::OUTPUT_DELIMITER) {
                    Some(s) => {
                        if s.is_empty() {
                            Some("\0".to_owned())
                        } else {
                            Some(s.clone())
                        }
                    }
                    None => None,
                };

                let only_delimited = matches.get_flag(options::ONLY_DELIMITED);
                let whitespace_delimited = matches.get_flag(options::WHITESPACE_DELIMITED);
                let zero_terminated = matches.get_flag(options::ZERO_TERMINATED);
                let line_ending = LineEnding::from_zero_flag(zero_terminated);

                match matches.get_one::<String>(options::DELIMITER).map(|s| s.as_str()) {
                    Some(_) if whitespace_delimited => {
                            Err("invalid input: Only one of --delimiter (-d) or -w option can be specified".into())
                        }
                    Some(mut delim) => {
                        // GNU's `cut` supports `-d=` to set the delimiter to `=`.
                        // Clap parsing is limited in this situation, see:
                        // https://github.com/uutils/coreutils/issues/2424#issuecomment-863825242
                        if delimiter_is_equal {
                            delim = "=";
                        } else if delim == "''" {
                            // treat `''` as empty delimiter
                            delim = "";
                        }
                        if delim.chars().count() > 1 {
                            Err("the delimiter must be a single character".into())
                        } else {
                            let delim = if delim.is_empty() {
                                "\0".to_owned()
                            } else {
                                delim.to_owned()
                            };

                            Ok(Mode::Fields(
                                ranges,
                                FieldOptions {
                                    delimiter: Delimiter::String(delim),
                                    out_delimiter: out_delim,
                                    only_delimited,
                                    line_ending,
                                },
                            ))
                        }
                    }
                    None => Ok(Mode::Fields(
                        ranges,
                        FieldOptions {
                            delimiter: match whitespace_delimited {
                                true => Delimiter::Whitespace,
                                false => Delimiter::String("\t".to_owned()),
                            },
                            out_delimiter: out_delim,
                            only_delimited,
                            line_ending,
                        },
                    )),
                }
            })
        }
        (2.., _, _, _) => Err(
            "invalid usage: expects no more than one of --fields (-f), --chars (-c) or --bytes (-b)".into()
        ),
        _ => Err("invalid usage: expects one of --fields (-f), --chars (-c) or --bytes (-b)".into()),
    };

    let mode_parse = match mode_parse {
        Err(_) => mode_parse,
        Ok(mode) => match mode {
            Mode::Bytes(_, _) | Mode::Characters(_, _)
                if matches.contains_id(options::DELIMITER) =>
            {
                Err("invalid input: The '--delimiter' ('-d') option only usable if printing a sequence of fields".into())
            }
            Mode::Bytes(_, _) | Mode::Characters(_, _)
                if matches.get_flag(options::WHITESPACE_DELIMITED) =>
            {
                Err("invalid input: The '-w' option only usable if printing a sequence of fields".into())
            }
            Mode::Bytes(_, _) | Mode::Characters(_, _)
                if matches.get_flag(options::ONLY_DELIMITED) =>
            {
                Err("invalid input: The '--only-delimited' ('-s') option only usable if printing a sequence of fields".into())
            }
            _ => Ok(mode),
        },
    };

    let files: Vec<String> = matches
        .get_many::<String>(options::FILE)
        .unwrap_or_default()
        .cloned()
        .collect();

    match mode_parse {
        Ok(mode) => {
            cut_files(files, &mode);
            Ok(())
        }
        Err(e) => Err(USimpleError::new(1, e)),
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .infer_long_args(true)
        // While `args_override_self(true)` for some arguments, such as `-d`
        // and `--output-delimiter`, is consistent to the behavior of GNU cut,
        // arguments related to cutting mode, i.e. `-b`, `-c`, `-f`, should
        // cause an error when there is more than one of them, as described in
        // the manual of GNU cut: "Use one, and only one of -b, -c or -f".
        // `ArgAction::Append` is used on `-b`, `-c`, `-f` arguments, so that
        // the occurrences of those could be counted and be handled accordingly.
        .args_override_self(true)
        .arg(
            Arg::new(options::BYTES)
                .short('b')
                .long(options::BYTES)
                .help("filter byte columns from the input source")
                .allow_hyphen_values(true)
                .value_name("LIST")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new(options::CHARACTERS)
                .short('c')
                .long(options::CHARACTERS)
                .help("alias for character mode")
                .allow_hyphen_values(true)
                .value_name("LIST")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new(options::DELIMITER)
                .short('d')
                .long(options::DELIMITER)
                .help("specify the delimiter character that separates fields in the input source. Defaults to Tab.")
                .value_name("DELIM"),
        )
        .arg(
            Arg::new(options::WHITESPACE_DELIMITED)
                .short('w')
                .help("Use any number of whitespace (Space, Tab) to separate fields in the input source (FreeBSD extension).")
                .value_name("WHITESPACE")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FIELDS)
                .short('f')
                .long(options::FIELDS)
                .help("filter field columns from the input source")
                .allow_hyphen_values(true)
                .value_name("LIST")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new(options::COMPLEMENT)
                .long(options::COMPLEMENT)
                .help("invert the filter - instead of displaying only the filtered columns, display all but those columns")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ONLY_DELIMITED)
                .short('s')
                .long(options::ONLY_DELIMITED)
                .help("in field mode, only print lines which contain the delimiter")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ZERO_TERMINATED)
                .short('z')
                .long(options::ZERO_TERMINATED)
                .help("instead of filtering columns based on line, filter columns based on \\0 (NULL character)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OUTPUT_DELIMITER)
                .long(options::OUTPUT_DELIMITER)
                .help("in field mode, replace the delimiter in output lines with this option's argument")
                .value_name("NEW_DELIM"),
        )
        .arg(
            Arg::new(options::FILE)
            .hide(true)
            .action(ArgAction::Append)
            .value_hint(clap::ValueHint::FilePath)
        )
}
