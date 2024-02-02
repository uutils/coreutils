// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{builder::ValueParser, crate_version, Arg, ArgAction, ArgGroup, ArgMatches, Command};
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{self, stdin, stdout, BufRead, BufReader, BufWriter, Write};
use std::str::FromStr;
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};
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
    pub static OBSOLETE_SKIP_FIELDS: &str = "obsolete_skip_field";
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

const OBSOLETE_SKIP_FIELDS_DIGITS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

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
        let mut lines = reader.split(line_terminator).map(get_line_string);
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

    fn skip_fields<'a>(&self, line: &'a str) -> &'a str {
        if let Some(skip_fields) = self.skip_fields {
            let mut i = 0;
            let mut char_indices = line.char_indices();
            for _ in 0..skip_fields {
                if char_indices.all(|(_, c)| c.is_whitespace()) {
                    return "";
                }
                match char_indices.find(|(_, c)| c.is_whitespace()) {
                    None => return "",
                    Some((next_field_i, _)) => i = next_field_i,
                }
            }
            &line[i..]
        } else {
            line
        }
    }

    fn get_line_terminator(&self) -> u8 {
        if self.zero_terminated {
            0
        } else {
            b'\n'
        }
    }

    fn cmp_keys(&self, first: &str, second: &str) -> bool {
        self.cmp_key(first, |first_iter| {
            self.cmp_key(second, |second_iter| first_iter.ne(second_iter))
        })
    }

    fn cmp_key<F>(&self, line: &str, mut closure: F) -> bool
    where
        F: FnMut(&mut dyn Iterator<Item = char>) -> bool,
    {
        let fields_to_check = self.skip_fields(line);
        let len = fields_to_check.len();
        let slice_start = self.slice_start.unwrap_or(0);
        let slice_stop = self.slice_stop.unwrap_or(len);
        if len > 0 {
            // fast path: avoid doing any work if there is no need to skip or map to lower-case
            if !self.ignore_case && slice_start == 0 && slice_stop == len {
                return closure(&mut fields_to_check.chars());
            }

            // fast path: avoid skipping
            if self.ignore_case && slice_start == 0 && slice_stop == len {
                return closure(&mut fields_to_check.chars().flat_map(char::to_uppercase));
            }

            // fast path: we can avoid mapping chars to upper-case, if we don't want to ignore the case
            if !self.ignore_case {
                return closure(&mut fields_to_check.chars().skip(slice_start).take(slice_stop));
            }

            closure(
                &mut fields_to_check
                    .chars()
                    .skip(slice_start)
                    .take(slice_stop)
                    .flat_map(char::to_uppercase),
            )
        } else {
            closure(&mut fields_to_check.chars())
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
        line: &str,
        count: usize,
        first_line_printed: bool,
    ) -> UResult<()> {
        let line_terminator = self.get_line_terminator();

        if self.should_print_delimiter(count, first_line_printed) {
            write_line_terminator!(writer, line_terminator)?;
        }

        if self.show_counts {
            write!(writer, "{count:7} {line}")
        } else {
            writer.write_all(line.as_bytes())
        }
        .map_err_context(|| "Failed to write line".to_string())?;

        write_line_terminator!(writer, line_terminator)
    }
}

fn get_line_string(io_line: io::Result<Vec<u8>>) -> UResult<String> {
    let line_bytes = io_line.map_err_context(|| "failed to split lines".to_string())?;
    String::from_utf8(line_bytes)
        .map_err(|e| USimpleError::new(1, format!("failed to convert line to utf8: {e}")))
}

fn opt_parsed<T: FromStr>(opt_name: &str, matches: &ArgMatches) -> UResult<Option<T>> {
    Ok(match matches.get_one::<String>(opt_name) {
        Some(arg_str) => Some(arg_str.parse().map_err(|_| {
            USimpleError::new(
                1,
                format!(
                    "Invalid argument for {}: {}",
                    opt_name,
                    arg_str.maybe_quote()
                ),
            )
        })?),
        None => None,
    })
}

/// Gets number of fields to be skipped from the shorthand option `-N`
///
/// ```bash
/// uniq -12345
/// ```
/// the first digit isn't interpreted by clap as part of the value
/// so `get_one()` would return `2345`, then to get the actual value
/// we loop over every possible first digit, only one of which can be
/// found in the command line because they conflict with each other,
/// append the value to it and parse the resulting string as usize,
/// an error at this point means that a character that isn't a digit was given
fn obsolete_skip_field(matches: &ArgMatches) -> UResult<Option<usize>> {
    for opt_text in OBSOLETE_SKIP_FIELDS_DIGITS {
        let argument = matches.get_one::<String>(opt_text);
        if matches.contains_id(opt_text) {
            let mut full = opt_text.to_owned();
            if let Some(ar) = argument {
                full.push_str(ar);
            }
            let value = full.parse::<usize>();

            if let Ok(val) = value {
                return Ok(Some(val));
            } else {
                return Err(USimpleError {
                    code: 1,
                    message: format!("Invalid argument for skip-fields: {}", full),
                }
                .into());
            }
        }
    }
    Ok(None)
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().after_help(AFTER_HELP).try_get_matches_from(args)?;

    let files = matches.get_many::<OsString>(ARG_FILES);

    let (in_file_name, out_file_name) = files
        .map(|fi| fi.map(AsRef::as_ref))
        .map(|mut fi| (fi.next(), fi.next()))
        .unwrap_or_default();

    let skip_fields_modern: Option<usize> = opt_parsed(options::SKIP_FIELDS, &matches)?;

    let skip_fields_old: Option<usize> = obsolete_skip_field(&matches)?;

    let uniq = Uniq {
        repeats_only: matches.get_flag(options::REPEATED)
            || matches.contains_id(options::ALL_REPEATED),
        uniques_only: matches.get_flag(options::UNIQUE),
        all_repeated: matches.contains_id(options::ALL_REPEATED)
            || matches.contains_id(options::GROUP),
        delimiters: get_delimiter(&matches),
        show_counts: matches.get_flag(options::COUNT),
        skip_fields: skip_fields_modern.or(skip_fields_old),
        slice_start: opt_parsed(options::SKIP_CHARS, &matches)?,
        slice_stop: opt_parsed(options::CHECK_CHARS, &matches)?,
        ignore_case: matches.get_flag(options::IGNORE_CASE),
        zero_terminated: matches.get_flag(options::ZERO_TERMINATED),
    };

    if uniq.show_counts && uniq.all_repeated {
        return Err(UUsageError::new(
            1,
            "printing all duplicated lines and repeat counts is meaningless",
        ));
    }

    uniq.print_uniq(
        open_input_file(in_file_name)?,
        open_output_file(out_file_name)?,
    )
}

pub fn uu_app() -> Command {
    let mut cmd = Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::ALL_REPEATED)
                .short('D')
                .long(options::ALL_REPEATED)
                .value_parser([
                    "none",
                    "prepend",
                    "separate"
                ])
                .help("print all duplicate lines. Delimiting is done with blank lines. [default: none]")
                .value_name("delimit-method")
                .num_args(0..=1)
                .default_missing_value("none")
                .require_equals(true),
        )
        .arg(
            Arg::new(options::GROUP)
                .long(options::GROUP)
                .value_parser([
                    "separate",
                    "prepend",
                    "append",
                    "both",
                ])
                .help("show all items, separating groups with an empty line. [default: separate]")
                .value_name("group-method")
                .num_args(0..=1)
                .default_missing_value("separate")
                .require_equals(true)
                .conflicts_with_all([
                    options::REPEATED,
                    options::ALL_REPEATED,
                    options::UNIQUE,
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
                .overrides_with_all(OBSOLETE_SKIP_FIELDS_DIGITS)
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
        .group(
            // in GNU `uniq` every every digit of these arguments
            // would be interpreted as a simple flag,
            // these flags then are concatenated to get
            // the number of fields to skip.
            // in this way `uniq -1 -z -2` would be
            // equal to `uniq -12 -q`, since this behavior
            // is counterintuitive and it's hard to do in clap
            // we handle it more like GNU `fold`: we have a flag
            // for each possible initial digit, that takes the
            // rest of the value as argument.
            // we disallow explicitly multiple occurrences
            // because then it would have a different behavior
            // from GNU
            ArgGroup::new(options::OBSOLETE_SKIP_FIELDS)
                .multiple(false)
                .args(OBSOLETE_SKIP_FIELDS_DIGITS)
        )
        .arg(
            Arg::new(ARG_FILES)
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .num_args(0..=2)
                .value_hint(clap::ValueHint::FilePath),
        );

    for i in OBSOLETE_SKIP_FIELDS_DIGITS {
        cmd = cmd.arg(
            Arg::new(i)
                .short(i.chars().next().unwrap())
                .num_args(0..=1)
                .hide(true),
        );
    }

    cmd
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
