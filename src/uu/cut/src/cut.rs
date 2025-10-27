// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) delim sourcefiles

use bstr::io::BufReadExt;
use clap::{Arg, ArgAction, ArgMatches, Command, builder::ValueParser};
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, IsTerminal, Read, Write, stdin, stdout};
use std::path::Path;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError, set_exit_code};
use uucore::line_ending::LineEnding;
use uucore::os_str_as_bytes;

use self::searcher::Searcher;
use matcher::{ExactMatcher, Matcher, WhitespaceMatcher};
use uucore::ranges::Range;
use uucore::translate;
use uucore::{format_usage, show_error, show_if_err};

mod matcher;
mod searcher;

struct Options<'a> {
    out_delimiter: Option<&'a [u8]>,
    line_ending: LineEnding,
    field_opts: Option<FieldOptions<'a>>,
}

enum Delimiter<'a> {
    Whitespace,
    Slice(&'a [u8]),
}

struct FieldOptions<'a> {
    delimiter: Delimiter<'a>,
    only_delimited: bool,
}

enum Mode<'a> {
    Bytes(Vec<Range>, Options<'a>),
    Characters(Vec<Range>, Options<'a>),
    Fields(Vec<Range>, Options<'a>),
}

impl Default for Delimiter<'_> {
    fn default() -> Self {
        Self::Slice(b"\t")
    }
}

impl<'a> From<&'a OsString> for Delimiter<'a> {
    fn from(s: &'a OsString) -> Self {
        Self::Slice(os_str_as_bytes(s).unwrap())
    }
}

fn list_to_ranges(list: &str, complement: bool) -> Result<Vec<Range>, String> {
    if complement {
        Range::from_list(list).map(|r| uucore::ranges::complement(&r))
    } else {
        Range::from_list(list)
    }
}

fn cut_bytes<R: Read, W: Write>(
    reader: R,
    out: &mut W,
    ranges: &[Range],
    opts: &Options,
) -> UResult<()> {
    let newline_char = opts.line_ending.into();
    let mut buf_in = BufReader::new(reader);
    let out_delim = opts.out_delimiter.unwrap_or(b"\t");

    let result = buf_in.for_byte_record(newline_char, |line| {
        let mut print_delim = false;
        for &Range { low, high } in ranges {
            if low > line.len() {
                break;
            }
            if print_delim {
                out.write_all(out_delim)?;
            } else if opts.out_delimiter.is_some() {
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

/// Output delimiter is explicitly specified
fn cut_fields_explicit_out_delim<R: Read, W: Write, M: Matcher>(
    reader: R,
    out: &mut W,
    matcher: &M,
    ranges: &[Range],
    only_delimited: bool,
    newline_char: u8,
    out_delim: &[u8],
) -> UResult<()> {
    let mut buf_in = BufReader::new(reader);

    let result = buf_in.for_byte_record_with_terminator(newline_char, |line| {
        let mut fields_pos = 1;
        let mut low_idx = 0;
        let mut delim_search = Searcher::new(matcher, line).peekable();
        let mut print_delim = false;

        if delim_search.peek().is_none() {
            if !only_delimited {
                // Always write the entire line, even if it doesn't end with `newline_char`
                out.write_all(line)?;
                if line.is_empty() || line[line.len() - 1] != newline_char {
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
                    out.write_all(out_delim)?;
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

/// Output delimiter is the same as input delimiter
fn cut_fields_implicit_out_delim<R: Read, W: Write, M: Matcher>(
    reader: R,
    out: &mut W,
    matcher: &M,
    ranges: &[Range],
    only_delimited: bool,
    newline_char: u8,
) -> UResult<()> {
    let mut buf_in = BufReader::new(reader);

    let result = buf_in.for_byte_record_with_terminator(newline_char, |line| {
        let mut fields_pos = 1;
        let mut low_idx = 0;
        let mut delim_search = Searcher::new(matcher, line).peekable();
        let mut print_delim = false;

        if delim_search.peek().is_none() {
            if !only_delimited {
                // Always write the entire line, even if it doesn't end with `newline_char`
                out.write_all(line)?;
                if line.is_empty() || line[line.len() - 1] != newline_char {
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

/// The input delimiter is identical to `newline_char`
fn cut_fields_newline_char_delim<R: Read, W: Write>(
    reader: R,
    out: &mut W,
    ranges: &[Range],
    newline_char: u8,
    out_delim: &[u8],
) -> UResult<()> {
    let buf_in = BufReader::new(reader);

    let segments: Vec<_> = buf_in.split(newline_char).filter_map(|x| x.ok()).collect();
    let mut print_delim = false;

    for &Range { low, high } in ranges {
        for i in low..=high {
            // "- 1" is necessary because fields start from 1 whereas a Vec starts from 0
            if let Some(segment) = segments.get(i - 1) {
                if print_delim {
                    out.write_all(out_delim)?;
                } else {
                    print_delim = true;
                }
                out.write_all(segment.as_slice())?;
            } else {
                break;
            }
        }
    }
    out.write_all(&[newline_char])?;
    Ok(())
}

fn cut_fields<R: Read, W: Write>(
    reader: R,
    out: &mut W,
    ranges: &[Range],
    opts: &Options,
) -> UResult<()> {
    let newline_char = opts.line_ending.into();
    let field_opts = opts.field_opts.as_ref().unwrap(); // it is safe to unwrap() here - field_opts will always be Some() for cut_fields() call
    match field_opts.delimiter {
        Delimiter::Slice(delim) if delim == [newline_char] => {
            let out_delim = opts.out_delimiter.unwrap_or(delim);
            cut_fields_newline_char_delim(reader, out, ranges, newline_char, out_delim)
        }
        Delimiter::Slice(delim) => {
            let matcher = ExactMatcher::new(delim);
            match opts.out_delimiter {
                Some(out_delim) => cut_fields_explicit_out_delim(
                    reader,
                    out,
                    &matcher,
                    ranges,
                    field_opts.only_delimited,
                    newline_char,
                    out_delim,
                ),
                None => cut_fields_implicit_out_delim(
                    reader,
                    out,
                    &matcher,
                    ranges,
                    field_opts.only_delimited,
                    newline_char,
                ),
            }
        }
        Delimiter::Whitespace => {
            let matcher = WhitespaceMatcher {};
            cut_fields_explicit_out_delim(
                reader,
                out,
                &matcher,
                ranges,
                field_opts.only_delimited,
                newline_char,
                opts.out_delimiter.unwrap_or(b"\t"),
            )
        }
    }
}

fn cut_files(mut filenames: Vec<OsString>, mode: &Mode) {
    let mut stdin_read = false;

    if filenames.is_empty() {
        filenames.push(OsString::from("-"));
    }

    let mut out: Box<dyn Write> = if stdout().is_terminal() {
        Box::new(stdout())
    } else {
        Box::new(BufWriter::new(stdout())) as Box<dyn Write>
    };

    for filename in &filenames {
        if filename == "-" {
            if stdin_read {
                continue;
            }

            show_if_err!(match mode {
                Mode::Bytes(ranges, opts) => cut_bytes(stdin(), &mut out, ranges, opts),
                Mode::Characters(ranges, opts) => cut_bytes(stdin(), &mut out, ranges, opts),
                Mode::Fields(ranges, opts) => cut_fields(stdin(), &mut out, ranges, opts),
            });

            stdin_read = true;
        } else {
            let path = Path::new(filename);

            if path.is_dir() {
                show_error!(
                    "{}: {}",
                    filename.to_string_lossy().maybe_quote(),
                    translate!("cut-error-is-directory")
                );
                set_exit_code(1);
                continue;
            }

            show_if_err!(
                File::open(path)
                    .map_err_context(|| filename.to_string_lossy().to_string())
                    .and_then(|file| {
                        match &mode {
                            Mode::Bytes(ranges, opts) | Mode::Characters(ranges, opts) => {
                                cut_bytes(file, &mut out, ranges, opts)
                            }
                            Mode::Fields(ranges, opts) => cut_fields(file, &mut out, ranges, opts),
                        }
                    })
            );
        }
    }

    show_if_err!(
        out.flush()
            .map_err_context(|| translate!("cut-error-write-error"))
    );
}

/// Get delimiter and output delimiter from `-d`/`--delimiter` and `--output-delimiter` options respectively
/// Allow either delimiter to have a value that is neither UTF-8 nor ASCII to align with GNU behavior
fn get_delimiters(matches: &ArgMatches) -> UResult<(Delimiter<'_>, Option<&[u8]>)> {
    let whitespace_delimited = matches.get_flag(options::WHITESPACE_DELIMITED);
    let delim_opt = matches.get_one::<OsString>(options::DELIMITER);
    let delim = match delim_opt {
        Some(_) if whitespace_delimited => {
            return Err(USimpleError::new(
                1,
                translate!("cut-error-delimiter-and-whitespace-conflict"),
            ));
        }
        Some(os_string) => {
            if os_string == "''" || os_string.is_empty() {
                // treat `''` as empty delimiter
                Delimiter::Slice(b"\0")
            } else {
                // For delimiter `-d` option value - allow both UTF-8 (possibly multi-byte) characters
                // and Non UTF-8 (and not ASCII) single byte "characters", like `b"\xAD"` to align with GNU behavior
                let bytes = os_str_as_bytes(os_string)?;
                if os_string.to_str().is_some_and(|s| s.chars().count() > 1)
                    || os_string.to_str().is_none() && bytes.len() > 1
                {
                    return Err(USimpleError::new(
                        1,
                        translate!("cut-error-delimiter-must-be-single-character"),
                    ));
                }
                Delimiter::from(os_string)
            }
        }
        None => {
            if whitespace_delimited {
                Delimiter::Whitespace
            } else {
                Delimiter::default()
            }
        }
    };
    let out_delim = matches
        .get_one::<OsString>(options::OUTPUT_DELIMITER)
        .map(|os_string| {
            if os_string.is_empty() || os_string == "''" {
                b"\0"
            } else {
                os_str_as_bytes(os_string).unwrap()
            }
        });
    Ok((delim, out_delim))
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
    // ignored option
    pub const NOTHING: &str = "nothing";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    // GNU's `cut` supports `-d=` to set the delimiter to `=`.
    // Clap parsing is limited in this situation, see:
    // https://github.com/uutils/coreutils/issues/2424#issuecomment-863825242
    let args: Vec<OsString> = args
        .into_iter()
        .map(|x| {
            if x == "-d=" {
                "--delimiter==".into()
            } else {
                x
            }
        })
        .collect();

    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let complement = matches.get_flag(options::COMPLEMENT);
    let only_delimited = matches.get_flag(options::ONLY_DELIMITED);

    let (delimiter, out_delimiter) = get_delimiters(&matches)?;
    let line_ending = LineEnding::from_zero_flag(matches.get_flag(options::ZERO_TERMINATED));

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
        (1, Some(byte_ranges), None, None) => {
            list_to_ranges(byte_ranges, complement).map(|ranges| {
                Mode::Bytes(
                    ranges,
                    Options {
                        out_delimiter,
                        line_ending,
                        field_opts: None,
                    },
                )
            })
        }

        (1, None, Some(char_ranges), None) => {
            list_to_ranges(char_ranges, complement).map(|ranges| {
                Mode::Characters(
                    ranges,
                    Options {
                        out_delimiter,
                        line_ending,
                        field_opts: None,
                    },
                )
            })
        }

        (1, None, None, Some(field_ranges)) => {
            list_to_ranges(field_ranges, complement).map(|ranges| {
                Mode::Fields(
                    ranges,
                    Options {
                        out_delimiter,
                        line_ending,
                        field_opts: Some(FieldOptions {
                            delimiter,
                            only_delimited,
                        }),
                    },
                )
            })
        }

        (2.., _, _, _) => Err(translate!("cut-error-multiple-mode-args")),
        _ => Err(translate!("cut-error-missing-mode-arg")),
    };

    let mode_parse = match mode_parse {
        Err(_) => mode_parse,
        Ok(mode) => match mode {
            Mode::Bytes(_, _) | Mode::Characters(_, _)
                if matches.contains_id(options::DELIMITER) =>
            {
                Err(translate!("cut-error-delimiter-only-with-fields"))
            }
            Mode::Bytes(_, _) | Mode::Characters(_, _)
                if matches.get_flag(options::WHITESPACE_DELIMITED) =>
            {
                Err(translate!("cut-error-whitespace-only-with-fields"))
            }
            Mode::Bytes(_, _) | Mode::Characters(_, _)
                if matches.get_flag(options::ONLY_DELIMITED) =>
            {
                Err(translate!("cut-error-only-delimited-only-with-fields"))
            }
            _ => Ok(mode),
        },
    };

    let files: Vec<OsString> = matches
        .get_many::<OsString>(options::FILE)
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
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .override_usage(format_usage(&translate!("cut-usage")))
        .about(translate!("cut-about"))
        .after_help(translate!("cut-after-help"))
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
                .help(translate!("cut-help-bytes"))
                .allow_hyphen_values(true)
                .value_name("LIST")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new(options::CHARACTERS)
                .short('c')
                .long(options::CHARACTERS)
                .help(translate!("cut-help-characters"))
                .allow_hyphen_values(true)
                .value_name("LIST")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new(options::DELIMITER)
                .short('d')
                .long(options::DELIMITER)
                .value_parser(ValueParser::os_string())
                .help(translate!("cut-help-delimiter"))
                .value_name("DELIM"),
        )
        .arg(
            Arg::new(options::WHITESPACE_DELIMITED)
                .short('w')
                .help(translate!("cut-help-whitespace-delimited"))
                .value_name("WHITESPACE")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FIELDS)
                .short('f')
                .long(options::FIELDS)
                .help(translate!("cut-help-fields"))
                .allow_hyphen_values(true)
                .value_name("LIST")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new(options::COMPLEMENT)
                .long(options::COMPLEMENT)
                .help(translate!("cut-help-complement"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ONLY_DELIMITED)
                .short('s')
                .long(options::ONLY_DELIMITED)
                .help(translate!("cut-help-only-delimited"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ZERO_TERMINATED)
                .short('z')
                .long(options::ZERO_TERMINATED)
                .help(translate!("cut-help-zero-terminated"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OUTPUT_DELIMITER)
                .long(options::OUTPUT_DELIMITER)
                .value_parser(ValueParser::os_string())
                .help(translate!("cut-help-output-delimiter"))
                .value_name("NEW_DELIM"),
        )
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString)),
        )
        .arg(
            Arg::new(options::NOTHING)
                .short('n')
                .help("(ignored)")
                .action(ArgAction::SetTrue),
        )
}
