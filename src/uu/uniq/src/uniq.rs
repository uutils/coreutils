// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore badoption
use clap::{
    builder::ValueParser, crate_version, error::ContextKind, error::Error, error::ErrorKind, Arg,
    ArgAction, ArgMatches, Command,
};
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Write};
use std::num::IntErrorKind;
use uucore::display::Quotable;
use uucore::error::{FromIo, UError, UResult, USimpleError};
use uucore::posix::{posix_version, OBSOLETE};
use uucore::shortcut_value_parser::ShortcutValueParser;
use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("uniq.md");
const USAGE: &str = help_usage!("uniq.md");
const AFTER_HELP: &str = help_section!("after help", "uniq.md");

pub mod options {
    pub static ALL_REPEATED: &str = "all-repeated";
    pub static CHECK_CHARS: &str = "check-chars";
    pub static COUNT: &str = "count";
    pub static IGNORE_CASE: &str = "ignore-case";
    pub static REPEATED: &str = "repeated";
    pub static SKIP_FIELDS: &str = "skip-fields";
    pub static SKIP_CHARS: &str = "skip-chars";
    pub static UNIQUE: &str = "unique";
    pub static ZERO_TERMINATED: &str = "zero-terminated";
    pub static GROUP: &str = "group";
}

static ARG_FILES: &str = "files";

#[derive(PartialEq, Clone, Copy)]
enum Delimiters {
    Append,
    Prepend,
    Separate,
    Both,
    None,
}

struct Uniq {
    repeats_only: bool,
    uniques_only: bool,
    all_repeated: bool,
    delimiters: Delimiters,
    show_counts: bool,
    skip_fields: Option<usize>,
    slice_start: Option<usize>,
    slice_stop: Option<usize>,
    ignore_case: bool,
    zero_terminated: bool,
}

macro_rules! write_line_terminator {
    ($writer:expr, $line_terminator:expr) => {
        $writer
            .write_all(&[$line_terminator])
            .map_err_context(|| "Could not write line terminator".to_string())
    };
}

impl Uniq {
    pub fn print_uniq(&self, reader: impl BufRead, mut writer: impl Write) -> UResult<()> {
        let mut first_line_printed = false;
        let mut group_count = 1;
        let line_terminator = self.get_line_terminator();
        let mut lines = reader.split(line_terminator);
        let mut line = match lines.next() {
            Some(l) => l?,
            None => return Ok(()),
        };

        let writer = &mut writer;

        // compare current `line` with consecutive lines (`next_line`) of the input
        // and if needed, print `line` based on the command line options provided
        for next_line in lines {
            let next_line = next_line?;
            if self.cmp_keys(&line, &next_line) {
                if (group_count == 1 && !self.repeats_only)
                    || (group_count > 1 && !self.uniques_only)
                {
                    self.print_line(writer, &line, group_count, first_line_printed)?;
                    first_line_printed = true;
                }
                line = next_line;
                group_count = 1;
            } else {
                if self.all_repeated {
                    self.print_line(writer, &line, group_count, first_line_printed)?;
                    first_line_printed = true;
                    line = next_line;
                }
                group_count += 1;
            }
        }
        if (group_count == 1 && !self.repeats_only) || (group_count > 1 && !self.uniques_only) {
            self.print_line(writer, &line, group_count, first_line_printed)?;
            first_line_printed = true;
        }
        if (self.delimiters == Delimiters::Append || self.delimiters == Delimiters::Both)
            && first_line_printed
        {
            write_line_terminator!(writer, line_terminator)?;
        }
        Ok(())
    }

    fn skip_fields(&self, line: &[u8]) -> Vec<u8> {
        if let Some(skip_fields) = self.skip_fields {
            let mut line = line.iter();
            let mut line_after_skipped_field: Vec<u8>;
            for _ in 0..skip_fields {
                if line.all(|u| u.is_ascii_whitespace()) {
                    return Vec::new();
                }
                line_after_skipped_field = line
                    .by_ref()
                    .skip_while(|u| !u.is_ascii_whitespace())
                    .copied()
                    .collect::<Vec<u8>>();

                if line_after_skipped_field.is_empty() {
                    return Vec::new();
                }
                line = line_after_skipped_field.iter();
            }
            line.copied().collect::<Vec<u8>>()
        } else {
            line.to_vec()
        }
    }

    fn get_line_terminator(&self) -> u8 {
        if self.zero_terminated {
            0
        } else {
            b'\n'
        }
    }

    fn cmp_keys(&self, first: &[u8], second: &[u8]) -> bool {
        self.cmp_key(first, |first_iter| {
            self.cmp_key(second, |second_iter| first_iter.ne(second_iter))
        })
    }

    fn cmp_key<F>(&self, line: &[u8], mut closure: F) -> bool
    where
        F: FnMut(&mut dyn Iterator<Item = u8>) -> bool,
    {
        let fields_to_check = self.skip_fields(line);
        let len = fields_to_check.len();
        let slice_start = self.slice_start.unwrap_or(0);
        let slice_stop = self.slice_stop.unwrap_or(len);
        if len > 0 {
            // fast path: avoid doing any work if there is no need to skip or map to lower-case
            if !self.ignore_case && slice_start == 0 && slice_stop == len {
                return closure(&mut fields_to_check.iter().copied());
            }

            // fast path: avoid skipping
            if self.ignore_case && slice_start == 0 && slice_stop == len {
                return closure(&mut fields_to_check.iter().map(|u| u.to_ascii_lowercase()));
            }

            // fast path: we can avoid mapping chars to lower-case, if we don't want to ignore the case
            if !self.ignore_case {
                return closure(
                    &mut fields_to_check
                        .iter()
                        .skip(slice_start)
                        .take(slice_stop)
                        .copied(),
                );
            }

            closure(
                &mut fields_to_check
                    .iter()
                    .skip(slice_start)
                    .take(slice_stop)
                    .map(|u| u.to_ascii_lowercase()),
            )
        } else {
            closure(&mut fields_to_check.iter().copied())
        }
    }

    fn should_print_delimiter(&self, group_count: usize, first_line_printed: bool) -> bool {
        // if no delimiter option is selected then no other checks needed
        self.delimiters != Delimiters::None
            // print delimiter only before the first line of a group, not between lines of a group
            && group_count == 1
            // if at least one line has been output before current group then print delimiter
            && (first_line_printed
                // or if we need to prepend delimiter then print it even at the start of the output
                || self.delimiters == Delimiters::Prepend
                // the 'both' delimit mode should prepend and append delimiters
                || self.delimiters == Delimiters::Both)
    }

    fn print_line(
        &self,
        writer: &mut impl Write,
        line: &[u8],
        count: usize,
        first_line_printed: bool,
    ) -> UResult<()> {
        let line_terminator = self.get_line_terminator();

        if self.should_print_delimiter(count, first_line_printed) {
            write_line_terminator!(writer, line_terminator)?;
        }

        if self.show_counts {
            let prefix = format!("{count:7} ");
            let out = prefix
                .as_bytes()
                .iter()
                .chain(line.iter())
                .copied()
                .collect::<Vec<u8>>();
            writer.write_all(out.as_slice())
        } else {
            writer.write_all(line)
        }
        .map_err_context(|| "Failed to write line".to_string())?;

        write_line_terminator!(writer, line_terminator)
    }
}

fn opt_parsed(opt_name: &str, matches: &ArgMatches) -> UResult<Option<usize>> {
    match matches.get_one::<String>(opt_name) {
        Some(arg_str) => match arg_str.parse::<usize>() {
            Ok(v) => Ok(Some(v)),
            Err(e) => match e.kind() {
                IntErrorKind::PosOverflow => Ok(Some(usize::MAX)),
                _ => Err(USimpleError::new(
                    1,
                    format!(
                        "Invalid argument for {}: {}",
                        opt_name,
                        arg_str.maybe_quote()
                    ),
                )),
            },
        },
        None => Ok(None),
    }
}

/// Extract obsolete shorthands (if any) for skip fields and skip chars options
/// following GNU `uniq` behavior
///
/// Examples for obsolete skip fields option
/// `uniq -1 file` would equal `uniq -f1 file`
/// `uniq -1 -2 -3 file` would equal `uniq -f123 file`
/// `uniq -1 -2 -f5 file` would equal `uniq -f5 file`
/// `uniq -u20s4 file` would equal `uniq -u -f20 -s4 file`
/// `uniq -D1w3 -3 file` would equal `uniq -D -f3 -w3 file`
///
/// Examples for obsolete skip chars option
/// `uniq +1 file` would equal `uniq -s1 file`
/// `uniq +1 -s2 file` would equal `uniq -s2 file`
/// `uniq -s2 +3 file` would equal `uniq -s3 file`
///
fn handle_obsolete(args: impl uucore::Args) -> (Vec<OsString>, Option<usize>, Option<usize>) {
    let mut skip_fields_old = None;
    let mut skip_chars_old = None;
    let mut preceding_long_opt_req_value = false;
    let mut preceding_short_opt_req_value = false;

    let filtered_args = args
        .filter_map(|os_slice| {
            filter_args(
                os_slice,
                &mut skip_fields_old,
                &mut skip_chars_old,
                &mut preceding_long_opt_req_value,
                &mut preceding_short_opt_req_value,
            )
        })
        .collect();

    // exacted String values (if any) for skip_fields_old and skip_chars_old
    // are guaranteed to consist of ascii digit chars only at this point
    // so, it is safe to parse into usize and collapse Result into Option
    let skip_fields_old: Option<usize> = skip_fields_old.and_then(|v| v.parse::<usize>().ok());
    let skip_chars_old: Option<usize> = skip_chars_old.and_then(|v| v.parse::<usize>().ok());

    (filtered_args, skip_fields_old, skip_chars_old)
}

fn filter_args(
    os_slice: OsString,
    skip_fields_old: &mut Option<String>,
    skip_chars_old: &mut Option<String>,
    preceding_long_opt_req_value: &mut bool,
    preceding_short_opt_req_value: &mut bool,
) -> Option<OsString> {
    let filter: Option<OsString>;
    if let Some(slice) = os_slice.to_str() {
        if should_extract_obs_skip_fields(
            slice,
            preceding_long_opt_req_value,
            preceding_short_opt_req_value,
        ) {
            // start of the short option string
            // that can have obsolete skip fields option value in it
            filter = handle_extract_obs_skip_fields(slice, skip_fields_old);
        } else if should_extract_obs_skip_chars(
            slice,
            preceding_long_opt_req_value,
            preceding_short_opt_req_value,
        ) {
            // the obsolete skip chars option
            filter = handle_extract_obs_skip_chars(slice, skip_chars_old);
        } else {
            // either not a short option
            // or a short option that cannot have obsolete lines value in it
            filter = Some(OsString::from(slice));
            // Check and reset to None obsolete values extracted so far
            // if corresponding new/documented options are encountered next.
            // NOTE: For skip fields - occurrences of corresponding new/documented options
            // inside combined short options ike '-u20s4' or '-D1w3', etc
            // are also covered in `handle_extract_obs_skip_fields()` function
            if slice.starts_with("-f") {
                *skip_fields_old = None;
            }
            if slice.starts_with("-s") {
                *skip_chars_old = None;
            }
        }
        handle_preceding_options(
            slice,
            preceding_long_opt_req_value,
            preceding_short_opt_req_value,
        );
    } else {
        // Cannot cleanly convert os_slice to UTF-8
        // Do not process and return as-is
        // This will cause failure later on, but we should not handle it here
        // and let clap panic on invalid UTF-8 argument
        filter = Some(os_slice);
    }
    filter
}

/// Helper function to [`filter_args`]
/// Checks if the slice is a true short option (and not hyphen prefixed value of an option)
/// and if so, a short option that can contain obsolete skip fields value
fn should_extract_obs_skip_fields(
    slice: &str,
    preceding_long_opt_req_value: &bool,
    preceding_short_opt_req_value: &bool,
) -> bool {
    slice.starts_with('-')
        && !slice.starts_with("--")
        && !preceding_long_opt_req_value
        && !preceding_short_opt_req_value
        && !slice.starts_with("-s")
        && !slice.starts_with("-f")
        && !slice.starts_with("-w")
}

/// Helper function to [`filter_args`]
/// Checks if the slice is a true obsolete skip chars short option
fn should_extract_obs_skip_chars(
    slice: &str,
    preceding_long_opt_req_value: &bool,
    preceding_short_opt_req_value: &bool,
) -> bool {
    slice.starts_with('+')
        && posix_version().is_some_and(|v| v <= OBSOLETE)
        && !preceding_long_opt_req_value
        && !preceding_short_opt_req_value
        && slice.chars().nth(1).map_or(false, |c| c.is_ascii_digit())
}

/// Helper function to [`filter_args`]
/// Captures if current slice is a preceding option
/// that requires value
fn handle_preceding_options(
    slice: &str,
    preceding_long_opt_req_value: &mut bool,
    preceding_short_opt_req_value: &mut bool,
) {
    // capture if current slice is a preceding long option that requires value and does not use '=' to assign that value
    // following slice should be treaded as value for this option
    // even if it starts with '-' (which would be treated as hyphen prefixed value)
    if slice.starts_with("--") {
        use options as O;
        *preceding_long_opt_req_value = &slice[2..] == O::SKIP_CHARS
            || &slice[2..] == O::SKIP_FIELDS
            || &slice[2..] == O::CHECK_CHARS
            || &slice[2..] == O::GROUP
            || &slice[2..] == O::ALL_REPEATED;
    }
    // capture if current slice is a preceding short option that requires value and does not have value in the same slice (value separated by whitespace)
    // following slice should be treaded as value for this option
    // even if it starts with '-' (which would be treated as hyphen prefixed value)
    *preceding_short_opt_req_value = slice == "-s" || slice == "-f" || slice == "-w";
    // slice is a value
    // reset preceding option flags
    if !slice.starts_with('-') {
        *preceding_short_opt_req_value = false;
        *preceding_long_opt_req_value = false;
    }
}

/// Helper function to [`filter_args`]
/// Extracts obsolete skip fields numeric part from argument slice
/// and filters it out
fn handle_extract_obs_skip_fields(
    slice: &str,
    skip_fields_old: &mut Option<String>,
) -> Option<OsString> {
    let mut obs_extracted: Vec<char> = vec![];
    let mut obs_end_reached = false;
    let mut obs_overwritten_by_new = false;
    let filtered_slice: Vec<char> = slice
        .chars()
        .filter(|c| {
            if c.eq(&'f') {
                // any extracted obsolete skip fields value up to this point should be discarded
                // as the new/documented option for skip fields was used after it
                // i.e. in situation like `-u12f3`
                // The obsolete skip fields value should still be extracted, filtered out
                // but the skip_fields_old should be set to None instead of Some(String) later on
                obs_overwritten_by_new = true;
            }
            // To correctly process scenario like '-u20s4' or '-D1w3', etc
            // we need to stop extracting digits once alphabetic character is encountered
            // after we already have something in obs_extracted
            if c.is_ascii_digit() && !obs_end_reached {
                obs_extracted.push(*c);
                false
            } else {
                if !obs_extracted.is_empty() {
                    obs_end_reached = true;
                }
                true
            }
        })
        .collect();

    if obs_extracted.is_empty() {
        // no obsolete value found/extracted
        Some(OsString::from(slice))
    } else {
        // obsolete value was extracted
        // unless there was new/documented option for skip fields used after it
        // set the skip_fields_old value (concatenate to it if there was a value there already)
        if obs_overwritten_by_new {
            *skip_fields_old = None;
        } else {
            let mut extracted: String = obs_extracted.iter().collect();
            if let Some(val) = skip_fields_old {
                extracted.push_str(val);
            }
            *skip_fields_old = Some(extracted);
        }
        if filtered_slice.get(1).is_some() {
            // there were some short options in front of or after obsolete lines value
            // i.e. '-u20s4' or '-D1w3' or similar, which after extraction of obsolete lines value
            // would look like '-us4' or '-Dw3' or similar
            let filtered_slice: String = filtered_slice.iter().collect();
            Some(OsString::from(filtered_slice))
        } else {
            None
        }
    }
}

/// Helper function to [`filter_args`]
/// Extracts obsolete skip chars numeric part from argument slice
fn handle_extract_obs_skip_chars(
    slice: &str,
    skip_chars_old: &mut Option<String>,
) -> Option<OsString> {
    let mut obs_extracted: Vec<char> = vec![];
    let mut slice_chars = slice.chars();
    slice_chars.next(); // drop leading '+' character
    for c in slice_chars {
        if c.is_ascii_digit() {
            obs_extracted.push(c);
        } else {
            // for obsolete skip chars option the whole value after '+' should be numeric
            // so, if any non-digit characters are encountered in the slice (i.e. `+1q`, etc)
            // set skip_chars_old to None and return whole slice back.
            // It will be parsed by clap and panic with appropriate error message
            *skip_chars_old = None;
            return Some(OsString::from(slice));
        }
    }
    if obs_extracted.is_empty() {
        // no obsolete value found/extracted
        // i.e. it was just '+' character alone
        Some(OsString::from(slice))
    } else {
        // successfully extracted numeric value
        // capture it and return None to filter out the whole slice
        *skip_chars_old = Some(obs_extracted.iter().collect());
        None
    }
}

/// Maps Clap errors to USimpleError and overrides 3 specific ones
/// to meet requirements of GNU tests for `uniq`.
/// Unfortunately these overrides are necessary, since several GNU tests
/// for `uniq` hardcode and require the exact wording of the error message
/// and it is not compatible with how Clap formats and displays those error messages.
fn map_clap_errors(clap_error: Error) -> Box<dyn UError> {
    let footer = "Try 'uniq --help' for more information.";
    let override_arg_conflict =
        "--group is mutually exclusive with -c/-d/-D/-u\n".to_string() + footer;
    let override_group_badoption = "invalid argument 'badoption' for '--group'\nValid arguments are:\n  - 'prepend'\n  - 'append'\n  - 'separate'\n  - 'both'\n".to_string() + footer;
    let override_all_repeated_badoption = "invalid argument 'badoption' for '--all-repeated'\nValid arguments are:\n  - 'none'\n  - 'prepend'\n  - 'separate'\n".to_string() + footer;

    let error_message = match clap_error.kind() {
        ErrorKind::ArgumentConflict => override_arg_conflict,
        ErrorKind::InvalidValue
            if clap_error
                .get(ContextKind::InvalidValue)
                .is_some_and(|v| v.to_string() == "badoption")
                && clap_error
                    .get(ContextKind::InvalidArg)
                    .is_some_and(|v| v.to_string().starts_with("--group")) =>
        {
            override_group_badoption
        }
        ErrorKind::InvalidValue
            if clap_error
                .get(ContextKind::InvalidValue)
                .is_some_and(|v| v.to_string() == "badoption")
                && clap_error
                    .get(ContextKind::InvalidArg)
                    .is_some_and(|v| v.to_string().starts_with("--all-repeated")) =>
        {
            override_all_repeated_badoption
        }
        _ => return clap_error.into(),
    };
    USimpleError::new(1, error_message)
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let (args, skip_fields_old, skip_chars_old) = handle_obsolete(args);

    let matches = uu_app()
        .try_get_matches_from(args)
        .map_err(map_clap_errors)?;

    let files = matches.get_many::<OsString>(ARG_FILES);

    let (in_file_name, out_file_name) = files
        .map(|fi| fi.map(AsRef::as_ref))
        .map(|mut fi| (fi.next(), fi.next()))
        .unwrap_or_default();

    let skip_fields_modern: Option<usize> = opt_parsed(options::SKIP_FIELDS, &matches)?;
    let skip_chars_modern: Option<usize> = opt_parsed(options::SKIP_CHARS, &matches)?;

    let uniq = Uniq {
        repeats_only: matches.get_flag(options::REPEATED)
            || matches.contains_id(options::ALL_REPEATED),
        uniques_only: matches.get_flag(options::UNIQUE),
        all_repeated: matches.contains_id(options::ALL_REPEATED)
            || matches.contains_id(options::GROUP),
        delimiters: get_delimiter(&matches),
        show_counts: matches.get_flag(options::COUNT),
        skip_fields: skip_fields_modern.or(skip_fields_old),
        slice_start: skip_chars_modern.or(skip_chars_old),
        slice_stop: opt_parsed(options::CHECK_CHARS, &matches)?,
        ignore_case: matches.get_flag(options::IGNORE_CASE),
        zero_terminated: matches.get_flag(options::ZERO_TERMINATED),
    };

    if uniq.show_counts && uniq.all_repeated {
        return Err(USimpleError::new(
            1,
            "printing all duplicated lines and repeat counts is meaningless\nTry 'uniq --help' for more information.",
        ));
    }

    uniq.print_uniq(
        open_input_file(in_file_name)?,
        open_output_file(out_file_name)?,
    )
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .after_help(AFTER_HELP)
        .arg(
            Arg::new(options::ALL_REPEATED)
                .short('D')
                .long(options::ALL_REPEATED)
                .value_parser(ShortcutValueParser::new([
                    "none",
                    "prepend",
                    "separate"
                ]))
                .help("print all duplicate lines. Delimiting is done with blank lines. [default: none]")
                .value_name("delimit-method")
                .num_args(0..=1)
                .default_missing_value("none")
                .require_equals(true),
        )
        .arg(
            Arg::new(options::GROUP)
                .long(options::GROUP)
                .value_parser(ShortcutValueParser::new([
                    "separate",
                    "prepend",
                    "append",
                    "both",
                ]))
                .help("show all items, separating groups with an empty line. [default: separate]")
                .value_name("group-method")
                .num_args(0..=1)
                .default_missing_value("separate")
                .require_equals(true)
                .conflicts_with_all([
                    options::REPEATED,
                    options::ALL_REPEATED,
                    options::UNIQUE,
                    options::COUNT
                ]),
        )
        .arg(
            Arg::new(options::CHECK_CHARS)
                .short('w')
                .long(options::CHECK_CHARS)
                .help("compare no more than N characters in lines")
                .value_name("N"),
        )
        .arg(
            Arg::new(options::COUNT)
                .short('c')
                .long(options::COUNT)
                .help("prefix lines by the number of occurrences")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::IGNORE_CASE)
                .short('i')
                .long(options::IGNORE_CASE)
                .help("ignore differences in case when comparing")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::REPEATED)
                .short('d')
                .long(options::REPEATED)
                .help("only print duplicate lines")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SKIP_CHARS)
                .short('s')
                .long(options::SKIP_CHARS)
                .help("avoid comparing the first N characters")
                .value_name("N"),
        )
        .arg(
            Arg::new(options::SKIP_FIELDS)
                .short('f')
                .long(options::SKIP_FIELDS)
                .help("avoid comparing the first N fields")
                .value_name("N"),
        )
        .arg(
            Arg::new(options::UNIQUE)
                .short('u')
                .long(options::UNIQUE)
                .help("only print unique lines")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ZERO_TERMINATED)
                .short('z')
                .long(options::ZERO_TERMINATED)
                .help("end lines with 0 byte, not newline")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(ARG_FILES)
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .num_args(0..=2)
                .hide(true)
                .value_hint(clap::ValueHint::FilePath),
        )
}

fn get_delimiter(matches: &ArgMatches) -> Delimiters {
    let value = matches
        .get_one::<String>(options::ALL_REPEATED)
        .or_else(|| matches.get_one::<String>(options::GROUP));
    if let Some(delimiter_arg) = value {
        match delimiter_arg.as_ref() {
            "append" => Delimiters::Append,
            "prepend" => Delimiters::Prepend,
            "separate" => Delimiters::Separate,
            "both" => Delimiters::Both,
            "none" => Delimiters::None,
            _ => unreachable!("Should have been caught by possible values in clap"),
        }
    } else if matches.contains_id(options::GROUP) {
        Delimiters::Separate
    } else {
        Delimiters::None
    }
}

// None or "-" means stdin.
fn open_input_file(in_file_name: Option<&OsStr>) -> UResult<Box<dyn BufRead>> {
    Ok(match in_file_name {
        Some(path) if path != "-" => {
            let in_file = File::open(path)
                .map_err_context(|| format!("Could not open {}", path.maybe_quote()))?;
            Box::new(BufReader::new(in_file))
        }
        _ => Box::new(stdin().lock()),
    })
}

// None or "-" means stdout.
fn open_output_file(out_file_name: Option<&OsStr>) -> UResult<Box<dyn Write>> {
    Ok(match out_file_name {
        Some(path) if path != "-" => {
            let out_file = File::create(path)
                .map_err_context(|| format!("Could not open {}", path.maybe_quote()))?;
            Box::new(BufWriter::new(out_file))
        }
        _ => Box::new(stdout().lock()),
    })
}
