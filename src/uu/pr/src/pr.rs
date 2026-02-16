// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//

// spell-checker:ignore (ToDO) adFfmprt, kmerge

use clap::{Arg, ArgAction, ArgMatches, Command};
use itertools::Itertools;
use regex::Regex;
use std::fs::metadata;
use std::io::{Read, Write, stderr, stdin, stdout};
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::time::SystemTime;
use thiserror::Error;

use uucore::display::Quotable;
use uucore::error::UResult;
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
    offset_spaces: String,
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
    line_content: String,
}

impl FileLine {
    fn from_buf(
        file_id: usize,
        page_number: usize,
        line_number: usize,
        buf: &[u8],
        options: &OutputOptions,
    ) -> Result<Self, PrError> {
        // TODO Don't read bytes to String just to directly write them
        // out again anyway.
        let line_content = if let Some(expand_tabs) = &options.expand_tabs {
            // Anticipate a few expandable chars to reduce reallocations
            let mut line_content =
                String::with_capacity(buf.len() + buf.len() / 20 * expand_tabs.width as usize);
            // validate utf correctness
            let s = std::str::from_utf8(buf)?;
            for b in s.as_bytes() {
                apply_expand_tab(&mut line_content, *b, expand_tabs);
            }
            line_content
        } else {
            String::from_utf8(buf.to_vec())?
        };

        Ok(Self {
            file_id,
            page_number,
            line_number,
            line_content,
        })
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
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
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

    let mut files = matches
        .get_many::<String>(options::FILES)
        .map(|v| v.map(String::as_str).collect::<Vec<_>>())
        .unwrap_or_default()
        .clone();
    if files.is_empty() {
        files.insert(0, FILE_STDIN);
    }

    let file_groups: Vec<_> = if matches.get_flag(options::MERGE) {
        vec![files]
    } else {
        files.into_iter().map(|i| vec![i]).collect()
    };

    for file_group in file_groups {
        let result_options = build_options(&matches, &file_group, &args.join(" "));
        let options = match result_options {
            Ok(options) => options,
            Err(err) => {
                print_error(&matches, &err);
                return Err(1.into());
            }
        };

        let cmd_result = if let Ok(group) = file_group.iter().exactly_one() {
            pr(group, &options)
        } else {
            mpr(&file_group, &options)
        };

        let status = match cmd_result {
            Err(error) => {
                print_error(&matches, &error);
                1
            }
            _ => 0,
        };
        if status != 0 {
            return Err(status.into());
        }
    }
    Ok(())
}

/// Returns re-written arguments which are passed to the program.
/// Removes -column and +page option as getopts cannot parse things like -3 etc
/// # Arguments
/// * `args` - Command line arguments
fn recreate_arguments(args: &[String]) -> Vec<String> {
    let column_page_option = Regex::new(r"^[-+]\d+.*").unwrap();
    let num_regex = Regex::new(r"^[^-]\d*$").unwrap();
    let n_regex = Regex::new(r"^-n\s*$").unwrap();
    let e_regex = Regex::new(r"^-e").unwrap();
    let mut arguments = args.to_owned();
    let num_option = args.iter().find_position(|x| n_regex.is_match(x.trim()));
    if let Some((pos, _value)) = num_option {
        if let Some(num_val_opt) = args.get(pos + 1) {
            if !num_regex.is_match(num_val_opt) {
                let could_be_file = arguments.remove(pos + 1);
                arguments.insert(pos + 1, format!("{}", NumberingMode::default().width));
                arguments.insert(pos + 2, could_be_file);
            }
        }
    }

    // To ensure not to accidentally delete the next argument after a short flag for -e we insert
    // the default values for the -e flag is '-e' is present without direct arguments.
    let expand_tabs_option = arguments
        .iter()
        .find_position(|x| e_regex.is_match(x.trim()));
    if let Some((pos, value)) = expand_tabs_option {
        if value.trim().len() <= 2 {
            arguments[pos] = "-e\t8".to_string();
        }
    }

    arguments
        .into_iter()
        .filter(|i| !column_page_option.is_match(i))
        .collect()
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
            msg: format!("invalid {option} argument {}", i.quote()),
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
                && (std::env::var("LC_TIME").unwrap_or_default() == "POSIX"
                    || std::env::var("LC_ALL").unwrap_or_default() == "POSIX")
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
    free_args: &str,
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
            let parse_result = i.parse::<usize>();

            let separator = if parse_result.is_err() {
                i[0..1].to_string()
            } else {
                NumberingMode::default().separator
            };

            let width = match parse_result {
                Ok(res) => res,
                Err(_) => i[1..]
                    .parse::<usize>()
                    .unwrap_or(NumberingMode::default().width),
            };

            NumberingMode {
                width,
                separator,
                first_number,
            }
        })
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
            s.chars().next().map_or(Ok(ExpandTabsOptions::default()), |c| {
                if c.is_ascii_digit() {
                    s
                        .parse()
                        .map_err(|_e| PrError::EncounteredErrors { msg: format!("{}\n{}", translate!("pr-error-invalid-expand-tab-argument", "arg" => s), translate!("pr-try-help-message")) })
                        .map(|width| ExpandTabsOptions{input_char: TAB, width})
                } else if s.len() > 1 {
                    s[1..]
                        .parse()
                        .map_err(|_e| PrError::EncounteredErrors { msg: format!("{}\n{}", translate!("pr-error-invalid-expand-tab-argument", "arg" => &s[1..]), translate!("pr-try-help-message")) })
                        .map(|width| ExpandTabsOptions{input_char: c, width})
                } else {
                    Ok(ExpandTabsOptions{input_char: c, width: 8})
                }
            })
        }).transpose()?;

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
    let page_plus_re = Regex::new(r"\s*\+(\d+:*\d*)\s*").unwrap();
    let res = page_plus_re.captures(free_args).map(|i| {
        let unparsed_num = i.get(1).unwrap().as_str().trim();
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

    let res = page_plus_re
        .captures(free_args)
        .map(|i| i.get(1).unwrap().as_str().trim())
        .filter(|i| i.contains(':'))
        .map(|unparsed_num| {
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
        i.parse::<usize>().map_err(|_e| PrError::EncounteredErrors {
            msg: format!("invalid --pages argument {}", unparsed_value.quote()),
        })
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

    if let Some(end_page) = end_page {
        if start_page > end_page {
            return Err(PrError::EncounteredErrors {
                msg: translate!("pr-error-invalid-pages-range", "start" => start_page, "end" => end_page),
            });
        }
    }

    let default_lines_per_page = if form_feed_used {
        LINES_PER_PAGE_FOR_FORM_FEED
    } else {
        LINES_PER_PAGE
    };

    let page_length =
        parse_usize(matches, options::PAGE_LENGTH).unwrap_or(Ok(default_lines_per_page))?;

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

    let page_width = if matches.get_flag(options::JOIN_LINES) {
        None
    } else {
        match parse_usize(matches, options::PAGE_WIDTH) {
            Some(res) => Some(res?),
            None => None,
        }
    };

    let re_col = Regex::new(r"\s*-(\d+)\s*").unwrap();

    let res = re_col.captures(free_args).map(|i| {
        let unparsed_num = i.get(1).unwrap().as_str().trim();
        unparsed_num
            .parse::<usize>()
            .map_err(|_e| PrError::EncounteredErrors {
                msg: format!("invalid {} argument {}", "-", unparsed_num.quote()),
            })
    });
    let start_column_option = match res {
        Some(res) => Some(res?),
        None => None,
    };

    // --column has more priority than -column

    let column_option_value = match parse_usize(matches, options::COLUMN) {
        Some(res) => Some(res?),
        None => start_column_option,
    };

    let column_mode_options = column_option_value.map(|columns| ColumnModeOptions {
        columns,
        width: column_width,
        column_separator,
        across_mode,
    });

    let offset_spaces = " ".repeat(parse_usize(matches, options::INDENT).unwrap_or(Ok(0))?);
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

/// Read the entire contents of the given path into memory.
///
/// If `path` is `"-"`, then read from stdin.
fn read_to_end(path: &str) -> Result<Vec<u8>, std::io::Error> {
    if path == "-" {
        let mut f = stdin();
        let mut buf = vec![];
        f.read_to_end(&mut buf)?;
        Ok(buf)
    } else {
        std::fs::read(path)
    }
}

fn apply_expand_tab(chunk: &mut String, byte: u8, expand_options: &ExpandTabsOptions) {
    if byte == expand_options.input_char as u8 {
        // If the byte encountered is the input char we use width to calculate
        // the amount of spaces needed (if no input char given we stored '\t'
        // in our struct)
        let spaces_needed =
            expand_options.width as usize - (chunk.len() % expand_options.width as usize);
        chunk.extend(std::iter::repeat_n(' ', spaces_needed));
    } else if byte == TAB as u8 {
        // If a byte got passed to the -e flag (eg -ea1)  which is not '\t' GNU
        // still expands it but does not use an optionally given width parameter
        // but does the '\t' expansion with the default value (8)
        let spaces_needed = 8 - (chunk.len() % 8);
        chunk.extend(std::iter::repeat_n(' ', spaces_needed));
    } else {
        // This arm means the byte is neither '\t' nor the bytes to be
        // expanded
        chunk.push(byte as char);
    }
}

fn pr(path: &str, options: &OutputOptions) -> Result<i32, PrError> {
    // Read the entire contents of the file into a buffer.
    //
    // TODO Read incrementally.
    let buf = read_to_end(path)?;

    let pages = get_pages(options, 0, &buf)?;

    // Split the text into pages, and then print each line in each page.
    for page_with_page_number in pages {
        let page_number = page_with_page_number.0 + 1;
        let page = page_with_page_number.1;
        print_page(&page, options, page_number)?;
    }

    Ok(0)
}

/// Group lines of a file into pages.
///
/// Returns a list of the form `(page_num, lines)`.
///
/// # Errors
///
/// Returns an error if the bytes are not a valid UTF-8 string.
fn get_pages(
    options: &OutputOptions,
    file_id: usize,
    buf: &[u8],
) -> Result<Vec<(usize, Vec<FileLine>)>, PrError> {
    let start_page = options.start_page;
    let end_page = options.end_page;
    let lines_needed_per_page = lines_to_read_for_page(options);

    // Keep a running total of the number of lines read, starting with
    // 0 or another specified number.
    let mut line_num = get_start_line_number(options);

    // We will collect each page into a list of pages, along with
    // its page number.
    let mut pages: Vec<(usize, Vec<FileLine>)> = vec![];

    // We will build each page iteratively, since one page may
    // contain multiple lines and may be interrupted by either a
    // form feed or by reaching a line limit.
    let mut page = vec![];
    let mut page_num = 0;

    // Remember the index of the end of the last line to use as the
    // beginning of the next line.
    let mut prev = 0;

    // Search for either the form feed character `\f` or the newline
    // character `\n`. The newline character marks the end of a line,
    // and a page comprises several lines. A form feed character marks
    // the end of a page regardless of how many lines have been read.
    for i in memchr::memchr2_iter(FF, NL, buf) {
        if buf[i] == FF {
            // Treat everything up to (but not including) the form feed
            // character as the last line of the page.
            if i > 0 && i == prev && buf[i - 1] == NL {
                // If the file has the pattern `\n\f`, don't treat the
                // `\f` as its own line; instead ignore the empty line.
            } else {
                let file_line =
                    FileLine::from_buf(file_id, page_num, line_num, &buf[prev..i], options)?;
                page.push(file_line);
            }

            // Remember where the last line ended.
            prev = i + 1;

            // The page is finished, so we add it to the list of
            // pages and clear the `page` buffer for the next
            // iteration.
            //
            // TODO Optimization opportunity: don't bother pushing
            // lines and pages if we aren't going to display it.
            if start_page <= page_num + 1 && end_page.is_none_or(|e| page_num < e) {
                pages.push((page_num, page.clone()));
            }
            page_num += 1;
            page.clear();
        } else {
            // Add everything up to (but not including) the newline
            // character as one line of the page.
            if i > 0 && i == prev && buf[i - 1] == FF {
                // If the file has the pattern `\f\n`, don't treat the
                // `\n` as its own line; instead ignore the empty line.
            } else {
                let file_line =
                    FileLine::from_buf(file_id, page_num, line_num, &buf[prev..i], options)?;
                page.push(file_line);
                line_num += 1;
            }

            // Remember where the last line ended.
            prev = i + 1;

            // If the page is finished, add it to the list of pages
            // and clear the `page` buffer for the next iteration.
            if page.len() >= lines_needed_per_page {
                if start_page <= page_num + 1 && end_page.is_none_or(|e| page_num < e) {
                    pages.push((page_num, page.clone()));
                }
                page_num += 1;
                page.clear();
            }
        }
    }

    // Consider all trailing bytes as the last line.
    if prev < buf.len() {
        let file_line = FileLine::from_buf(file_id, page_num, line_num, &buf[prev..], options)?;
        page.push(file_line);
    }

    // Consider all trailing lines as the last page.
    if !page.is_empty() && start_page <= page_num + 1 && end_page.is_none_or(|e| page_num < e) {
        pages.push((page_num, page.clone()));
    }

    Ok(pages)
}

/// Key used to group lines together according to their file and page number.
fn group_key(num_files: usize, line: &FileLine) -> usize {
    (line.page_number + 1) * num_files + line.file_id
}

/// Group each line by its file and page number.
///
/// The input list of `lines` must be already sorted according to the
/// `group_key`.
fn group_lines(num_files: usize, lines: Vec<FileLine>) -> Vec<(usize, Vec<FileLine>)> {
    let mut result: Vec<(usize, Vec<FileLine>)> = vec![];
    let mut current_key: Option<usize> = None;
    let mut current_group: Vec<FileLine> = vec![];
    for file_line in lines {
        match current_key {
            None => {
                current_key = Some(group_key(num_files, &file_line));
                current_group.push(file_line);
            }
            Some(key) if group_key(num_files, &file_line) == key => {
                current_group.push(file_line);
            }
            Some(key) => {
                result.push((key, current_group.clone()));
                current_group.clear();
                current_key = Some(group_key(num_files, &file_line));
                current_group.push(file_line);
            }
        }
    }
    // TODO Handle empty file.
    result.push((current_key.unwrap(), current_group));
    result
}

/// Group each line by its file and page number.
///
/// Each group can then be merged into columns of a single page.
fn get_file_line_groups(
    options: &OutputOptions,
    paths: &[&str],
) -> Result<Vec<(usize, Vec<FileLine>)>, PrError> {
    let num_files = paths.len();
    let mut all_lines = vec![];
    for (file_id, path) in paths.iter().enumerate() {
        // Read the entire contents of the file into a buffer.
        //
        // TODO Read incrementally.
        let buf = read_to_end(path)?;

        // Split the text into pages and collect each line for
        // subsequent grouping.
        for (_, mut page) in get_pages(options, file_id, &buf)? {
            all_lines.append(&mut page);
        }
    }
    // Sort each line by group number and then by line number.
    all_lines.sort_by_key(|l| (group_key(num_files, l), l.line_number));

    Ok(group_lines(num_files, all_lines))
}

fn mpr(paths: &[&str], options: &OutputOptions) -> Result<i32, PrError> {
    let file_line_groups = get_file_line_groups(options, paths)?;

    let start_page = options.start_page;
    let mut lines = Vec::new();
    let mut page_counter = start_page;

    for (_key, file_line_group) in file_line_groups {
        for file_line in file_line_group {
            let new_page_number = file_line.page_number + 1;
            if page_counter != new_page_number {
                print_page(&lines, options, page_counter)?;
                lines = Vec::new();
                page_counter = new_page_number;
            }
            lines.push(file_line);
        }
    }

    print_page(&lines, options, page_counter)?;

    Ok(0)
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
    let mut out = out.lock();

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
    out.flush()?;
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

/// Group the lines of the input files in columns, one column per file.
fn to_table_merged(
    content_lines_per_page: usize,
    columns: usize,
    filled_lines: Vec<Option<&FileLine>>,
) -> Vec<Vec<Option<&FileLine>>> {
    (0..content_lines_per_page)
        .map(|i| {
            (0..columns)
                .map(|j| {
                    *filled_lines
                        .get(content_lines_per_page * j + i)
                        .unwrap_or(&None)
                })
                .collect()
        })
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

    let mut filled_lines: Vec<Option<&FileLine>> = Vec::new();
    if options.merge_files_print.is_some() {
        let mut offset = 0;
        for col in 0..columns {
            let mut inserted = 0;
            for line in &lines[offset..] {
                if line.file_id != col {
                    break;
                }
                filled_lines.push(Some(line));
                inserted += 1;
            }
            offset += inserted;

            for _i in inserted..content_lines_per_page {
                filled_lines.push(None);
            }
        }
    }

    // Group the flat list of lines into a 2-dimensional table of
    // cells, where each row will be printed as a single line in the
    // output.
    let merge = options.merge_files_print.is_some();
    let table = if !merge && (lines.len() < (content_lines_per_page * columns)) {
        to_table_short_file(content_lines_per_page, columns, lines)
    } else if across_mode {
        to_table_across(content_lines_per_page, columns, lines)
    } else if merge {
        to_table_merged(content_lines_per_page, columns, filled_lines)
    } else {
        to_table(content_lines_per_page, columns, lines)
    };

    let blank_line = FileLine::default();
    for row in table {
        let indexes = row.len();
        for (i, cell) in row.iter().enumerate() {
            if cell.is_none() && options.merge_files_print.is_some() {
                out.write_all(
                    get_line_for_printing(options, &blank_line, columns, i, line_width, indexes)
                        .as_bytes(),
                )?;
            } else if cell.is_none() {
                not_found_break = true;
                break;
            } else if cell.is_some() {
                let file_line = cell.unwrap();

                out.write_all(
                    get_line_for_printing(options, file_line, columns, i, line_width, indexes)
                        .as_bytes(),
                )?;
            }
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

    let mut complete_line = format!("{formatted_line_number}{}", file_line.line_content);

    let offset_spaces = &options.offset_spaces;

    let tab_count = complete_line.chars().filter(|i| i == &TAB).count();

    let display_length = complete_line.len() + (tab_count * 7);

    let sep = if (index + 1) != indexes && !options.join_lines {
        &options.col_sep_for_printing
    } else {
        &blank_line
    };

    format!(
        "{offset_spaces}{}{sep}",
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
            format!("{:>width$}{separator}", &line_str[line_str.len() - width..],)
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
