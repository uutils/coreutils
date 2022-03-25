// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.
//

// spell-checker:ignore (ToDO) adFfmprt, kmerge

#[macro_use]
extern crate quick_error;

use chrono::offset::Local;
use chrono::DateTime;
use clap::{AppSettings, Arg, ArgMatches, Command};
use itertools::Itertools;
use quick_error::ResultExt;
use regex::Regex;
use std::convert::From;
use std::fs::{metadata, File};
use std::io::{stdin, stdout, BufRead, BufReader, Lines, Read, Write};
#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;

use uucore::display::Quotable;
use uucore::error::{set_exit_code, UResult};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const ABOUT: &str =
    "Write content of given file or standard input to standard output with pagination filter";
const AFTER_HELP: &str =
    "    +PAGE\n            Begin output at page number page of the formatted input.
    -COLUMN\n            Produce multi-column output. See --column

The pr utility is a printing and pagination filter
for text files.  When multiple input files are specified,
each is read, formatted, and written to standard
output.  By default, the input is separated
into 66-line pages, each with

o   A 5-line header with the page number, date,
    time, and the pathname of the file.

o   A 5-line trailer consisting of blank lines.

If standard output is associated with a terminal,
diagnostic messages are suppressed until the pr
utility has completed processing.

When multiple column output is specified, text columns
are of equal width.  By default text columns
are separated by at least one <blank>.  Input lines
that do not fit into a text column are truncated.
Lines are not truncated under single column output.";
const TAB: char = '\t';
const LINES_PER_PAGE: usize = 66;
const LINES_PER_PAGE_FOR_FORM_FEED: usize = 63;
const HEADER_LINES_PER_PAGE: usize = 5;
const TRAILER_LINES_PER_PAGE: usize = 5;
const FILE_STDIN: &str = "-";
const READ_BUFFER_SIZE: usize = 1024 * 64;
const DEFAULT_COLUMN_WIDTH: usize = 72;
const DEFAULT_COLUMN_WIDTH_WITH_S_OPTION: usize = 512;
const DEFAULT_COLUMN_SEPARATOR: &char = &TAB;
const FF: u8 = 0x0C_u8;

mod options {
    pub const HEADER: &str = "header";
    pub const DOUBLE_SPACE: &str = "double-space";
    pub const NUMBER_LINES: &str = "number-lines";
    pub const FIRST_LINE_NUMBER: &str = "first-line-number";
    pub const PAGES: &str = "pages";
    pub const OMIT_HEADER: &str = "omit-header";
    pub const PAGE_LENGTH: &str = "length";
    pub const NO_FILE_WARNINGS: &str = "no-file-warnings";
    pub const FORM_FEED: &str = "form-feed";
    pub const COLUMN_WIDTH: &str = "width";
    pub const PAGE_WIDTH: &str = "page-width";
    pub const ACROSS: &str = "across";
    pub const COLUMN: &str = "column";
    pub const COLUMN_CHAR_SEPARATOR: &str = "separator";
    pub const COLUMN_STRING_SEPARATOR: &str = "sep-string";
    pub const MERGE: &str = "merge";
    pub const INDENT: &str = "indent";
    pub const JOIN_LINES: &str = "join-lines";
    pub const HELP: &str = "help";
    pub const VERSION: &str = "version";
    pub const FILES: &str = "files";
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
}

struct FileLine {
    file_id: usize,
    line_number: usize,
    page_number: usize,
    group_key: usize,
    line_content: Result<String, std::io::Error>,
    form_feeds_after: usize,
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

impl Default for NumberingMode {
    fn default() -> Self {
        Self {
            width: 5,
            separator: TAB.to_string(),
            first_number: 1,
        }
    }
}

impl Default for FileLine {
    fn default() -> Self {
        Self {
            file_id: 0,
            line_number: 0,
            page_number: 0,
            group_key: 0,
            line_content: Ok(String::new()),
            form_feeds_after: 0,
        }
    }
}

impl From<std::io::Error> for PrError {
    fn from(err: std::io::Error) -> Self {
        Self::EncounteredErrors(err.to_string())
    }
}

quick_error! {
    #[derive(Debug)]
    enum PrError {
        Input(err: std::io::Error, path: String) {
            context(path: &'a str, err: std::io::Error) -> (err, path.to_owned())
            display("pr: Reading from input {0} gave error", path)
            source(err)
        }

        UnknownFiletype(path: String) {
            display("pr: {0}: unknown filetype", path)
        }

        EncounteredErrors(msg: String) {
            display("pr: {0}", msg)
        }

        IsDirectory(path: String) {
            display("pr: {0}: Is a directory", path)
        }

        IsSocket(path: String) {
            display("pr: cannot open {}, Operation not supported on socket", path)
        }

        NotExists(path: String) {
            display("pr: cannot open {}, No such file or directory", path)
        }
    }
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(VERSION)
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .infer_long_args(true)
        .args_override_self(true)
        .setting(AppSettings::NoAutoHelp)
        .setting(AppSettings::NoAutoVersion)
        .arg(
            Arg::new(options::PAGES)
                .long(options::PAGES)
                .help("Begin and stop printing with page FIRST_PAGE[:LAST_PAGE]")
                .takes_value(true)
                .value_name("FIRST_PAGE[:LAST_PAGE]"),
        )
        .arg(
            Arg::new(options::HEADER)
                .short('h')
                .long(options::HEADER)
                .help(
                    "Use the string header to replace the file name \
                    in the header line.",
                )
                .takes_value(true)
                .value_name("STRING"),
        )
        .arg(
            Arg::new(options::DOUBLE_SPACE)
                .short('d')
                .long(options::DOUBLE_SPACE)
                .help("Produce output that is double spaced. An extra <newline> character is output following every <newline>
                found in the input.")
        )
        .arg(
            Arg::new(options::NUMBER_LINES)
                .short('n')
                .long(options::NUMBER_LINES)
                .help("Provide width digit line numbering.  The default for width, if not specified, is 5.  The number occupies
                the first width column positions of each text column or each line of -m output.  If char (any non-digit
                character) is given, it is appended to the line number to separate it from whatever follows.  The default
                for char is a <tab>.  Line numbers longer than width columns are truncated.")
                .takes_value(true)
                .allow_hyphen_values(true)
                .value_name("[char][width]")
        )
        .arg(
            Arg::new(options::FIRST_LINE_NUMBER)
                .short('N')
                .long(options::FIRST_LINE_NUMBER)
                .help("start counting with NUMBER at 1st line of first page printed")
                .takes_value(true)
                .value_name("NUMBER")
        )
        .arg(
            Arg::new(options::OMIT_HEADER)
                .short('t')
                .long(options::OMIT_HEADER)
                .help("Write neither the five-line identifying header nor the five-line trailer usually supplied for  each  page.  Quit
                writing after the last line of each file without spacing to the end of the page.")
        )
        .arg(
            Arg::new(options::PAGE_LENGTH)
                .short('l')
                .long(options::PAGE_LENGTH)
                .help("Override the 66-line default (default number of lines of text 56, and with -F 63) and reset the page length to lines.  If lines is not greater than the sum  of  both
                the  header  and trailer depths (in lines), the pr utility shall suppress both the header and trailer, as if the
                -t option were in effect. ")
                .takes_value(true)
                .value_name("PAGE_LENGTH")
        )
        .arg(
            Arg::new(options::NO_FILE_WARNINGS)
                .short('r')
                .long(options::NO_FILE_WARNINGS)
                .help("omit warning when a file cannot be opened")
        )
        .arg(
            Arg::new(options::FORM_FEED)
                .short('F')
                .short_alias('f')
                .long(options::FORM_FEED)
                .help("Use a <form-feed> for new pages, instead of the default behavior that uses a sequence of <newline>s.")
        )
        .arg(
            Arg::new(options::COLUMN_WIDTH)
                .short('w')
                .long(options::COLUMN_WIDTH)
                .help("Set the width of the line to width column positions for multiple text-column output only. If the -w option is
                not specified and the -s option is not specified, the default width shall be 72. If the -w option is not specified
                and the -s option is specified, the default width shall be 512.")
                .takes_value(true)
                .value_name("width")
        )
        .arg(
            Arg::new(options::PAGE_WIDTH)
                .short('W')
                .long(options::PAGE_WIDTH)
                .help("set page width to PAGE_WIDTH (72) characters always,
                truncate lines, except -J option is set, no interference
                with -S or -s")
                .takes_value(true)
                .value_name("width")
        )
        .arg(
            Arg::new(options::ACROSS)
                .short('a')
                .long(options::ACROSS)
                .help("Modify the effect of the - column option so that the columns are filled across the page in a  round-robin  order
                (for example, when column is 2, the first input line heads column 1, the second heads column 2, the third is the
                second line in column 1, and so on).")
        )
        .arg(
            Arg::new(options::COLUMN)
                .long(options::COLUMN)
                .help("Produce multi-column output that is arranged in column columns (the default shall be 1) and is written down each
                column  in  the order in which the text is received from the input file. This option should not be used with -m.
                The options -e and -i shall be assumed for multiple text-column output.  Whether or not text columns are produced
                with identical vertical lengths is unspecified, but a text column shall never exceed the length of the
                page (see the -l option). When used with -t, use the minimum number of lines to write the output.")
                .takes_value(true)
                .value_name("column")
        )
        .arg(
            Arg::new(options::COLUMN_CHAR_SEPARATOR)
                .short('s')
                .long(options::COLUMN_CHAR_SEPARATOR)
                .help("Separate text columns by the single character char instead of by the appropriate number of <space>s
                (default for char is the <tab> character).")
                .takes_value(true)
                .value_name("char")
        )
        .arg(
            Arg::new(options::COLUMN_STRING_SEPARATOR)
                .short('S')
                .long(options::COLUMN_STRING_SEPARATOR)
                .help("separate columns by STRING,
                without -S: Default separator <TAB> with -J and <space>
                otherwise (same as -S\" \"), no effect on column options")
                .takes_value(true)
                .value_name("string")
        )
        .arg(
            Arg::new(options::MERGE)
                .short('m')
                .long(options::MERGE)
                .help("Merge files. Standard output shall be formatted so the pr utility writes one line from each file specified by  a
                file  operand, side by side into text columns of equal fixed widths, in terms of the number of column positions.
                Implementations shall support merging of at least nine file operands.")
        )
        .arg(
            Arg::new(options::INDENT)
                .short('o')
                .long(options::INDENT)
                .help("Each line of output shall be preceded by offset <space>s. If the -o option is not specified, the default offset
                shall be zero. The space taken is in addition to the output line width (see the -w option below).")
                .takes_value(true)
                .value_name("margin")
        )
        .arg(
            Arg::new(options::JOIN_LINES)
                .short('J')
                .help("merge full lines, turns off -W line truncation, no column
                alignment, --sep-string[=STRING] sets separators")
        )
        .arg(
            Arg::new(options::HELP)
                .long(options::HELP)
                .help("Show this help message")
        )
        .arg(
            Arg::new(options::VERSION)
                .short('V')
                .long(options::VERSION)
                .help("Show version information")
        )
        .arg(
            Arg::new(options::FILES)
                .multiple_occurrences(true)
                .multiple_values(true)
        )
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(uucore::InvalidEncodingHandling::Ignore)
        .accept_any();

    let opt_args = recreate_arguments(&args);

    let mut command = uu_app();
    let matches = match command.try_get_matches_from_mut(opt_args) {
        Ok(m) => m,
        Err(e) => {
            e.print()?;
            set_exit_code(1);
            return Ok(());
        }
    };

    if matches.is_present(options::VERSION) {
        println!("{}", command.render_long_version());
        return Ok(());
    }

    if matches.is_present(options::HELP) {
        command.print_long_help()?;
        return Ok(());
    }

    let mut files = matches
        .values_of(options::FILES)
        .map(|v| v.collect::<Vec<_>>())
        .unwrap_or_default()
        .clone();
    if files.is_empty() {
        files.insert(0, FILE_STDIN);
    }

    let file_groups: Vec<_> = if matches.is_present(options::MERGE) {
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
    //let a_file: Regex = Regex::new(r"^[^-+].*").unwrap();
    let n_regex = Regex::new(r"^-n\s*$").unwrap();
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

    arguments
        .into_iter()
        .filter(|i| !column_page_option.is_match(i))
        .collect()
}

fn print_error(matches: &ArgMatches, err: &PrError) {
    if !matches.is_present(options::NO_FILE_WARNINGS) {
        eprintln!("{}", err);
    }
}

fn parse_usize(matches: &ArgMatches, opt: &str) -> Option<Result<usize, PrError>> {
    let from_parse_error_to_pr_error = |value_to_parse: (String, String)| {
        let i = value_to_parse.0;
        let option = value_to_parse.1;
        i.parse().map_err(|_e| {
            PrError::EncounteredErrors(format!("invalid {} argument {}", option, i.quote()))
        })
    };
    matches
        .value_of(opt)
        .map(|i| (i.to_string(), format!("-{}", opt)))
        .map(from_parse_error_to_pr_error)
}

fn build_options(
    matches: &ArgMatches,
    paths: &[&str],
    free_args: &str,
) -> Result<OutputOptions, PrError> {
    let form_feed_used = matches.is_present(options::FORM_FEED);

    let is_merge_mode = matches.is_present(options::MERGE);

    if is_merge_mode && matches.is_present(options::COLUMN) {
        let err_msg = String::from("cannot specify number of columns when printing in parallel");
        return Err(PrError::EncounteredErrors(err_msg));
    }

    if is_merge_mode && matches.is_present(options::ACROSS) {
        let err_msg = String::from("cannot specify both printing across and printing in parallel");
        return Err(PrError::EncounteredErrors(err_msg));
    }

    let merge_files_print = if matches.is_present(options::MERGE) {
        Some(paths.len())
    } else {
        None
    };

    let header = matches
        .value_of(options::HEADER)
        .unwrap_or(if is_merge_mode || paths[0] == FILE_STDIN {
            ""
        } else {
            paths[0]
        })
        .to_string();

    let default_first_number = NumberingMode::default().first_number;
    let first_number =
        parse_usize(matches, options::FIRST_LINE_NUMBER).unwrap_or(Ok(default_first_number))?;

    let number = matches
        .value_of(options::NUMBER_LINES)
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
            if matches.is_present(options::NUMBER_LINES) {
                Some(NumberingMode::default())
            } else {
                None
            }
        });

    let double_space = matches.is_present(options::DOUBLE_SPACE);

    let content_line_separator = if double_space {
        "\n".repeat(2)
    } else {
        "\n".to_string()
    };

    let line_separator = "\n".to_string();

    let last_modified_time = if is_merge_mode || paths[0].eq(FILE_STDIN) {
        let date_time = Local::now();
        date_time.format("%b %d %H:%M %Y").to_string()
    } else {
        file_last_modified_time(paths.get(0).unwrap())
    };

    // +page option is less priority than --pages
    let page_plus_re = Regex::new(r"\s*\+(\d+:*\d*)\s*").unwrap();
    let start_page_in_plus_option = match page_plus_re.captures(free_args).map(|i| {
        let unparsed_num = i.get(1).unwrap().as_str().trim();
        let x: Vec<_> = unparsed_num.split(':').collect();
        x[0].to_string().parse::<usize>().map_err(|_e| {
            PrError::EncounteredErrors(format!("invalid {} argument {}", "+", unparsed_num.quote()))
        })
    }) {
        Some(res) => res?,
        None => 1,
    };

    let end_page_in_plus_option = match page_plus_re
        .captures(free_args)
        .map(|i| i.get(1).unwrap().as_str().trim())
        .filter(|i| i.contains(':'))
        .map(|unparsed_num| {
            let x: Vec<_> = unparsed_num.split(':').collect();
            x[1].to_string().parse::<usize>().map_err(|_e| {
                PrError::EncounteredErrors(format!(
                    "invalid {} argument {}",
                    "+",
                    unparsed_num.quote()
                ))
            })
        }) {
        Some(res) => Some(res?),
        None => None,
    };

    let invalid_pages_map = |i: String| {
        let unparsed_value = matches.value_of(options::PAGES).unwrap();
        i.parse::<usize>().map_err(|_e| {
            PrError::EncounteredErrors(format!(
                "invalid --pages argument {}",
                unparsed_value.quote()
            ))
        })
    };

    let start_page = match matches
        .value_of(options::PAGES)
        .map(|i| {
            let x: Vec<_> = i.split(':').collect();
            x[0].to_string()
        })
        .map(invalid_pages_map)
    {
        Some(res) => res?,
        None => start_page_in_plus_option,
    };

    let end_page = match matches
        .value_of(options::PAGES)
        .filter(|i| i.contains(':'))
        .map(|i| {
            let x: Vec<_> = i.split(':').collect();
            x[1].to_string()
        })
        .map(invalid_pages_map)
    {
        Some(res) => Some(res?),
        None => end_page_in_plus_option,
    };

    if let Some(end_page) = end_page {
        if start_page > end_page {
            return Err(PrError::EncounteredErrors(format!(
                "invalid --pages argument '{}:{}'",
                start_page, end_page
            )));
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

    let display_header_and_trailer =
        !(page_length_le_ht) && !matches.is_present(options::OMIT_HEADER);

    let content_lines_per_page = if page_length_le_ht {
        page_length
    } else {
        page_length - (HEADER_LINES_PER_PAGE + TRAILER_LINES_PER_PAGE)
    };

    let page_separator_char = if matches.is_present(options::FORM_FEED) {
        let bytes = vec![FF];
        String::from_utf8(bytes).unwrap()
    } else {
        "\n".to_string()
    };

    let across_mode = matches.is_present(options::ACROSS);

    let column_separator = match matches.value_of(options::COLUMN_STRING_SEPARATOR) {
        Some(x) => Some(x),
        None => matches.value_of(options::COLUMN_CHAR_SEPARATOR),
    }
    .map(ToString::to_string)
    .unwrap_or_else(|| DEFAULT_COLUMN_SEPARATOR.to_string());

    let default_column_width = if matches.is_present(options::COLUMN_WIDTH)
        && matches.is_present(options::COLUMN_CHAR_SEPARATOR)
    {
        DEFAULT_COLUMN_WIDTH_WITH_S_OPTION
    } else {
        DEFAULT_COLUMN_WIDTH
    };

    let column_width =
        parse_usize(matches, options::COLUMN_WIDTH).unwrap_or(Ok(default_column_width))?;

    let page_width = if matches.is_present(options::JOIN_LINES) {
        None
    } else {
        match parse_usize(matches, options::PAGE_WIDTH) {
            Some(res) => Some(res?),
            None => None,
        }
    };

    let re_col = Regex::new(r"\s*-(\d+)\s*").unwrap();

    let start_column_option = match re_col.captures(free_args).map(|i| {
        let unparsed_num = i.get(1).unwrap().as_str().trim();
        unparsed_num.parse::<usize>().map_err(|_e| {
            PrError::EncounteredErrors(format!("invalid {} argument {}", "-", unparsed_num.quote()))
        })
    }) {
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
    let join_lines = matches.is_present(options::JOIN_LINES);

    let col_sep_for_printing = column_mode_options
        .as_ref()
        .map(|i| i.column_separator.clone())
        .unwrap_or_else(|| {
            merge_files_print
                .map(|_k| DEFAULT_COLUMN_SEPARATOR.to_string())
                .unwrap_or_default()
        });

    let columns_to_print = merge_files_print
        .unwrap_or_else(|| column_mode_options.as_ref().map(|i| i.columns).unwrap_or(1));

    let line_width = if join_lines {
        None
    } else if columns_to_print > 1 {
        Some(
            column_mode_options
                .as_ref()
                .map(|i| i.width)
                .unwrap_or(DEFAULT_COLUMN_WIDTH),
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
    })
}

fn open(path: &str) -> Result<Box<dyn Read>, PrError> {
    if path == FILE_STDIN {
        let stdin = stdin();
        return Ok(Box::new(stdin) as Box<dyn Read>);
    }

    metadata(path)
        .map(|i| {
            let path_string = path.to_string();
            match i.file_type() {
                #[cfg(unix)]
                ft if ft.is_block_device() => Err(PrError::UnknownFiletype(path_string)),
                #[cfg(unix)]
                ft if ft.is_char_device() => Err(PrError::UnknownFiletype(path_string)),
                #[cfg(unix)]
                ft if ft.is_fifo() => Err(PrError::UnknownFiletype(path_string)),
                #[cfg(unix)]
                ft if ft.is_socket() => Err(PrError::IsSocket(path_string)),
                ft if ft.is_dir() => Err(PrError::IsDirectory(path_string)),
                ft if ft.is_file() || ft.is_symlink() => {
                    Ok(Box::new(File::open(path).context(path)?) as Box<dyn Read>)
                }
                _ => Err(PrError::UnknownFiletype(path_string)),
            }
        })
        .unwrap_or_else(|_| Err(PrError::NotExists(path.to_string())))
}

fn split_lines_if_form_feed(file_content: Result<String, std::io::Error>) -> Vec<FileLine> {
    file_content
        .map(|content| {
            let mut lines = Vec::new();
            let mut f_occurred = 0;
            let mut chunk = Vec::new();
            for byte in content.as_bytes() {
                if byte == &FF {
                    f_occurred += 1;
                } else {
                    if f_occurred != 0 {
                        // First time byte occurred in the scan
                        lines.push(FileLine {
                            line_content: Ok(String::from_utf8(chunk.clone()).unwrap()),
                            form_feeds_after: f_occurred,
                            ..FileLine::default()
                        });
                        chunk.clear();
                    }
                    chunk.push(*byte);
                    f_occurred = 0;
                }
            }

            lines.push(FileLine {
                line_content: Ok(String::from_utf8(chunk).unwrap()),
                form_feeds_after: f_occurred,
                ..FileLine::default()
            });

            lines
        })
        .unwrap_or_else(|e| {
            vec![FileLine {
                line_content: Err(e),
                ..FileLine::default()
            }]
        })
}

fn pr(path: &str, options: &OutputOptions) -> Result<i32, PrError> {
    let lines = BufReader::with_capacity(READ_BUFFER_SIZE, open(path)?).lines();

    let pages = read_stream_and_create_pages(options, lines, 0);

    for page_with_page_number in pages {
        let page_number = page_with_page_number.0 + 1;
        let page = page_with_page_number.1;
        print_page(&page, options, page_number)?;
    }

    Ok(0)
}

fn read_stream_and_create_pages(
    options: &OutputOptions,
    lines: Lines<BufReader<Box<dyn Read>>>,
    file_id: usize,
) -> Box<dyn Iterator<Item = (usize, Vec<FileLine>)>> {
    let start_page = options.start_page;
    let start_line_number = get_start_line_number(options);
    let last_page = options.end_page;
    let lines_needed_per_page = lines_to_read_for_page(options);

    Box::new(
        lines
            .flat_map(split_lines_if_form_feed)
            .enumerate()
            .map(move |(i, line)| FileLine {
                line_number: i + start_line_number,
                file_id,
                ..line
            }) // Add line number and file_id
            .batching(move |it| {
                let mut first_page = Vec::new();
                let mut page_with_lines = Vec::new();
                for line in it {
                    let form_feeds_after = line.form_feeds_after;
                    first_page.push(line);

                    if form_feeds_after > 1 {
                        // insert empty pages
                        page_with_lines.push(first_page);
                        for _i in 1..form_feeds_after {
                            page_with_lines.push(vec![]);
                        }
                        return Some(page_with_lines);
                    }

                    if first_page.len() == lines_needed_per_page || form_feeds_after == 1 {
                        break;
                    }
                }

                if first_page.is_empty() {
                    return None;
                }
                page_with_lines.push(first_page);
                Some(page_with_lines)
            }) // Create set of pages as form feeds could lead to empty pages
            .flatten() // Flatten to pages from page sets
            .enumerate() // Assign page number
            .skip_while(move |(x, _)| {
                // Skip the not needed pages
                let current_page = x + 1;
                current_page < start_page
            })
            .take_while(move |(x, _)| {
                // Take only the required pages
                let current_page = x + 1;

                current_page >= start_page
                    && last_page.map_or(true, |last_page| current_page <= last_page)
            }),
    )
}

fn mpr(paths: &[&str], options: &OutputOptions) -> Result<i32, PrError> {
    let n_files = paths.len();

    // Check if files exists
    for path in paths {
        open(path)?;
    }

    let file_line_groups = paths
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let lines = BufReader::with_capacity(READ_BUFFER_SIZE, open(path).unwrap()).lines();

            read_stream_and_create_pages(options, lines, i).flat_map(move |(x, line)| {
                let file_line = line;
                let page_number = x + 1;
                file_line
                    .into_iter()
                    .map(|fl| FileLine {
                        page_number,
                        group_key: page_number * n_files + fl.file_id,
                        ..fl
                    })
                    .collect::<Vec<_>>()
            })
        })
        .kmerge_by(|a, b| {
            if a.group_key == b.group_key {
                a.line_number < b.line_number
            } else {
                a.group_key < b.group_key
            }
        })
        .group_by(|file_line| file_line.group_key);

    let start_page = options.start_page;
    let mut lines = Vec::new();
    let mut page_counter = start_page;

    for (_key, file_line_group) in &file_line_groups {
        for file_line in file_line_group {
            if let Err(e) = file_line.line_content {
                return Err(e.into());
            }
            let new_page_number = file_line.page_number;
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
) -> Result<usize, std::io::Error> {
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

    let lines_written = write_columns(lines, options, &mut out)?;

    for (index, x) in trailer_content.iter().enumerate() {
        out.write_all(x.as_bytes())?;
        if index + 1 != trailer_content.len() {
            out.write_all(line_separator)?;
        }
    }
    out.write_all(page_separator)?;
    out.flush()?;
    Ok(lines_written)
}

fn write_columns(
    lines: &[FileLine],
    options: &OutputOptions,
    out: &mut impl Write,
) -> Result<usize, std::io::Error> {
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
    let mut lines_printed = 0;
    let feed_line_present = options.form_feed_used;
    let mut not_found_break = false;

    let across_mode = options
        .column_mode_options
        .as_ref()
        .map(|i| i.across_mode)
        .unwrap_or(false);

    let mut filled_lines = Vec::new();
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

    let table: Vec<Vec<_>> = (0..content_lines_per_page)
        .map(move |a| {
            (0..columns)
                .map(|i| {
                    if across_mode {
                        lines.get(a * columns + i)
                    } else if options.merge_files_print.is_some() {
                        *filled_lines
                            .get(content_lines_per_page * i + a)
                            .unwrap_or(&None)
                    } else {
                        lines.get(content_lines_per_page * i + a)
                    }
                })
                .collect()
        })
        .collect();

    let blank_line = FileLine::default();
    for row in table {
        let indexes = row.len();
        for (i, cell) in row.iter().enumerate() {
            if cell.is_none() && options.merge_files_print.is_some() {
                out.write_all(
                    get_line_for_printing(options, &blank_line, columns, i, &line_width, indexes)
                        .as_bytes(),
                )?;
            } else if cell.is_none() {
                not_found_break = true;
                break;
            } else if cell.is_some() {
                let file_line = cell.unwrap();

                out.write_all(
                    get_line_for_printing(options, file_line, columns, i, &line_width, indexes)
                        .as_bytes(),
                )?;
                lines_printed += 1;
            }
        }
        if not_found_break && feed_line_present {
            break;
        } else {
            out.write_all(line_separator)?;
        }
    }

    Ok(lines_printed)
}

fn get_line_for_printing(
    options: &OutputOptions,
    file_line: &FileLine,
    columns: usize,
    index: usize,
    line_width: &Option<usize>,
    indexes: usize,
) -> String {
    let blank_line = String::new();
    let formatted_line_number = get_formatted_line_number(options, file_line.line_number, index);

    let mut complete_line = format!(
        "{}{}",
        formatted_line_number,
        file_line.line_content.as_ref().unwrap()
    );

    let offset_spaces = &options.offset_spaces;

    let tab_count = complete_line.chars().filter(|i| i == &TAB).count();

    let display_length = complete_line.len() + (tab_count * 7);

    let sep = if (index + 1) != indexes && !options.join_lines {
        &options.col_sep_for_printing
    } else {
        &blank_line
    };

    format!(
        "{}{}{}",
        offset_spaces,
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
        sep
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
            format!(
                "{:>width$}{}",
                &line_str[line_str.len() - width..],
                separator,
                width = width
            )
        } else {
            format!("{:>width$}{}", line_str, separator, width = width)
        }
    } else {
        String::new()
    }
}

/// Returns a five line header content if displaying header is not disabled by
/// using `NO_HEADER_TRAILER_OPTION` option.
fn header_content(options: &OutputOptions, page: usize) -> Vec<String> {
    if options.display_header_and_trailer {
        let first_line = format!(
            "{} {} Page {}",
            options.last_modified_time, options.header, page
        );
        vec![
            String::new(),
            String::new(),
            first_line,
            String::new(),
            String::new(),
        ]
    } else {
        Vec::new()
    }
}

fn file_last_modified_time(path: &str) -> String {
    metadata(path)
        .map(|i| {
            i.modified()
                .map(|x| {
                    let date_time: DateTime<Local> = x.into();
                    date_time.format("%b %d %H:%M %Y").to_string()
                })
                .unwrap_or_default()
        })
        .unwrap_or_default()
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
    opts.number.as_ref().map(|i| i.first_number).unwrap_or(1)
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
    opts.column_mode_options
        .as_ref()
        .map(|i| i.columns)
        .unwrap_or(1)
}
