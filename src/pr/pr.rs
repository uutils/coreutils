#![crate_name = "uu_pr"]

// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.
//

#[cfg(unix)]
extern crate unix_socket;
#[macro_use]
extern crate quick_error;
extern crate itertools;
extern crate chrono;
extern crate getopts;
extern crate uucore;

use std::io::{BufRead, BufReader, stdin, stdout, stderr, Error, Read, Write, Stdout, Lines};
use std::vec::Vec;
use chrono::offset::Local;
use chrono::DateTime;
use getopts::{Matches, Options};
use std::fs::{metadata, File};
#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
use quick_error::ResultExt;
use std::convert::From;
use getopts::{HasArg, Occur};
use std::num::ParseIntError;
use itertools::{Itertools, GroupBy};
use std::iter::{Enumerate, Map, TakeWhile, SkipWhile};

static NAME: &str = "pr";
static VERSION: &str = env!("CARGO_PKG_VERSION");
static TAB: char = '\t';
static LINES_PER_PAGE: usize = 66;
static HEADER_LINES_PER_PAGE: usize = 5;
static TRAILER_LINES_PER_PAGE: usize = 5;
static NUMBERING_MODE_DEFAULT_SEPARATOR: &str = "\t";
static NUMBERING_MODE_DEFAULT_WIDTH: usize = 5;
static STRING_HEADER_OPTION: &str = "h";
static DOUBLE_SPACE_OPTION: &str = "d";
static NUMBERING_MODE_OPTION: &str = "n";
static FIRST_LINE_NUMBER_OPTION: &str = "N";
static PAGE_RANGE_OPTION: &str = "pages";
static NO_HEADER_TRAILER_OPTION: &str = "t";
static PAGE_LENGTH_OPTION: &str = "l";
static SUPPRESS_PRINTING_ERROR: &str = "r";
static FORM_FEED_OPTION: &str = "F";
static COLUMN_WIDTH_OPTION: &str = "w";
static ACROSS_OPTION: &str = "a";
static COLUMN_OPTION: &str = "column";
static FILE_STDIN: &str = "-";
static READ_BUFFER_SIZE: usize = 1024 * 64;
static DEFAULT_COLUMN_WIDTH: usize = 72;
static DEFAULT_COLUMN_SEPARATOR: &str = "\t";

struct OutputOptions {
    /// Line numbering mode
    number: Option<NumberingMode>,
    header: String,
    double_space: bool,
    line_separator: String,
    content_line_separator: String,
    last_modified_time: String,
    start_page: Option<usize>,
    end_page: Option<usize>,
    display_header: bool,
    display_trailer: bool,
    content_lines_per_page: usize,
    page_separator_char: String,
    column_mode_options: Option<ColumnModeOptions>,
}

struct ColumnModeOptions {
    width: Option<usize>,
    columns: usize,
    column_separator: String,
    across_mode: bool,
}

impl AsRef<OutputOptions> for OutputOptions {
    fn as_ref(&self) -> &OutputOptions {
        self
    }
}

struct NumberingMode {
    /// Line numbering mode
    width: usize,
    separator: String,
    first_number: usize,
}

impl Default for NumberingMode {
    fn default() -> NumberingMode {
        NumberingMode {
            width: NUMBERING_MODE_DEFAULT_WIDTH,
            separator: NUMBERING_MODE_DEFAULT_SEPARATOR.to_string(),
            first_number: 1,
        }
    }
}

impl From<Error> for PrError {
    fn from(err: Error) -> Self {
        PrError::EncounteredErrors(err.to_string())
    }
}

impl From<std::num::ParseIntError> for PrError {
    fn from(err: std::num::ParseIntError) -> Self {
        PrError::EncounteredErrors(err.to_string())
    }
}

quick_error! {
    #[derive(Debug)]
    enum PrError {
        Input(err: Error, path: String) {
            context(path: &'a str, err: Error) -> (err, path.to_owned())
            display("pr: Reading from input {0} gave error", path)
            cause(err)
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

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.opt(
        "",
        PAGE_RANGE_OPTION,
        "Begin and stop printing with page FIRST_PAGE[:LAST_PAGE]",
        "FIRST_PAGE[:LAST_PAGE]",
        HasArg::Yes,
        Occur::Optional,
    );

    opts.opt(
        STRING_HEADER_OPTION,
        "header",
        "Use the string header to replace the file name \
     in the header line.",
        "STRING",
        HasArg::Yes,
        Occur::Optional,
    );

    opts.opt(
        DOUBLE_SPACE_OPTION,
        "double-space",
        "Produce output that is double spaced. An extra <newline> character is output following every <newline>
           found in the input.",
        "",
        HasArg::No,
        Occur::Optional,
    );

    opts.opt(
        NUMBERING_MODE_OPTION,
        "number-lines",
        "Provide width digit line numbering.  The default for width, if not specified, is 5.  The number occupies
           the first width column positions of each text column or each line of -m output.  If char (any nondigit
           character) is given, it is appended to the line number to separate it from whatever follows.  The default
           for char is a <tab>.  Line numbers longer than width columns are truncated.",
        "[char][width]",
        HasArg::Maybe,
        Occur::Optional,
    );

    opts.opt(
        FIRST_LINE_NUMBER_OPTION,
        "first-line-number",
        "start counting with NUMBER at 1st line of first page printed",
        "NUMBER",
        HasArg::Yes,
        Occur::Optional,
    );

    opts.opt(
        NO_HEADER_TRAILER_OPTION,
        "omit-header",
        "Write neither the five-line identifying header nor the five-line trailer usually supplied for  each  page.  Quit
              writing after the last line of each file without spacing to the end of the page.",
        "",
        HasArg::No,
        Occur::Optional,
    );

    opts.opt(
        PAGE_LENGTH_OPTION,
        "length",
        "Override the 66-line default and reset the page length to lines.  If lines is not greater than the sum  of  both
              the  header  and trailer depths (in lines), the pr utility shall suppress both the header and trailer, as if the
              -t option were in effect.",
        "lines",
        HasArg::Yes,
        Occur::Optional,
    );

    opts.opt(
        SUPPRESS_PRINTING_ERROR,
        "no-file-warnings",
        "omit warning when a file cannot be opened",
        "",
        HasArg::No,
        Occur::Optional,
    );

    opts.opt(
        FORM_FEED_OPTION,
        "form-feed",
        "Use a <form-feed> for new pages, instead of the default behavior that uses a sequence of <newline>s.",
        "",
        HasArg::No,
        Occur::Optional,
    );

    opts.opt(
        "",
        COLUMN_OPTION,
        "Produce multi-column output that is arranged in column columns (the default shall be 1) and is written down each
              column  in  the order in which the text is received from the input file. This option should not be used with -m.
              The options -e and -i shall be assumed for multiple text-column output.  Whether or not text  columns  are  pro‐
              duced  with  identical  vertical  lengths is unspecified, but a text column shall never exceed the length of the
              page (see the -l option). When used with -t, use the minimum number of lines to write the output.",
        "[column]",
        HasArg::Yes,
        Occur::Optional,
    );

    opts.opt(
        COLUMN_WIDTH_OPTION,
        "width",
        "Set  the  width  of the line to width column positions for multiple text-column output only. If the -w option is
              not specified and the -s option is not specified, the default width shall be 72. If the -w option is not  speci‐
              fied and the -s option is specified, the default width shall be 512.",
        "[width]",
        HasArg::Yes,
        Occur::Optional,
    );

    opts.opt(
        ACROSS_OPTION,
        "across",
        "Modify the effect of the - column option so that the columns are filled across the page in a  round-robin  order
              (for example, when column is 2, the first input line heads column 1, the second heads column 2, the third is the
              second line in column 1, and so on).",
        "",
        HasArg::No,
        Occur::Optional,
    );

    opts.optflag("", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => panic!("Invalid options\n{}", e),
    };

    if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
        return 0;
    }

    let mut files: Vec<String> = matches.free.clone();
    if files.is_empty() {
        // -n value is optional if -n <path> is given the opts gets confused
        if matches.opt_present(NUMBERING_MODE_OPTION) {
            let maybe_file = matches.opt_str(NUMBERING_MODE_OPTION).unwrap();
            let is_afile = is_a_file(&maybe_file);
            if !is_afile {
                print_error(&matches, PrError::NotExists(maybe_file));
                return 1;
            } else {
                files.push(maybe_file);
            }
        } else {
            //For stdin
            files.push(FILE_STDIN.to_owned());
        }
    }


    if matches.opt_present("help") {
        return print_usage(&mut opts, &matches);
    }

    for f in files {
        let result_options = build_options(&matches, &f);
        if result_options.is_err() {
            print_error(&matches, result_options.err().unwrap());
            return 1;
        }
        let options = &result_options.unwrap();
        let status: i32 = match pr(&f, options) {
            Err(error) => {
                print_error(&matches, error);
                1
            }
            _ => 0
        };
        if status != 0 {
            return status;
        }
    }
    return 0;
}

fn is_a_file(could_be_file: &String) -> bool {
    File::open(could_be_file).is_ok()
}

fn print_error(matches: &Matches, err: PrError) {
    if !matches.opt_present(SUPPRESS_PRINTING_ERROR) {
        writeln!(&mut stderr(), "{}", err);
    }
}

fn print_usage(opts: &mut Options, matches: &Matches) -> i32 {
    println!("{} {} -- print files", NAME, VERSION);
    println!();
    println!("Usage: {} [+page] [-column] [-adFfmprt] [[-e] [char] [gap]]
        [-L locale] [-h header] [[-i] [char] [gap]]
        [-l lines] [-o offset] [[-s] [char]] [[-n] [char]
        [width]] [-w width] [-] [file ...].", NAME);
    println!();
    let usage: &str = "The pr utility is a printing and pagination filter
     for text files.  When multiple input files are spec-
     ified, each is read, formatted, and written to stan-
     dard output.  By default, the input is separated
     into 66-line pages, each with

     o   A 5-line header with the page number, date,
         time, and the pathname of the file.

     o   A 5-line trailer consisting of blank lines.

     If standard output is associated with a terminal,
     diagnostic messages are suppressed until the pr
     utility has completed processing.

     When multiple column output is specified, text col-
     umns are of equal width.  By default text columns
     are separated by at least one <blank>.  Input lines
     that do not fit into a text column are truncated.
     Lines are not truncated under single column output.";
    println!("{}", opts.usage(usage));
    if matches.free.is_empty() {
        return 1;
    }
    return 0;
}

fn build_options(matches: &Matches, path: &String) -> Result<OutputOptions, PrError> {
    let header: String = matches.opt_str(STRING_HEADER_OPTION).unwrap_or(path.to_string());

    let default_first_number = NumberingMode::default().first_number;
    let first_number = matches.opt_str(FIRST_LINE_NUMBER_OPTION).map(|n| {
        n.parse::<usize>().unwrap_or(default_first_number)
    }).unwrap_or(default_first_number);

    let numbering_options: Option<NumberingMode> = matches.opt_str(NUMBERING_MODE_OPTION).map(|i| {
        let parse_result = i.parse::<usize>();

        let separator = if parse_result.is_err() {
            if is_a_file(&i) {
                NumberingMode::default().separator
            } else {
                i[0..1].to_string()
            }
        } else {
            NumberingMode::default().separator
        };

        let width = if parse_result.is_err() {
            if is_a_file(&i) {
                NumberingMode::default().width
            } else {
                i[1..].parse::<usize>().unwrap_or(NumberingMode::default().width)
            }
        } else {
            parse_result.unwrap()
        };

        NumberingMode {
            width,
            separator,
            first_number,
        }
    }).or_else(|| {
        if matches.opt_present(NUMBERING_MODE_OPTION) {
            return Some(NumberingMode::default());
        }
        return None;
    });

    let double_space = matches.opt_present(DOUBLE_SPACE_OPTION);

    let content_line_separator: String = if double_space {
        "\n\n".to_string()
    } else {
        "\n".to_string()
    };

    let line_separator: String = "\n".to_string();

    let last_modified_time = if path.eq(FILE_STDIN) {
        current_time()
    } else {
        file_last_modified_time(path)
    };

    let invalid_pages_map = |i: Result<usize, ParseIntError>| {
        let unparsed_value = matches.opt_str(PAGE_RANGE_OPTION).unwrap();
        match i {
            Ok(val) => Ok(val),
            Err(_e) => Err(PrError::EncounteredErrors(format!("invalid --pages argument '{}'", unparsed_value)))
        }
    };

    let start_page = match matches.opt_str(PAGE_RANGE_OPTION).map(|i| {
        let x: Vec<&str> = i.split(":").collect();
        x[0].parse::<usize>()
    }).map(invalid_pages_map)
        {
            Some(res) => Some(res?),
            _ => None
        };

    let end_page = match matches.opt_str(PAGE_RANGE_OPTION)
        .filter(|i| i.contains(":"))
        .map(|i| {
            let x: Vec<&str> = i.split(":").collect();
            x[1].parse::<usize>()
        })
        .map(invalid_pages_map)
        {
            Some(res) => Some(res?),
            _ => None
        };

    if start_page.is_some() && end_page.is_some() && start_page.unwrap() > end_page.unwrap() {
        return Err(PrError::EncounteredErrors(format!("invalid --pages argument '{}:{}'", start_page.unwrap(), end_page.unwrap())));
    }

    let page_length = match matches.opt_str(PAGE_LENGTH_OPTION).map(|i| {
        i.parse::<usize>()
    }) {
        Some(res) => res?,
        _ => LINES_PER_PAGE
    };
    let page_length_le_ht = page_length < (HEADER_LINES_PER_PAGE + TRAILER_LINES_PER_PAGE);

    let display_header_and_trailer = !(page_length_le_ht) && !matches.opt_present(NO_HEADER_TRAILER_OPTION);

    let content_lines_per_page = if page_length_le_ht {
        page_length
    } else {
        page_length - (HEADER_LINES_PER_PAGE + TRAILER_LINES_PER_PAGE)
    };

    let page_separator_char = matches.opt_str(FORM_FEED_OPTION).map(|_i| {
        '\u{000A}'.to_string()
    }).unwrap_or("\n".to_string());

    let column_width = match matches.opt_str(COLUMN_WIDTH_OPTION).map(|i| i.parse::<usize>()) {
        Some(res) => Some(res?),
        _ => None
    };

    let across_mode = matches.opt_present(ACROSS_OPTION);

    let column_mode_options = match matches.opt_str(COLUMN_OPTION).map(|i| {
        i.parse::<usize>()
    }) {
        Some(res) => {
            Some(ColumnModeOptions {
                columns: res?,
                width: match column_width {
                    Some(x) => Some(x),
                    None => Some(DEFAULT_COLUMN_WIDTH)
                },
                column_separator: DEFAULT_COLUMN_SEPARATOR.to_string(),
                across_mode,
            })
        }
        _ => None
    };

    Ok(OutputOptions {
        number: numbering_options,
        header,
        double_space,
        line_separator,
        content_line_separator,
        last_modified_time,
        start_page,
        end_page,
        display_header: display_header_and_trailer,
        display_trailer: display_header_and_trailer,
        content_lines_per_page,
        page_separator_char,
        column_mode_options,
    })
}

fn open(path: &str) -> Result<Box<Read>, PrError> {
    if path == FILE_STDIN {
        let stdin = stdin();
        return Ok(Box::new(stdin) as Box<Read>);
    }

    metadata(path).map(|i| {
        let path_string = path.to_string();
        match i.file_type() {
            #[cfg(unix)]
            ft if ft.is_block_device() =>
                {
                    Err(PrError::UnknownFiletype(path_string))
                }
            #[cfg(unix)]
            ft if ft.is_char_device() =>
                {
                    Err(PrError::UnknownFiletype(path_string))
                }
            #[cfg(unix)]
            ft if ft.is_fifo() =>
                {
                    Err(PrError::UnknownFiletype(path_string))
                }
            #[cfg(unix)]
            ft if ft.is_socket() =>
                {
                    Err(PrError::IsSocket(path_string))
                }
            ft if ft.is_dir() => Err(PrError::IsDirectory(path_string)),
            ft if ft.is_file() || ft.is_symlink() => Ok(Box::new(File::open(path).context(path)?) as Box<Read>),
            _ => Err(PrError::UnknownFiletype(path_string))
        }
    }).unwrap_or(Err(PrError::NotExists(path.to_string())))
}

fn pr(path: &str, options: &OutputOptions) -> Result<i32, PrError> {
    let start_page: &usize = options.start_page.as_ref().unwrap_or(&1);
    let last_page: Option<&usize> = options.end_page.as_ref();
    let lines_needed_per_page: usize = lines_to_read_for_page(options);
    let start_line_number: usize = get_start_line_number(options);

    let pages: GroupBy<usize, Map<TakeWhile<SkipWhile<Enumerate<Lines<BufReader<Box<Read>>>>, _>, _>, _>, _> =
        BufReader::with_capacity(READ_BUFFER_SIZE, open(path)?)
            .lines()
            .enumerate()
            .skip_while(|line_index: &(usize, Result<String, Error>)| {
                // Skip the initial lines if not in page range
                let start_line_index_of_start_page = (*start_page - 1) * lines_needed_per_page;
                line_index.0 < (start_line_index_of_start_page)
            })
            .take_while(|i: &(usize, Result<String, Error>)| {
                // Only read the file until provided last page reached
                last_page
                    .map(|lp| i.0 < ((*lp) * lines_needed_per_page))
                    .unwrap_or(true)
            })
            .map(|i: (usize, Result<String, Error>)| (i.0 + start_line_number, i.1)) // get display line number with line content
            .group_by(|i: &(usize, Result<String, Error>)| {
                ((i.0 - start_line_number + 1) as f64 / lines_needed_per_page as f64).ceil() as usize
            }); // group them by page number


    for (page_number, content_with_line_number) in pages.into_iter() {
        let mut lines: Vec<(usize, String)> = Vec::new();
        for line_number_and_line in content_with_line_number {
            let line_number: usize = line_number_and_line.0;
            let line: Result<String, Error> = line_number_and_line.1;
            let x = line?;
            lines.push((line_number, x));
        }

        print_page(&lines, options, &page_number);
    }


    return Ok(0);
}

fn print_page(lines: &Vec<(usize, String)>, options: &OutputOptions, page: &usize) -> Result<usize, Error> {
    let page_separator = options.page_separator_char.as_bytes();
    let header: Vec<String> = header_content(options, page);
    let trailer_content: Vec<String> = trailer_content(options);

    let out: &mut Stdout = &mut stdout();
    let line_separator = options.line_separator.as_bytes();

    out.lock();
    for x in header {
        out.write(x.as_bytes())?;
        out.write(line_separator)?;
    }

    let lines_written = write_columns(lines, options, out)?;

    for index in 0..trailer_content.len() {
        let x: &String = trailer_content.get(index).unwrap();
        out.write(x.as_bytes())?;
        if index + 1 != trailer_content.len() {
            out.write(line_separator)?;
        }
    }
    out.write(page_separator)?;
    out.flush()?;
    Ok(lines_written)
}

fn write_columns(lines: &Vec<(usize, String)>, options: &OutputOptions, out: &mut Stdout) -> Result<usize, Error> {
    let line_separator = options.content_line_separator.as_bytes();
    let content_lines_per_page = if options.double_space {
        options.content_lines_per_page / 2
    } else {
        options.content_lines_per_page
    };

    let width: usize = options
        .number.as_ref()
        .map(|i| i.width)
        .unwrap_or(0);
    let number_separator: String = options
        .number.as_ref()
        .map(|i| i.separator.to_string())
        .unwrap_or(NumberingMode::default().separator);

    let blank_line = "".to_string();
    let columns = get_columns(options);

    let col_sep: &String = options
        .column_mode_options.as_ref()
        .map(|i| &i.column_separator)
        .unwrap_or(&blank_line);

    let col_width: Option<usize> = options
        .column_mode_options.as_ref()
        .map(|i| i.width)
        .unwrap_or(None);

    let across_mode = options
        .column_mode_options.as_ref()
        .map(|i| i.across_mode)
        .unwrap_or(false);

    let mut lines_printed = 0;
    let is_number_mode = options.number.is_some();

    let fetch_indexes: Vec<Vec<usize>> = if across_mode {
        (0..content_lines_per_page)
            .map(|a|
                (0..columns)
                    .map(|i| a * columns + i)
                    .collect()
            ).collect()
    } else {
        (0..content_lines_per_page)
            .map(|start|
                (0..columns)
                    .map(|i| start + content_lines_per_page * i)
                    .collect()
            ).collect()
    };

    for fetch_index in fetch_indexes {
        let indexes = fetch_index.len();
        for i in 0..indexes {
            let index = fetch_index[i];
            if lines.get(index).is_none() {
                break;
            }
            let read_line: &String = &lines.get(index).unwrap().1;
            let next_line_number: usize = lines.get(index).unwrap().0;
            let trimmed_line = get_line_for_printing(
                next_line_number, &width,
                &number_separator, columns, col_width,
                read_line, is_number_mode);
            out.write(trimmed_line.as_bytes())?;
            if (i + 1) != indexes {
                out.write(col_sep.as_bytes())?;
            }
            lines_printed += 1;
        }
        out.write(line_separator)?;
    }
    Ok(lines_printed)
}

fn get_line_for_printing(line_number: usize, width: &usize,
                         separator: &String, columns: usize,
                         col_width: Option<usize>,
                         read_line: &String, is_number_mode: bool) -> String {
    let fmtd_line_number: String = if is_number_mode {
        get_fmtd_line_number(&width, line_number, &separator)
    } else {
        "".to_string()
    };
    let mut complete_line = format!("{}{}", fmtd_line_number, read_line);

    let tab_count: usize = complete_line
        .chars()
        .filter(|i| i == &TAB)
        .count();

    let display_length = complete_line.len() + (tab_count * 7);
// TODO Adjust the width according to -n option
// TODO actual len of the string vs display len of string because of tabs
    col_width.map(|i| {
        let min_width = (i - (columns - 1)) / columns;
        if display_length < min_width {
            for _i in 0..(min_width - display_length) {
                complete_line.push(' ');
            }
        }

        complete_line
            .chars()
            .take(min_width)
            .collect()
    }).unwrap_or(complete_line)
}

fn get_fmtd_line_number(width: &usize, line_number: usize, separator: &String) -> String {
    let line_str = line_number.to_string();
    if line_str.len() >= *width {
        format!("{:>width$}{}", &line_str[line_str.len() - *width..], separator, width = width)
    } else {
        format!("{:>width$}{}", line_str, separator, width = width)
    }
}


fn header_content(options: &OutputOptions, page: &usize) -> Vec<String> {
    if options.display_header {
        let first_line: String = format!("{} {} Page {}", options.last_modified_time, options.header, page);
        vec!["".to_string(), "".to_string(), first_line, "".to_string(), "".to_string()]
    } else {
        Vec::new()
    }
}

fn file_last_modified_time(path: &str) -> String {
    let file_metadata = metadata(path);
    return file_metadata.map(|i| {
        return i.modified().map(|x| {
            let datetime: DateTime<Local> = x.into();
            datetime.format("%b %d %H:%M %Y").to_string()
        }).unwrap_or(String::new());
    }).unwrap_or(String::new());
}

fn current_time() -> String {
    let datetime: DateTime<Local> = Local::now();
    datetime.format("%b %d %H:%M %Y").to_string()
}

fn trailer_content(options: &OutputOptions) -> Vec<String> {
    if options.as_ref().display_trailer {
        vec!["".to_string(), "".to_string(), "".to_string(), "".to_string(), "".to_string()]
    } else {
        Vec::new()
    }
}

/// Returns starting line number for the file to be printed.
/// If -N is specified the first line number changes otherwise
/// default is 1.
/// # Arguments
/// * `opts` - A reference to OutputOptions
fn get_start_line_number(opts: &OutputOptions) -> usize {
    opts.number
        .as_ref()
        .map(|i| i.first_number)
        .unwrap_or(1)
}

/// Returns number of lines to read from input for constructing one page of pr output.
/// If double space -d is used lines are halved.
/// If columns --columns is used the lines are multiplied by the value.
/// # Arguments
/// * `opts` - A reference to OutputOptions
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
/// # Arguments
/// * `opts` - A reference to OutputOptions
fn get_columns(opts: &OutputOptions) -> usize {
    opts.column_mode_options
        .as_ref()
        .map(|i| i.columns)
        .unwrap_or(1)
}
