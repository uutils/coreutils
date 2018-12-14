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
extern crate chrono;
extern crate getopts;
extern crate uucore;

use std::io::{BufRead, BufReader, stdin, stdout, stderr, Error, Read, Write, Stdout};
use std::vec::Vec;
use chrono::offset::Local;
use chrono::DateTime;
use getopts::{Matches, Options};
use std::fs::{metadata, File};
#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
use quick_error::ResultExt;
use std::convert::From;
use getopts::HasArg;
use getopts::Occur;

static NAME: &str = "pr";
static VERSION: &str = env!("CARGO_PKG_VERSION");
static LINES_PER_PAGE: usize = 66;
static HEADER_LINES_PER_PAGE: usize = 5;
static TRAILER_LINES_PER_PAGE: usize = 5;
static NUMBERING_MODE_DEFAULT_SEPARATOR: &str = "\t";
static NUMBERING_MODE_DEFAULT_WIDTH: usize = 5;
static STRING_HEADER_OPTION: &str = "h";
static DOUBLE_SPACE_OPTION: &str = "d";
static NUMBERING_MODE_OPTION: &str = "n";
static PAGE_RANGE_OPTION: &str = "pages";
static NO_HEADER_TRAILER_OPTION: &str = "t";
static PAGE_LENGTH_OPTION: &str = "l";
static SUPPRESS_PRINTING_ERROR: &str = "r";
static FORM_FEED_OPTION: &str = "F";
static COLUMN_WIDTH_OPTION: &str = "w";
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
    last_modified_time: String,
    start_page: Option<usize>,
    end_page: Option<usize>,
    display_header: bool,
    display_trailer: bool,
    content_lines_per_page: usize,
    suppress_errors: bool,
    page_separator_char: String,
    column_mode_options: Option<ColumnModeOptions>,
}

struct ColumnModeOptions {
    width: usize,
    columns: usize,
    column_separator: String,
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
}

impl Default for NumberingMode {
    fn default() -> NumberingMode {
        NumberingMode {
            width: NUMBERING_MODE_DEFAULT_WIDTH,
            separator: NUMBERING_MODE_DEFAULT_SEPARATOR.to_string(),
        }
    }
}

enum InputType {
    Directory,
    File,
    StdIn,
    SymLink,
    #[cfg(unix)]
    BlockDevice,
    #[cfg(unix)]
    CharacterDevice,
    #[cfg(unix)]
    Fifo,
    #[cfg(unix)]
    Socket,
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
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optflagopt(
        "",
        PAGE_RANGE_OPTION,
        "Begin and stop printing with page FIRST_PAGE[:LAST_PAGE]",
        "FIRST_PAGE[:LAST_PAGE]",
    );

    opts.optopt(
        STRING_HEADER_OPTION,
        "header",
        "Use the string header to replace the file name \
     in the header line.",
        "STRING"
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

    opts.optflagopt(
        NUMBERING_MODE_OPTION,
        "",
        "Provide width digit line numbering.  The default for width, if not specified, is 5.  The number occupies
           the first width column positions of each text column or each line of -m output.  If char (any nondigit
           character) is given, it is appended to the line number to separate it from whatever follows.  The default
           for char is a <tab>.  Line numbers longer than width columns are truncated.",
        "[char][width]"
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
        //For stdin
        files.push(FILE_STDIN.to_owned());
    }

    if matches.opt_present("help") {
        return print_usage(&mut opts, &matches);
    }

    for f in files {
        let header: &String = &matches.opt_str(STRING_HEADER_OPTION).unwrap_or(f.to_string());
        let result_options = build_options(&matches, header, &f);
        if result_options.is_err() {
            writeln!(&mut stderr(), "{}", result_options.err().unwrap());
            return 1;
        }
        let options = &result_options.unwrap();
        let status: i32 = match pr(&f, options) {
            Err(error) => {
                if !options.suppress_errors {
                    writeln!(&mut stderr(), "{}", error);
                }
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

fn build_options(matches: &Matches, header: &String, path: &String) -> Result<OutputOptions, PrError> {
    let numbering_options: Option<NumberingMode> = matches.opt_str(NUMBERING_MODE_OPTION).map(|i| {
        NumberingMode {
            width: i.parse::<usize>().unwrap_or(NumberingMode::default().width),
            separator: NumberingMode::default().separator,
        }
    }).or_else(|| {
        if matches.opt_present(NUMBERING_MODE_OPTION) {
            return Some(NumberingMode::default());
        }
        return None;
    });

    let double_space = matches.opt_present(DOUBLE_SPACE_OPTION);

    let line_separator: String = if double_space {
        "\n\n".to_string()
    } else {
        "\n".to_string()
    };

    let last_modified_time = if path.eq(FILE_STDIN) {
        current_time()
    } else {
        file_last_modified_time(path)
    };

    let start_page = match matches.opt_str(PAGE_RANGE_OPTION).map(|i| {
        let x: Vec<&str> = i.split(":").collect();
        x[0].parse::<usize>()
    }) {
        Some(res) => Some(res?),
        _ => None
    };

    let end_page = match matches.opt_str(PAGE_RANGE_OPTION)
        .filter(|i| i.contains(":"))
        .map(|i| {
            let x: Vec<&str> = i.split(":").collect();
            x[1].parse::<usize>()
        }) {
        Some(res) => Some(res?),
        _ => None
    };

    if start_page.is_some() && end_page.is_some() && start_page.unwrap() > end_page.unwrap() {
        return Err(PrError::EncounteredErrors(format!("invalid page range ‘{}:{}’", start_page.unwrap(), end_page.unwrap())));
    }

    let page_length = match matches.opt_str(PAGE_LENGTH_OPTION).map(|i| {
        i.parse::<usize>()
    }) {
        Some(res) => res?,
        _ => LINES_PER_PAGE
    };

    let content_lines_per_page = page_length - (HEADER_LINES_PER_PAGE - TRAILER_LINES_PER_PAGE);

    let display_header_and_trailer = !(page_length < (HEADER_LINES_PER_PAGE + TRAILER_LINES_PER_PAGE))
        && !matches.opt_present(NO_HEADER_TRAILER_OPTION);

    let suppress_errors = matches.opt_present(SUPPRESS_PRINTING_ERROR);

    let page_separator_char = matches.opt_str(FORM_FEED_OPTION).map(|i| {
        '\u{000A}'.to_string()
    }).unwrap_or("\n".to_string());

    let column_width = match matches.opt_str(COLUMN_WIDTH_OPTION).map(|i| i.parse::<usize>()) {
        Some(res) => res?,
        _ => DEFAULT_COLUMN_WIDTH
    };

    let column_mode_options = match matches.opt_str(COLUMN_OPTION).map(|i| {
        i.parse::<usize>()
    }) {
        Some(res) => {
            Some(ColumnModeOptions {
                columns: res?,
                width: column_width,
                column_separator: DEFAULT_COLUMN_SEPARATOR.to_string(),
            })
        }
        _ => None
    };

    Ok(OutputOptions {
        number: numbering_options,
        header: header.to_string(),
        double_space,
        line_separator,
        last_modified_time,
        start_page,
        end_page,
        display_header: display_header_and_trailer,
        display_trailer: display_header_and_trailer,
        content_lines_per_page,
        suppress_errors,
        page_separator_char,
        column_mode_options,
    })
}

fn open(path: &str) -> Result<Box<Read>, PrError> {
    match get_input_type(path) {
        Some(InputType::Directory) => Err(PrError::IsDirectory(path.to_string())),
        #[cfg(unix)]
        Some(InputType::Socket) => {
            Err(PrError::IsSocket(path.to_string()))
        }
        Some(InputType::StdIn) => {
            let stdin = stdin();
            Ok(Box::new(stdin) as Box<Read>)
        }
        Some(_) => Ok(Box::new(File::open(path).context(path)?) as Box<Read>),
        None => Err(PrError::UnknownFiletype(path.to_string()))
    }
}

fn pr(path: &str, options: &OutputOptions) -> Result<i32, PrError> {
    let mut i = 0;
    let mut page: usize = 0;
    let mut buffered_content: Vec<String> = Vec::new();
    let content_lines_per_page = options.as_ref().content_lines_per_page;
    let columns = options.as_ref().column_mode_options.as_ref().map(|i| i.columns).unwrap_or(1);
    let lines_per_page = if options.as_ref().double_space {
        (content_lines_per_page / 2) * columns
    } else {
        content_lines_per_page * columns
    };
    for line in BufReader::with_capacity(READ_BUFFER_SIZE, open(path)?).lines() {
        if i == lines_per_page {
            page = page + 1;
            i = 0;
            if !_is_within_page_range(options, &page) {
                return Ok(0)
            }
            print_page(&buffered_content, options, &page)?;
            buffered_content = Vec::new();
        }
        buffered_content.push(line?);
        i = i + 1;
    }

    if i != 0 {
        if !_is_within_page_range(options, &page) {
            return Ok(0)
        }
        page = page + 1;
        print_page(&buffered_content, options, &page)?;
    }

    return Ok(0);
}

fn _is_within_page_range(options: &OutputOptions, page: &usize) -> bool {
    let start_page = options.as_ref().start_page.as_ref();
    let last_page = options.as_ref().end_page.as_ref();
    (start_page.is_none() || page >= start_page.unwrap()) && (last_page.is_none() || page <= last_page.unwrap())
}

fn print_page(lines: &Vec<String>, options: &OutputOptions, page: &usize) -> Result<usize, Error> {
    let page_separator = options.as_ref().page_separator_char.as_bytes();
    let header: Vec<String> = header_content(options, page);
    let trailer_content: Vec<String> = trailer_content(options);

    let out: &mut Stdout = &mut stdout();
    let line_separator = options.as_ref().line_separator.as_bytes();
    let mut lines_written = 0;

    out.lock();
    for x in header {
        out.write(x.as_bytes())?;
        out.write(line_separator)?;
        lines_written += 1;
    }

    lines_written += write_columns(lines, options, page_separator, out, line_separator, page)?;

    for index in 0..trailer_content.len() {
        let x: &String = trailer_content.get(index).unwrap();
        out.write(x.as_bytes())?;
        if index + 1 == trailer_content.len() {
            out.write(page_separator)?;
        } else {
            out.write(line_separator)?;
        }
        lines_written += 1;
    }
    out.flush()?;
    Ok(lines_written)
}

fn write_columns(lines: &Vec<String>, options: &OutputOptions, page_separator: &[u8], out: &mut Stdout, line_separator: &[u8], page: &usize) -> Result<usize, Error> {
    let content_lines_per_page = options.as_ref().content_lines_per_page;
    let prev_lines = content_lines_per_page * (page - 1);
    let width: usize = options.as_ref()
        .number.as_ref()
        .map(|i| i.width)
        .unwrap_or(0);
    let separator: String = options.as_ref()
        .number.as_ref()
        .map(|i| i.separator.to_string())
        .unwrap_or(NumberingMode::default().separator);

    let mut i = 0;
    for x in lines {
        if options.number.is_none() {
            out.write(x.as_bytes())?;
        } else {
            let fmtd_line_number: String = get_fmtd_line_number(&width, prev_lines + i, &separator);
            out.write(format!("{}{}", fmtd_line_number, x).as_bytes())?;
        }

        if i == lines.len() {
            out.write(page_separator)?;
        } else {
            out.write(line_separator)?;
        }
        i += 1;
    }
    Ok(i)
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
    if options.as_ref().display_header {
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

fn get_input_type(path: &str) -> Option<InputType> {
    if path == FILE_STDIN {
        return Some(InputType::StdIn);
    }

    metadata(path).map(|i| {
        match i.file_type() {
            #[cfg(unix)]
            ft if ft.is_block_device() =>
                {
                    Some(InputType::BlockDevice)
                }
            #[cfg(unix)]
            ft if ft.is_char_device() =>
                {
                    Some(InputType::CharacterDevice)
                }
            #[cfg(unix)]
            ft if ft.is_fifo() =>
                {
                    Some(InputType::Fifo)
                }
            #[cfg(unix)]
            ft if ft.is_socket() =>
                {
                    Some(InputType::Socket)
                }
            ft if ft.is_dir() => Some(InputType::Directory),
            ft if ft.is_file() => Some(InputType::File),
            ft if ft.is_symlink() => Some(InputType::SymLink),
            _ => None
        }
    }).unwrap_or(None)
}
