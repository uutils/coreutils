// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//

// spell-checker:ignore (ToDO) adFfmprt, kmerge

use clap::{Arg, ArgAction, ArgMatches, Command};
use itertools::Itertools;
use regex::Regex;
use std::ffi::OsStr;
use std::fs::{File, metadata};
use std::io::{BufRead, BufReader, BufWriter, Write, stderr, stdin, stdout};
use std::num::IntErrorKind;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::time::SystemTime;
use thiserror::Error;

use uucore::display::Quotable;
use uucore::error::{UResult, strip_errno};
use uucore::format_usage;
use uucore::time::{FormatSystemTimeFallback, format, format_system_time};
use uucore::translate;

const TAB: char = '\t';
const LINES_PER_PAGE: usize = 66;
const LINES_PER_PAGE_FOR_FORM_FEED: usize = 63;
const HEADER_LINES_PER_PAGE: usize = 5;
const TRAILER_LINES_PER_PAGE: usize = 5;
const FILE_STDIN: &str = "-";
const DEFAULT_COLUMN_WIDTH: usize = 72;
const DEFAULT_COLUMN_WIDTH_WITH_S_OPTION: usize = 512;
const DEFAULT_COLUMN_SEPARATOR: &char = &TAB;
const FF: u8 = 0x0C_u8;
const NL: u8 = b'\n';

mod options {
    pub const HEADER: &str = "header";
    pub const DATE_FORMAT: &str = "date-format";
    pub const DOUBLE_SPACE: &str = "double-space";
    pub const NUMBER_LINES: &str = "number-lines";
    pub const FIRST_LINE_NUMBER: &str = "first-line-number";
    pub const PAGES: &str = "pages";
    pub const OMIT_HEADER: &str = "omit-header";
    pub const OMIT_PAGINATION: &str = "omit-pagination";
    pub const PAGE_LENGTH: &str = "length";
    pub const NO_FILE_WARNINGS: &str = "no-file-warnings";
    pub const FORM_FEED: &str = "form-feed";
    pub const COLUMN_WIDTH: &str = "width";
    pub const PAGE_WIDTH: &str = "page-width";
    pub const ACROSS: &str = "across";
    pub const COLUMN_DOWN: &str = "column-down";
    pub const COLUMN: &str = "column";
    pub const COLUMN_CHAR_SEPARATOR: &str = "separator";
    pub const COLUMN_STRING_SEPARATOR: &str = "sep-string";
    pub const MERGE: &str = "merge";
    pub const INDENT: &str = "indent";
    pub const JOIN_LINES: &str = "join-lines";
    pub const HELP: &str = "help";
    pub const FILES: &str = "files";
    pub const EXPAND_TABS: &str = "expand-tabs";
}

struct OutputOptions {
    /// Line numbering mode
    number: Option<NumberingMode>,
    header: String,
    double_space: bool,
    line_separator: String,
    content_line_separator: String,
    last_modified_time: String,
    start_page: usize,
    end_page: Option<usize>,
    display_header_and_trailer: bool,
    content_lines_per_page: usize,
    page_separator_char: String,
    column_mode_options: Option<ColumnModeOptions>,
    merge_files_print: Option<usize>,
    offset_spaces: usize,
    form_feed_used: bool,
    join_lines: bool,
    col_sep_for_printing: String,
    line_width: Option<usize>,
    expand_tabs: Option<ExpandTabsOptions>,
}

/// One line of an input file, annotated with file, page, and line number.
#[derive(Default, Clone)]
struct FileLine {
    file_id: usize,
    page_number: usize,
    line_number: usize,
    line_content: Vec<u8>,
}

impl FileLine {
    fn from_buf(
        file_id: usize,
        page_number: usize,
        line_number: usize,
        buf: &[u8],
        options: &OutputOptions,
    ) -> Self {
        let line_content = if let Some(expand_tabs) = &options.expand_tabs {
            let mut result =
                Vec::with_capacity(buf.len() + buf.len() / 20 * expand_tabs.width as usize);
            for b in buf {
                apply_expand_tab(&mut result, *b, expand_tabs);
            }
            result
        } else {
            buf.to_vec()
        };

        Self {
            file_id,
            page_number,
            line_number,
            line_content,
        }
    }
}

struct ColumnModeOptions {
    width: usize,
    columns: usize,
    column_separator: String,
    across_mode: bool,
}

/// Line numbering mode
struct NumberingMode {
    width: usize,
    separator: String,
    first_number: usize,
}

#[derive(Debug)]
struct ExpandTabsOptions {
    input_char: char,
    width: i32,
}

impl Default for ExpandTabsOptions {
    fn default() -> Self {
        Self {
            width: 8,
            input_char: TAB,
        }
    }
}

impl Default for NumberingMode {
    fn default() -> Self {
        Self {
            width: 5,
            separator: TAB.to_string(),
            first_number: 1,
        }
    }
}

impl From<std::io::Error> for PrError {
    fn from(err: std::io::Error) -> Self {
        Self::EncounteredErrors {
            msg: err.to_string(),
        }
    }
}

impl From<FromUtf8Error> for PrError {
    fn from(err: FromUtf8Error) -> Self {
        Self::EncounteredErrors {
            msg: err.to_string(),
        }
    }
}

impl From<Utf8Error> for PrError {
    fn from(err: Utf8Error) -> Self {
        Self::EncounteredErrors {
            msg: err.to_string(),
        }
    }
}

#[derive(Debug, Error)]
enum PrError {
    #[error("pr: {msg}")]
    EncounteredErrors { msg: String },

    #[error("pr: {path}: {msg}")]
    ReadError { path: String, msg: String },
}

pub fn uu_app() -> Command {
    Command::new("pr")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("pr"))
        .about(translate!("pr-about"))
        .after_help(translate!("pr-after-help"))
        .override_usage(format_usage(&translate!("pr-usage")))
        .infer_long_args(true)
        .args_override_self(true)
        .disable_help_flag(true)
        .arg(
            Arg::new(options::PAGES)
                .long(options::PAGES)
                .help(translate!("pr-help-pages"))
                .value_name("FIRST_PAGE[:LAST_PAGE]"),
        )
        .arg(
            Arg::new(options::HEADER)
                .short('h')
                .long(options::HEADER)
                .help(translate!("pr-help-header"))
                .value_name("STRING"),
        )
        .arg(
            Arg::new(options::DATE_FORMAT)
                .short('D')
                .long(options::DATE_FORMAT)
                .value_name("FORMAT")
                .help(translate!("pr-help-date-format")),
        )
        .arg(
            Arg::new(options::DOUBLE_SPACE)
                .short('d')
                .long(options::DOUBLE_SPACE)
                .help(translate!("pr-help-double-space"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NUMBER_LINES)
                .short('n')
                .long(options::NUMBER_LINES)
                .help(translate!("pr-help-number-lines"))
                .allow_hyphen_values(true)
                .value_name("[char][width]"),
        )
        .arg(
            Arg::new(options::FIRST_LINE_NUMBER)
                .short('N')
                .long(options::FIRST_LINE_NUMBER)
                .help(translate!("pr-help-first-line-number"))
                .value_name("NUMBER"),
        )
        .arg(
            Arg::new(options::OMIT_HEADER)
                .short('t')
                .long(options::OMIT_HEADER)
                .help(translate!("pr-help-omit-header"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OMIT_PAGINATION)
                .short('T')
                .long(options::OMIT_PAGINATION)
                .help(translate!("pr-help-omit-pagination"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PAGE_LENGTH)
                .short('l')
                .long(options::PAGE_LENGTH)
                .help(translate!("pr-help-page-length"))
                .value_name("PAGE_LENGTH"),
        )
        .arg(
            Arg::new(options::NO_FILE_WARNINGS)
                .short('r')
                .long(options::NO_FILE_WARNINGS)
                .help(translate!("pr-help-no-file-warnings"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FORM_FEED)
                .short('F')
                .short_alias('f')
                .long(options::FORM_FEED)
                .help(translate!("pr-help-form-feed"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::COLUMN_WIDTH)
                .short('w')
                .long(options::COLUMN_WIDTH)
                .help(translate!("pr-help-column-width"))
                .value_name("width"),
        )
        .arg(
            Arg::new(options::PAGE_WIDTH)
                .short('W')
                .long(options::PAGE_WIDTH)
                .help(translate!("pr-help-page-width"))
                .value_name("width"),
        )
        .arg(
            Arg::new(options::ACROSS)
                .short('a')
                .long(options::ACROSS)
                .help(translate!("pr-help-across"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            // -b is a no-op for backwards compatibility (column-down is now the default)
            Arg::new(options::COLUMN_DOWN)
                .short('b')
                .hide(true)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::COLUMN)
                .long(options::COLUMN)
                .help(translate!("pr-help-column"))
                .value_name("column"),
        )
        .arg(
            Arg::new(options::COLUMN_CHAR_SEPARATOR)
                .short('s')
                .long(options::COLUMN_CHAR_SEPARATOR)
                .help(translate!("pr-help-column-char-separator"))
                .value_name("char")
                .num_args(0..=1)
                .default_missing_value("\t"),
        )
        .arg(
            Arg::new(options::COLUMN_STRING_SEPARATOR)
                .short('S')
                .long(options::COLUMN_STRING_SEPARATOR)
                .help(translate!("pr-help-column-string-separator"))
                .value_name("string")
                .num_args(0..=1)
                .default_missing_value(" "),
        )
        .arg(
            Arg::new(options::MERGE)
                .short('m')
                .long(options::MERGE)
                .help(translate!("pr-help-merge"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::INDENT)
                .short('o')
                .long(options::INDENT)
                .help(translate!("pr-help-indent"))
                .value_name("margin"),
        )
        .arg(
            Arg::new(options::JOIN_LINES)
                .short('J')
                .help(translate!("pr-help-join-lines"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help(translate!("pr-help-help"))
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new(options::FILES)
                .action(ArgAction::Append)
                .default_value(FILE_STDIN)
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::EXPAND_TABS)
                .long(options::EXPAND_TABS)
                .short('e')
                .num_args(1)
                .value_name("[CHAR][WIDTH]")
                .help(translate!("pr-help-expand-tabs")),
        )
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args.collect_ignore();

    let opt_args = recreate_arguments(&args);

    let command = uu_app();
    let matches = uucore::clap_localization::handle_clap_result(command, opt_args)?;

    #[allow(clippy::unwrap_used, reason = "default value is set by clap")]
    let files = matches
        .get_many::<String>(options::FILES)
        .map(|v| v.map(String::as_str).collect::<Vec<_>>())
        .unwrap();

    let file_groups: Vec<_> = if matches.get_flag(options::MERGE) {
        vec![files]
    } else {
        files.into_iter().map(|i| vec![i]).collect()
    };

    let operands = parse_column_page_operands(&args);

    for file_group in file_groups {
        let result_options = build_options(&matches, &file_group, &operands);
        let options = match result_options {
            Ok(options) => options,
            Err(err) => {
                print_error(&matches, &err);
                return Err(1.into());
            }
        };

        let cmd_result = file_group
            .iter()
            .exactly_one()
            .map_or_else(|_| mpr(&file_group, &options), |group| pr(group, &options));

        if let Err(e) = cmd_result {
            print_error(&matches, &e);
            return Err(1.into());
        }
    }
    Ok(())
}

/// Rewrite arguments before clap parsing, preserving legacy numeric operands.
fn recreate_arguments(args: &[String]) -> Vec<String> {
    let num_regex = Regex::new(r"^[^-]\d*$").unwrap();
    let n_regex = Regex::new(r"^-n\s*$").unwrap();
    let e_regex = Regex::new(r"^-e").unwrap();
    let mut arguments = args.to_owned();
    let num_option = args
        .iter()
        .take_while(|arg| arg.as_str() != "--")
        .find_position(|x| n_regex.is_match(x.trim()));
    if let Some((pos, _value)) = num_option
        && let Some(num_val_opt) = args.get(pos + 1)
        && !num_regex.is_match(num_val_opt)
    {
        let could_be_file = arguments.remove(pos + 1);
        arguments.insert(pos + 1, format!("{}", NumberingMode::default().width));
        arguments.insert(pos + 2, could_be_file);
    }

    // To ensure not to accidentally delete the next argument after a short flag for -e we insert
    // the default values for the -e flag is '-e' is present without direct arguments.
    let expand_tabs_option = arguments
        .iter()
        .take_while(|arg| arg.as_str() != "--")
        .find_position(|x| e_regex.is_match(x.trim()));
    if let Some((pos, value)) = expand_tabs_option
        && value.trim().len() <= 2
    {
        arguments[pos] = "-e\t8".to_string();
    }

    // Remove only whole-token legacy operands before clap parsing.
    let mut past_terminator = false;
    arguments
        .into_iter()
        .filter(|arg| {
            if past_terminator {
                return true;
            }
            if arg == "--" {
                past_terminator = true;
                return true;
            }
            as_column_operand(arg).is_none() && as_page_operand(arg).is_none()
        })
        .collect()
}

#[derive(Default)]
struct ColumnPageOperands {
    column: Option<String>,
    page: Option<String>,
}

/// Extract legacy `-COLUMN` and `+FIRST[:LAST]` operands before `--`.
fn parse_column_page_operands(args: &[String]) -> ColumnPageOperands {
    let mut operands = ColumnPageOperands::default();
    for arg in args {
        if arg == "--" {
            break;
        }
        if operands.column.is_none()
            && let Some(digits) = as_column_operand(arg)
        {
            operands.column = Some(digits.to_string());
            continue;
        }
        if operands.page.is_none()
            && let Some(spec) = as_page_operand(arg)
        {
            operands.page = Some(spec.to_string());
        }
    }
    operands
}

/// Return the digits from a whole-token `-COLUMN` operand.
fn as_column_operand(arg: &str) -> Option<&str> {
    arg.strip_prefix('-')
        .filter(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

/// Return the range from a whole-token `+FIRST[:LAST]` operand.
fn as_page_operand(arg: &str) -> Option<&str> {
    let spec = arg.strip_prefix('+')?;
    let is_digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let valid = match spec.split_once(':') {
        Some((first, last)) => is_digits(first) && is_digits(last),
        None => is_digits(spec),
    };
    valid.then_some(spec)
}

fn print_error(matches: &ArgMatches, err: &PrError) {
    if !matches.get_flag(options::NO_FILE_WARNINGS) {
        let _ = writeln!(stderr(), "{err}");
    }
}

fn parse_usize(matches: &ArgMatches, opt: &str) -> Option<Result<usize, PrError>> {
    let from_parse_error_to_pr_error = |value_to_parse: (String, String)| {
        let i = value_to_parse.0;
        let option = value_to_parse.1;
        i.parse().map_err(|_e| PrError::EncounteredErrors {
            msg: format!("invalid -{option} argument {}", i.quote()),
        })
    };
    matches
        .get_one::<String>(opt)
        .map(|i| (i.to_owned(), format!("-{opt}")))
        .map(from_parse_error_to_pr_error)
}

fn get_date_format(matches: &ArgMatches) -> String {
    match matches.get_one::<String>(options::DATE_FORMAT) {
        Some(format) => format,
        None => {
            // Replicate behavior from GNU manual.
            if std::env::var("POSIXLY_CORRECT").is_ok()
                // TODO: This needs to be moved to uucore and handled by icu?
                && (std::env::var_os("LC_TIME").as_deref() == Some(OsStr::new("POSIX"))
                    || std::env::var_os("LC_ALL").as_deref() == Some(OsStr::new("POSIX")))
            {
                "%b %e %H:%M %Y"
            } else {
                format::LONG_ISO
            }
        }
    }
    .to_string()
}

#[allow(clippy::cognitive_complexity)]
fn build_options(
    matches: &ArgMatches,
    paths: &[&str],
    operands: &ColumnPageOperands,
) -> Result<OutputOptions, PrError> {
    let form_feed_used = matches.get_flag(options::FORM_FEED);

    let is_merge_mode = matches.get_flag(options::MERGE);

    if is_merge_mode && matches.contains_id(options::COLUMN) {
        return Err(PrError::EncounteredErrors {
            msg: translate!("pr-error-column-merge-conflict"),
        });
    }

    if is_merge_mode && matches.get_flag(options::ACROSS) {
        return Err(PrError::EncounteredErrors {
            msg: translate!("pr-error-across-merge-conflict"),
        });
    }

    let merge_files_print = if matches.get_flag(options::MERGE) {
        Some(paths.len())
    } else {
        None
    };

    let header = matches
        .get_one::<String>(options::HEADER)
        .map_or(
            if is_merge_mode || paths[0] == FILE_STDIN {
                ""
            } else {
                paths[0]
            },
            |s| s.as_str(),
        )
        .to_string();

    let default_first_number = NumberingMode::default().first_number;
    let first_number =
        parse_usize(matches, options::FIRST_LINE_NUMBER).unwrap_or(Ok(default_first_number))?;

    let number = matches
        .get_one::<String>(options::NUMBER_LINES)
        .map(|i| {
            let invalid = |arg: &str| PrError::EncounteredErrors {
                msg: format!(
                    "{}\n{}",
                    translate!("pr-error-invalid-number-argument", "arg" => arg),
                    translate!("pr-try-help-message")
                ),
            };

            let parse_result = i.parse::<usize>();

            let separator = if parse_result.is_err() {
                match i.chars().next() {
                    Some(c) if c.is_ascii() => c.to_string(),
                    Some(_) | None => return Err(invalid(i)),
                }
            } else {
                NumberingMode::default().separator
            };

            let width = match parse_result {
                Ok(res) => res,
                Err(_) => i
                    .get(1..)
                    .unwrap_or_default()
                    .parse::<usize>()
                    .unwrap_or(NumberingMode::default().width),
            };

            Ok(NumberingMode {
                width,
                separator,
                first_number,
            })
        })
        .transpose()?
        .or_else(|| {
            if matches.contains_id(options::NUMBER_LINES) {
                Some(NumberingMode::default())
            } else {
                None
            }
        });

    let expand_tabs = matches
        .get_one::<String>(options::EXPAND_TABS)
        .map(|s| {
            s.chars()
                .next()
                .map_or(Ok(ExpandTabsOptions::default()), |c| {
                    let invalid = |arg: &str| PrError::EncounteredErrors {
                        msg: format!(
                            "{}\n{}",
                            translate!("pr-error-invalid-expand-tab-argument", "arg" => arg),
                            translate!("pr-try-help-message")
                        ),
                    };
                    if c.is_ascii_digit() {
                        let width: i32 = s.parse().map_err(|_e| invalid(s))?;
                        if width <= 0 {
                            return Err(invalid(s));
                        }
                        Ok(ExpandTabsOptions {
                            input_char: TAB,
                            width,
                        })
                    } else if !c.is_ascii() {
                        Err(invalid(s))
                    } else if s.len() > 1 {
                        let width: i32 = s[1..].parse().map_err(|_e| invalid(&s[1..]))?;
                        if width <= 0 {
                            return Err(invalid(&s[1..]));
                        } else if s.starts_with('-') {
                            return Err(invalid(s));
                        }
                        Ok(ExpandTabsOptions {
                            input_char: c,
                            width,
                        })
                    } else {
                        Ok(ExpandTabsOptions {
                            input_char: c,
                            width: 8,
                        })
                    }
                })
        })
        .transpose()?;

    let double_space = matches.get_flag(options::DOUBLE_SPACE);

    let content_line_separator = if double_space {
        "\n".repeat(2)
    } else {
        "\n".to_string()
    };

    let line_separator = "\n".to_string();

    let last_modified_time = {
        let time = if is_merge_mode || paths[0].eq(FILE_STDIN) {
            Some(SystemTime::now())
        } else {
            metadata(paths.first().unwrap())
                .ok()
                .and_then(|i| i.modified().ok())
        };
        time.and_then(|time| {
            let mut v = Vec::new();
            format_system_time(
                &mut v,
                time,
                &get_date_format(matches),
                FormatSystemTimeFallback::Integer,
            )
            .ok()
            .map(|()| String::from_utf8_lossy(&v).to_string())
        })
        .unwrap_or_default()
    };

    // +page option is less priority than --pages
    let plus_page = operands.page.as_deref();
    let res = plus_page.map(|unparsed_num| {
        let x: Vec<_> = unparsed_num.split(':').collect();
        x[0].to_string()
            .parse::<usize>()
            .map_err(|_e| PrError::EncounteredErrors {
                msg: format!("invalid {} argument {}", "+", unparsed_num.quote()),
            })
    });
    let start_page_in_plus_option = match res {
        Some(res) => res?,
        None => 1,
    };

    let res = plus_page.filter(|i| i.contains(':')).map(|unparsed_num| {
        let x: Vec<_> = unparsed_num.split(':').collect();
        x[1].to_string()
            .parse::<usize>()
            .map_err(|_e| PrError::EncounteredErrors {
                msg: format!("invalid {} argument {}", "+", unparsed_num.quote()),
            })
    });
    let end_page_in_plus_option = match res {
        Some(res) => Some(res?),
        None => None,
    };

    let invalid_pages_map = |i: String| {
        let unparsed_value = matches.get_one::<String>(options::PAGES).unwrap();
        let parsed_value = i.parse::<usize>().map_err(|_e| PrError::EncounteredErrors {
            msg: format!("invalid --pages argument {}", unparsed_value.quote()),
        });

        match parsed_value {
            Ok(0) => Err(PrError::EncounteredErrors {
                msg: "invalid --pages argument '0'".to_string(),
            }),
            Ok(res) => Ok(res),
            Err(e) => Err(e),
        }
    };

    let res = matches
        .get_one::<String>(options::PAGES)
        .map(|i| {
            let x: Vec<_> = i.split(':').collect();
            x[0].to_string()
        })
        .map(invalid_pages_map);
    let start_page = match res {
        Some(res) => res?,
        None => start_page_in_plus_option,
    };

    let res = matches
        .get_one::<String>(options::PAGES)
        .filter(|i| i.contains(':'))
        .map(|i| {
            let x: Vec<_> = i.split(':').collect();
            x[1].to_string()
        })
        .map(invalid_pages_map);
    let end_page = match res {
        Some(res) => Some(res?),
        None => end_page_in_plus_option,
    };

    if let Some(end_page) = end_page.filter(|end| start_page > *end) {
        return Err(PrError::EncounteredErrors {
            msg: translate!("pr-error-invalid-pages-range", "start" => start_page, "end" => end_page),
        });
    }

    let default_lines_per_page = if form_feed_used {
        LINES_PER_PAGE_FOR_FORM_FEED
    } else {
        LINES_PER_PAGE
    };

    let page_length =
        parse_usize(matches, options::PAGE_LENGTH).unwrap_or(Ok(default_lines_per_page))?;

    if page_length == 0 {
        return Err(PrError::EncounteredErrors {
            msg: "invalid --length argument '0'".to_string(),
        });
    }

    let page_length_le_ht = page_length < (HEADER_LINES_PER_PAGE + TRAILER_LINES_PER_PAGE);

    let display_header_and_trailer = !page_length_le_ht
        && !matches.get_flag(options::OMIT_HEADER)
        && !matches.get_flag(options::OMIT_PAGINATION);

    let content_lines_per_page = if page_length_le_ht {
        page_length
    } else {
        page_length - (HEADER_LINES_PER_PAGE + TRAILER_LINES_PER_PAGE)
    };

    let page_separator_char = if matches.get_flag(options::FORM_FEED) {
        let bytes = vec![FF];
        String::from_utf8(bytes).unwrap()
    } else {
        "\n".to_string()
    };

    let across_mode = matches.get_flag(options::ACROSS);

    let column_separator = match matches.get_one::<String>(options::COLUMN_STRING_SEPARATOR) {
        Some(x) => Some(x),
        None => matches.get_one::<String>(options::COLUMN_CHAR_SEPARATOR),
    }
    .map_or_else(|| DEFAULT_COLUMN_SEPARATOR.to_string(), ToString::to_string);

    let default_column_width = if matches.contains_id(options::COLUMN_WIDTH)
        && matches.contains_id(options::COLUMN_CHAR_SEPARATOR)
    {
        DEFAULT_COLUMN_WIDTH_WITH_S_OPTION
    } else {
        DEFAULT_COLUMN_WIDTH
    };

    let column_width =
        parse_usize(matches, options::COLUMN_WIDTH).unwrap_or(Ok(default_column_width))?;

    if column_width == 0 {
        return Err(PrError::EncounteredErrors {
            msg: "invalid --width argument '0'".to_string(),
        });
    }

    let page_width = if matches.get_flag(options::JOIN_LINES) {
        None
    } else {
        match parse_usize(matches, options::PAGE_WIDTH) {
            Some(res) => Some(res?),
            None => None,
        }
    };

    if page_width == Some(0) {
        return Err(PrError::EncounteredErrors {
            msg: "invalid --page-width argument '0'".to_string(),
        });
    }

    let res = operands.column.as_deref().map(|unparsed_num| {
        unparsed_num
            .parse::<usize>()
            .map_err(|_e| PrError::EncounteredErrors {
                msg: format!("invalid {} argument {}", "-", unparsed_num.quote()),
            })
    });
    let start_column_option = match res {
        Some(Ok(0)) => {
            return Err(PrError::EncounteredErrors {
                msg: "invalid --column argument '0'".to_string(),
            });
        }
        Some(res) => Some(res?),
        None => None,
    };

    // --column has more priority than -column

    let column_option_value = match parse_usize(matches, options::COLUMN) {
        Some(Ok(0)) => {
            return Err(PrError::EncounteredErrors {
                msg: "invalid --column argument '0'".to_string(),
            });
        }
        Some(res) => Some(res?),
        None => start_column_option,
    };

    let column_mode_options = column_option_value.map(|columns| ColumnModeOptions {
        columns,
        width: column_width,
        column_separator,
        across_mode,
    });

    let offset_spaces = match matches.get_one::<String>(options::INDENT) {
        None => 0,
        // Parse as i32 to match GNU pr's behavior
        // Store the count. Spaces are streamed at print time to avoid huge allocations.
        Some(raw) => match raw.parse::<i32>() {
            Ok(n) if n >= 0 => n as usize,
            Err(e) if matches!(e.kind(), IntErrorKind::PosOverflow) => {
                return Err(PrError::EncounteredErrors {
                    msg: format!(
                        "'-o MARGIN' invalid line offset: {}: Value too large for defined data type",
                        raw.quote()
                    ),
                });
            }
            _ => {
                return Err(PrError::EncounteredErrors {
                    msg: format!("'-o MARGIN' invalid line offset: {}", raw.quote()),
                });
            }
        },
    };

    let join_lines = matches.get_flag(options::JOIN_LINES);

    let col_sep_for_printing = column_mode_options.as_ref().map_or_else(
        || {
            merge_files_print
                .map(|_k| DEFAULT_COLUMN_SEPARATOR.to_string())
                .unwrap_or_default()
        },
        |i| i.column_separator.clone(),
    );

    let columns_to_print =
        merge_files_print.unwrap_or_else(|| column_mode_options.as_ref().map_or(1, |i| i.columns));

    let line_width = if join_lines {
        None
    } else if columns_to_print > 1 {
        Some(
            column_mode_options
                .as_ref()
                .map_or(DEFAULT_COLUMN_WIDTH, |i| i.width),
        )
    } else {
        page_width
    };

    Ok(OutputOptions {
        number,
        header,
        double_space,
        line_separator,
        content_line_separator,
        last_modified_time,
        start_page,
        end_page,
        display_header_and_trailer,
        content_lines_per_page,
        page_separator_char,
        column_mode_options,
        merge_files_print,
        offset_spaces,
        form_feed_used,
        join_lines,
        col_sep_for_printing,
        line_width,
        expand_tabs,
    })
}

/// Open a file (or stdin) for reading.
///
/// If `path` is `"-"`, then read from stdin. The returned `BufRead` allows
/// streaming one page at a time to keep memory bounded on large inputs.
fn read_to_end(path: &str) -> Result<Box<dyn BufRead>, PrError> {
    if path == "-" {
        Ok(Box::new(stdin().lock()))
    } else {
        File::open(path).map(|f| Box::new(BufReader::new(f)) as Box<dyn BufRead>).map_err(|err| {
            PrError::ReadError {
                path: path.to_string(),
                msg: strip_errno(&err),
            }
        })
    }
}

fn apply_expand_tab(chunk: &mut Vec<u8>, byte: u8, expand_options: &ExpandTabsOptions) {
    if byte == expand_options.input_char as u8 {
        // If the byte encountered is the input char we use width to calculate
        // the amount of spaces needed (if no input char given we stored '\t'
        // in our struct)
        let spaces_needed =
            expand_options.width as usize - (chunk.len() % expand_options.width as usize);
        chunk.extend(std::iter::repeat_n(b' ', spaces_needed));
    } else if byte == TAB as u8 {
        // If a byte got passed to the -e flag (eg -ea1)  which is not '\t' GNU
        // still expands it but does not use an optionally given width parameter
        // but does the '\t' expansion with the default value (8)
        let spaces_needed = 8 - (chunk.len() % 8);
        chunk.extend(std::iter::repeat_n(b' ', spaces_needed));
    } else {
        // This arm means the byte is neither '\t' nor the bytes to be
        // expanded
        chunk.push(byte);
    }
}

/// Format a single file for printing (`pr` without `-m`).
///
/// Streams the input: reads chunks via `read_chunk`, builds one page at a
/// time, prints it immediately, and discards it. Never holds more than one
/// page in memory, which prevents OOM on large or infinite inputs.
fn pr(path: &str, options: &OutputOptions) -> Result<i32, PrError> {
    let mut reader = read_to_end(path)?;

    let start_page = options.start_page;
    let end_page = options.end_page;
    let lines_needed_per_page = lines_to_read_for_page(options);
    let mut line_num = get_start_line_number(options);
    let mut page_num = 0;
    let mut prev_sep: Option<u8> = None;

    loop {
        // Skip pages before start_page without allocating FileLine objects.
        // This avoids wasting memory when piping infinite output through
        // `pr --start-page=N --end-page=M`.
        if page_num + 1 < start_page {
            match read_one_page(
                &mut reader,
                options,
                0,
                &mut prev_sep,
                &mut line_num,
                &mut page_num,
                lines_needed_per_page,
                false,
            )? {
                None => return Ok(0),
                Some(_) => continue,
            }
        }

        let mut page = vec![];

        // Read chunks from the file and accumulate lines until a page
        // boundary (form feed or lines_needed_per_page), then print.
        loop {
            let (line_content, sep) = read_chunk(&mut reader)?;

            match sep {
                None => {
                    // EOF: trailing data
                    if !line_content.is_empty() {
                        page.push(FileLine::from_buf(
                            0,
                            page_num,
                            line_num,
                            &line_content,
                            options,
                        ));
                    }
                    if !page.is_empty()
                        && start_page <= page_num + 1
                        && end_page.is_none_or(|e| page_num < e)
                    {
                        let page_number = page_num + 1;
                        print_page(&page, options, page_number)?;
                    }
                    return Ok(0);
                }
                Some(FF) => {
                    if !(prev_sep == Some(NL) && line_content.is_empty()) {
                        page.push(FileLine::from_buf(
                            0,
                            page_num,
                            line_num,
                            &line_content,
                            options,
                        ));
                    }
                    if start_page <= page_num + 1
                        && end_page.is_none_or(|e| page_num < e)
                    {
                        let page_number = page_num + 1;
                        print_page(&page, options, page_number)?;
                    }
                    page_num += 1;
                    page.clear();
                    if end_page.is_some_and(|e| page_num >= e) {
                        return Ok(0);
                    }
                    prev_sep = Some(FF);
                    break;
                }
                _ => {
                    // NL
                    if !(prev_sep == Some(FF) && line_content.is_empty()) {
                        page.push(FileLine::from_buf(
                            0,
                            page_num,
                            line_num,
                            &line_content,
                            options,
                        ));
                        line_num += 1;
                    }
                    if page.len() >= lines_needed_per_page {
                        if start_page <= page_num + 1
                            && end_page.is_none_or(|e| page_num < e)
                        {
                            let page_number = page_num + 1;
                            print_page(&page, options, page_number)?;
                        }
                        page_num += 1;
                        page.clear();
                        if end_page.is_some_and(|e| page_num >= e) {
                            return Ok(0);
                        }
                        break;
                    }
                    prev_sep = Some(NL);
                }
            }
        }
    }
}

/// Maximum size of a single chunk read from input.
///
/// This prevents unbounded memory accumulation when the input has no
/// newline or form-feed characters (e.g. `/dev/zero`, `/dev/urandom`).
const MAX_CHUNK_SIZE: usize = 256 * 1024;

/// Read the next chunk of data up to (but not including) a FF or NL separator.
///
/// Returns the data before the separator and the separator byte that ended it,
/// or `None` for EOF. If no separator is found within `MAX_CHUNK_SIZE` bytes,
/// the data is returned with an implicit NL separator to keep memory bounded.
fn read_chunk(reader: &mut dyn BufRead) -> std::io::Result<(Vec<u8>, Option<u8>)> {
    let mut buf = Vec::new();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok((buf, None));
        }
        match memchr::memchr2(FF, NL, available) {
            Some(pos) => {
                // Found a separator: take everything before it, consume the
                // separator byte from the reader, and return.
                buf.extend_from_slice(&available[..pos]);
                let sep = available[pos];
                reader.consume(pos + 1);
                return Ok((buf, Some(sep)));
            }
            None => {
                // No separator in the reader's buffer: consume it all and
                // either read more (below 256KB) or force-split.
                buf.extend_from_slice(available);
                let len = available.len();
                reader.consume(len);
                if buf.len() >= MAX_CHUNK_SIZE {
                    // Safety limit: if we've accumulated 256KB without finding
                    // any separator, insert an artificial newline. This prevents
                    // unbounded memory growth on inputs without newlines or
                    // form feeds (e.g. /dev/zero, /dev/urandom).
                    return Ok((buf, Some(NL)));
                }
            }
        }
    }
}

/// Read one page from a BufReader.
///
/// Call this function repeatedly to stream pages one at a time instead of
/// accumulating all pages into memory at once. Returns `Ok(Some(lines))` when
/// a page boundary is reached, or `Ok(None)` at EOF.
///
/// When `track_content` is `false`, lines are skipped without allocation
/// (only counters advance). This is used to efficiently advance past pages
/// that fall outside the `start_page`..`end_page` range.
///
/// A page boundary is triggered by either:
///   - A form feed (`\f`) character — acts as an explicit page break
///   - Reaching `lines_needed_per_page` lines — fills a page to its capacity
///
/// State variables (`prev_sep`, `line_num`, `page_num`) are updated in place
/// so the caller can continue reading the next page from where this one left
/// off.
fn read_one_page(
    reader: &mut dyn BufRead,
    options: &OutputOptions,
    file_id: usize,
    prev_sep: &mut Option<u8>,
    line_num: &mut usize,
    page_num: &mut usize,
    lines_needed_per_page: usize,
    track_content: bool,
) -> Result<Option<Vec<FileLine>>, PrError> {
    let mut page = vec![];
    let mut page_lines_needed = lines_needed_per_page;

    loop {
        let (line_content, sep) = read_chunk(reader)?;

        match sep {
            None => {
                if track_content && !line_content.is_empty() {
                    page.push(FileLine::from_buf(
                        file_id, *page_num, *line_num, &line_content, options,
                    ));
                }
                if page.is_empty() {
                    return Ok(None);
                }
                return Ok(Some(page));
            }
            Some(ch) if ch == FF => {
                if track_content && !(*prev_sep == Some(NL) && line_content.is_empty()) {
                    page.push(FileLine::from_buf(
                        file_id, *page_num, *line_num, &line_content, options,
                    ));
                }
                let result = Some(page);
                *page_num += 1;
                *prev_sep = Some(FF);
                return Ok(result);
            }
            _ => {
                let is_data_line = !(*prev_sep == Some(FF) && line_content.is_empty());
                if track_content && is_data_line {
                    page.push(FileLine::from_buf(
                        file_id, *page_num, *line_num, &line_content, options,
                    ));
                }
                if is_data_line {
                    *line_num += 1;
                }
                if is_data_line {
                    page_lines_needed = page_lines_needed.saturating_sub(1);
                }
                if page_lines_needed == 0 {
                    let result = Some(page);
                    *page_num += 1;
                    *prev_sep = Some(NL);
                    return Ok(result);
                }
                *prev_sep = Some(NL);
            }
        }
    }
}

/// Merge-print multiple files side by side in columns (`pr -m`).
///
/// Instead of reading all files completely before merging, this streams one
/// page at a time from each file. For each output page number (1-based):
///   1. Advance each file past any pages before this output page
///   2. Read the matching page from each file that has one
///   3. Merge the lines into columns and print
///   4. Discard the page and continue
///
/// This keeps memory proportional to one merged page rather than the total
/// size of all input files.
fn mpr(paths: &[&str], options: &OutputOptions) -> Result<i32, PrError> {
    let lines_needed_per_page = lines_to_read_for_page(options);
    let start_page = options.start_page;
    let end_page = options.end_page;

    // Per-file state for streaming: reader position, current page number, and
    // line counter. `done` is set to true when the file is fully consumed.
    struct FileState {
        reader: Box<dyn BufRead>,
        file_id: usize,
        prev_sep: Option<u8>,
        line_num: usize,
        page_num: usize,
        done: bool,
    }

    // Open all input files and initialize their streaming state.
    let mut files: Vec<FileState> = Vec::new();
    for (file_id, path) in paths.iter().enumerate() {
        let reader = read_to_end(path)?;
        files.push(FileState {
            reader,
            file_id,
            prev_sep: None,
            line_num: get_start_line_number(options),
            page_num: 0,
            done: false,
        });
    }

    let columns = options.merge_files_print.unwrap_or(1);
    let mut col_counts = Vec::with_capacity(columns);
    let mut col_starts = Vec::with_capacity(columns);

    let mut any_active = true;
    let mut output_page = 1;
    let mut lines = Vec::new();

    let page_separator = options.page_separator_char.as_bytes();
    let print_header = options.display_header_and_trailer;

    // Outer loop: iterate through output pages. At each iteration we collect
    // lines from all files that have a page for the current output page.
    while any_active {
        any_active = false;
        lines.clear();

        // Inner loop over files: advance each file to the current output
        // page, then read its contribution if it has one.
        for file in files.iter_mut() {
            if file.done {
                continue;
            }
            any_active = true;

            // Advance the file past any pages numbered lower than the current
            // output page (read and discard without allocating FileLine
            // objects). This happens when a file has fewer lines per page and
            // ran ahead of the output page counter.
            while file.page_num + 1 < output_page {
                match read_one_page(
                    &mut file.reader,
                    options,
                    file.file_id,
                    &mut file.prev_sep,
                    &mut file.line_num,
                    &mut file.page_num,
                    lines_needed_per_page,
                    false,
                )? {
                    None => {
                        file.done = true;
                        break;
                    }
                    Some(_) => {}
                }
            }

            if file.done {
                continue;
            }

            // Read the page that matches the current output page number.
            // Track content (allocate FileLines) only when this page falls
            // within the printable range; otherwise just advance counters.
            if file.page_num + 1 == output_page {
                let track = output_page >= start_page;
                match read_one_page(
                    &mut file.reader,
                    options,
                    file.file_id,
                    &mut file.prev_sep,
                    &mut file.line_num,
                    &mut file.page_num,
                    lines_needed_per_page,
                    track,
                )? {
                    None => {
                        file.done = true;
                    }
                    Some(page_lines) => {
                        lines.extend(page_lines);
                    }
                }
            }
        }

        // Only print if this page falls within the user's page range.
        // Lines are already in file_id order (files are iterated in order)
        // and line_number order (each page appends lines in sequence).
        if !lines.is_empty()
            && output_page >= start_page
            && end_page.is_none_or(|e| output_page <= e)
        {
            let out = stdout();
            let mut out = BufWriter::new(out.lock());

            if print_header {
                for x in header_content(options, output_page) {
                    out.write_all(x.as_bytes())?;
                    out.write_all(b"\n")?;
                }
            }

            write_merge_page(&lines, options, &mut out, &mut col_counts, &mut col_starts)?;

            if print_header {
                let trailer = trailer_content(options);
                for (index, x) in trailer.iter().enumerate() {
                    out.write_all(x.as_bytes())?;
                    if index + 1 != trailer.len() {
                        out.write_all(b"\n")?;
                    }
                }
            }
            out.write_all(page_separator)?;
        }

        output_page += 1;
        if end_page.is_some_and(|e| output_page > e) {
            break;
        }
    }

    Ok(0)
}

/// Write one page of merge-mode output, using pre-allocated column-count and
/// column-start buffers to avoid per-page Vec allocations. Each cell is written
/// directly to `out` without intermediate String allocations.
fn write_merge_page(
    lines: &[FileLine],
    options: &OutputOptions,
    out: &mut impl Write,
    col_counts: &mut Vec<usize>,
    col_starts: &mut Vec<usize>,
) -> Result<(), std::io::Error> {
    let columns = options.merge_files_print.unwrap_or(1);
    let line_separator = options.content_line_separator.as_bytes();
    let line_width = options.line_width;

    let content_lines_per_page = if options.double_space {
        options.content_lines_per_page / 2
    } else {
        options.content_lines_per_page
    };

    col_counts.clear();
    col_counts.resize(columns, 0);
    for line in lines.iter() {
        if line.file_id < columns {
            col_counts[line.file_id] += 1;
        }
    }

    col_starts.clear();
    col_starts.resize(columns, 0);
    for j in 1..columns {
        col_starts[j] = col_starts[j - 1] + col_counts[j - 1];
    }

    let blank_line = FileLine::default();
    let min_width = line_width.map(|lw| (lw - (columns - 1)) / columns);

    for i in 0..content_lines_per_page {
        for j in 0..columns {
            let cell = if i < col_counts[j] {
                Some(&lines[col_starts[j] + i])
            } else {
                None
            };
            let line_to_print = cell.unwrap_or(&blank_line);

            write_offset_spaces(out, options.offset_spaces)?;

            let formatted_number = get_formatted_line_number(options, line_to_print.line_number, j);
            out.write_all(formatted_number.as_bytes())?;

            let content = &line_to_print.line_content;
            if let Some(mw) = min_width {
                let fn_tabs = formatted_number.bytes().filter(|&b| b == b'\t').count();
                let fn_width = formatted_number.len() + fn_tabs * 7;
                if fn_width < mw {
                    let content_max = mw - fn_width;
                    let content_tabs = content.iter().filter(|&&b| b == b'\t').count();
                    let content_width = content.len() + content_tabs * 7;
                    if content_width <= content_max {
                        out.write_all(content)?;
                        let padding = content_max - content_width;
                        if padding > 0 {
                            write_offset_spaces(out, padding)?;
                        }
                    } else {
                        write_truncated_bytes(out, content, content_max)?;
                    }
                }
            } else {
                out.write_all(content)?;
            }

            if (j + 1) != columns && !options.join_lines {
                out.write_all(options.col_sep_for_printing.as_bytes())?;
            }
        }
        out.write_all(line_separator)?;
    }

    Ok(())
}

/// Write up to `max_chars` Unicode characters from `content` to `out` without
/// allocating (fast path for ASCII). Falls back to char-boundary iteration for
/// multi-byte UTF-8.
fn write_truncated_bytes(
    out: &mut impl Write,
    content: &[u8],
    max_chars: usize,
) -> Result<(), std::io::Error> {
    if content.len() <= max_chars {
        return out.write_all(content);
    }
    if content.iter().all(|&b| b.is_ascii()) {
        return out.write_all(&content[..max_chars]);
    }
    match std::str::from_utf8(content) {
        Ok(s) => {
            let end = s
                .char_indices()
                .take(max_chars)
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(0);
            out.write_all(&content[..end])
        }
        Err(_) => out.write_all(&content[..content.len().min(max_chars)]),
    }
}

fn print_page(
    lines: &[FileLine],
    options: &OutputOptions,
    page: usize,
) -> Result<(), std::io::Error> {
    let line_separator = options.line_separator.as_bytes();
    let page_separator = options.page_separator_char.as_bytes();

    let header = header_content(options, page);
    let trailer_content = trailer_content(options);

    let out = stdout();
    let mut out = BufWriter::new(out.lock());

    for x in header {
        out.write_all(x.as_bytes())?;
        out.write_all(line_separator)?;
    }

    write_columns(lines, options, &mut out)?;

    for (index, x) in trailer_content.iter().enumerate() {
        out.write_all(x.as_bytes())?;
        if index + 1 != trailer_content.len() {
            out.write_all(line_separator)?;
        }
    }
    out.write_all(page_separator)?;
    Ok(())
}

/// Group the lines of the input file in columns read left-to-right.
fn to_table_across(
    content_lines_per_page: usize,
    columns: usize,
    lines: &[FileLine],
) -> Vec<Vec<Option<&FileLine>>> {
    (0..content_lines_per_page)
        .map(|i| (0..columns).map(|j| lines.get(i * columns + j)).collect())
        .collect()
}

/// Group lines of the file in columns, going top-to-bottom then left-to-right.
///
/// This function should be applied when there are more lines than the
/// total number of cells in the table.
fn to_table(
    content_lines_per_page: usize,
    columns: usize,
    lines: &[FileLine],
) -> Vec<Vec<Option<&FileLine>>> {
    (0..content_lines_per_page)
        .map(|i| {
            (0..columns)
                .map(|j| lines.get(content_lines_per_page * j + i))
                .collect()
        })
        .collect()
}

/// Group lines of the file in columns, going top-to-bottom then left-to-right.
///
/// This function should be applied when there are fewer lines than the
/// total number of cells in the table.
fn to_table_short_file(
    content_lines_per_page: usize,
    columns: usize,
    lines: &[FileLine],
) -> Vec<Vec<Option<&FileLine>>> {
    let num_rows = lines.len() / columns;
    let mut table: Vec<Vec<_>> = (0..num_rows)
        .map(|i| (0..columns).map(|j| lines.get(num_rows * j + i)).collect())
        .collect();
    // Fill the rest with Nones.
    for _ in num_rows..content_lines_per_page {
        table.push(vec![None; columns]);
    }
    table
}

/// Write `n` space characters to `out` in fixed-size chunks, so the indent is
/// streamed rather than allocated up front.
fn write_offset_spaces(out: &mut impl Write, mut n: usize) -> Result<(), std::io::Error> {
    const SPACES: [u8; 256] = [b' '; 256];
    while n > 0 {
        let chunk = n.min(SPACES.len());
        out.write_all(&SPACES[..chunk])?;
        n -= chunk;
    }
    Ok(())
}

#[allow(clippy::cognitive_complexity)]
fn write_columns(
    lines: &[FileLine],
    options: &OutputOptions,
    out: &mut impl Write,
) -> Result<(), std::io::Error> {
    let line_separator = options.content_line_separator.as_bytes();

    let content_lines_per_page = if options.double_space {
        options.content_lines_per_page / 2
    } else {
        options.content_lines_per_page
    };

    let columns = options
        .merge_files_print
        .unwrap_or_else(|| get_columns(options));
    let line_width = options.line_width;
    let feed_line_present = options.form_feed_used;
    let mut not_found_break = false;

    let across_mode = options
        .column_mode_options
        .as_ref()
        .is_some_and(|i| i.across_mode);

    // Group the flat list of lines into a 2-dimensional table of
    // cells, where each row will be printed as a single line in the
    // output.
    let table = if lines.len() < (content_lines_per_page * columns) {
        to_table_short_file(content_lines_per_page, columns, lines)
    } else if across_mode {
        to_table_across(content_lines_per_page, columns, lines)
    } else {
        to_table(content_lines_per_page, columns, lines)
    };

    for row in table {
        let indexes = row.len();
        for (i, cell) in row.iter().enumerate() {
            let line_to_print = match cell {
                None => {
                    not_found_break = true;
                    break;
                }
                Some(file_line) => file_line,
            };

            write_offset_spaces(out, options.offset_spaces)?;
            out.write_all(
                get_line_for_printing(options, line_to_print, columns, i, line_width, indexes)
                    .as_bytes(),
            )?;
        }
        if not_found_break && feed_line_present {
            break;
        }
        out.write_all(line_separator)?;
    }

    Ok(())
}

fn get_line_for_printing(
    options: &OutputOptions,
    file_line: &FileLine,
    columns: usize,
    index: usize,
    line_width: Option<usize>,
    indexes: usize,
) -> String {
    let blank_line = String::new();
    let formatted_line_number = get_formatted_line_number(options, file_line.line_number, index);

    // TODO: support non-UTF-8 bytes (currently replaced with U+FFFD)
    let content = String::from_utf8_lossy(&file_line.line_content);
    let mut complete_line = format!("{formatted_line_number}{content}");

    let tab_count = complete_line.chars().filter(|i| i == &TAB).count();

    let display_length = complete_line.len() + (tab_count * 7);

    let sep = if (index + 1) != indexes && !options.join_lines {
        &options.col_sep_for_printing
    } else {
        &blank_line
    };

    format!(
        "{}{sep}",
        line_width
            .map(|i| {
                let min_width = (i - (columns - 1)) / columns;
                if display_length < min_width {
                    for _i in 0..(min_width - display_length) {
                        complete_line.push(' ');
                    }
                }

                complete_line.chars().take(min_width).collect()
            })
            .unwrap_or(complete_line),
    )
}

fn get_formatted_line_number(opts: &OutputOptions, line_number: usize, index: usize) -> String {
    let should_show_line_number =
        opts.number.is_some() && (opts.merge_files_print.is_none() || index == 0);
    if should_show_line_number && line_number != 0 {
        let line_str = line_number.to_string();
        let num_opt = opts.number.as_ref().unwrap();
        let width = num_opt.width;
        let separator = &num_opt.separator;
        if line_str.len() >= width {
            format!("{:>width$}{separator}", &line_str[line_str.len() - width..])
        } else {
            format!("{line_str:>width$}{separator}")
        }
    } else {
        String::new()
    }
}

/// Returns a five line header content if displaying header is not disabled by
/// using `NO_HEADER_TRAILER_OPTION` option.
fn header_content(options: &OutputOptions, page: usize) -> Vec<String> {
    if !options.display_header_and_trailer {
        return Vec::new();
    }

    // The header should be formatted with proper spacing:
    // - Date/time on the left
    // - Filename centered
    // - "Page X" on the right
    let date_part = &options.last_modified_time;
    let filename = &options.header;
    let page_part = format!("{} {page}", translate!("pr-page"));

    // Use the line width if available, otherwise use default of 72
    let total_width = options.line_width.unwrap_or(DEFAULT_COLUMN_WIDTH);

    let date_len = date_part.chars().count();
    let filename_len = filename.chars().count();
    let page_len = page_part.chars().count();

    let header_line = if date_len + filename_len + page_len + 2 < total_width {
        // The filename should be centered between the date and page parts
        let space_for_filename = total_width - date_len - page_len;
        let padding_before_filename = (space_for_filename - filename_len) / 2;
        let padding_after_filename = space_for_filename - filename_len - padding_before_filename;

        format!(
            "{date_part}{:padding_before_filename$}{filename}{:padding_after_filename$}{page_part}",
            "", ""
        )
    } else {
        // If content is too long, just use single spaces
        format!("{date_part} {filename} {page_part}")
    };

    vec![
        String::new(),
        String::new(),
        header_line,
        String::new(),
        String::new(),
    ]
}

/// Returns five empty lines as trailer content if displaying trailer
/// is not disabled by using `NO_HEADER_TRAILER_OPTION`option.
fn trailer_content(options: &OutputOptions) -> Vec<String> {
    if options.display_header_and_trailer && !options.form_feed_used {
        vec![
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ]
    } else {
        Vec::new()
    }
}

/// Returns starting line number for the file to be printed.
/// If -N is specified the first line number changes otherwise
/// default is 1.
fn get_start_line_number(opts: &OutputOptions) -> usize {
    opts.number.as_ref().map_or(1, |i| i.first_number)
}

/// Returns number of lines to read from input for constructing one page of pr output.
/// If double space -d is used lines are halved.
/// If columns --columns is used the lines are multiplied by the value.
fn lines_to_read_for_page(opts: &OutputOptions) -> usize {
    let content_lines_per_page = opts.content_lines_per_page;
    let columns = get_columns(opts);
    if opts.double_space {
        (content_lines_per_page / 2) * columns
    } else {
        content_lines_per_page * columns
    }
}

/// Returns number of columns to output
fn get_columns(opts: &OutputOptions) -> usize {
    opts.column_mode_options.as_ref().map_or(1, |i| i.columns)
}
