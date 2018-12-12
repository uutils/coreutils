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


static NAME: &str = "pr";
static VERSION: &str = env!("CARGO_PKG_VERSION");
static LINES_PER_PAGE: usize = 66;
static HEADER_LINES_PER_PAGE: usize = 5;
static TRAILER_LINES_PER_PAGE: usize = 5;
static CONTENT_LINES_PER_PAGE: usize = LINES_PER_PAGE - HEADER_LINES_PER_PAGE - TRAILER_LINES_PER_PAGE;
static NUMBERING_MODE_DEFAULT_SEPARATOR: &str = "\t";
static NUMBERING_MODE_DEFAULT_WIDTH: usize = 5;

struct OutputOptions {
    /// Line numbering mode
    number: Option<NumberingMode>,
    header: String,
    double_spaced: bool,
    line_separator: String,
    last_modified_time: String,
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
            display("pr: {0} encountered", msg)
        }
        IsDirectory(path: String) {
            display("pr: {0}: Is a directory", path)
        }
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optopt(
        "h",
        "",
        "Use the string header to replace the file name \
     in the header line.",
        "STRING",
    );

    opts.optflag(
        "d",
        "",
        "Produce output that is double spaced. An extra <newline> character is output following every <newline>
           found in the input.",
    );

    opts.optflagopt(
        "n",
        "",
        "Provide width digit line numbering.  The default for width, if not specified, is 5.  The number occupies
           the first width column positions of each text column or each line of -m output.  If char (any nondigit
           character) is given, it is appended to the line number to separate it from whatever follows.  The default
           for char is a <tab>.  Line numbers longer than width columns are truncated.",
        "[char][width]",
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
        files.push("-".to_owned());
    }

    if matches.opt_present("help") {
        return print_usage(&mut opts, &matches);
    }

    for f in files {
        let header: &String = &matches.opt_str("h").unwrap_or(f.to_string());
        let options: &OutputOptions = &build_options(&matches, header, &f);
        let status: i32 = pr(&f, options);
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

fn build_options(matches: &Matches, header: &String, path: &String) -> OutputOptions {
    let numbering_options: Option<NumberingMode> = matches.opt_str("n").map(|i| {
        NumberingMode {
            width: i.parse::<usize>().unwrap_or(NumberingMode::default().width),
            separator: NumberingMode::default().separator,
        }
    }).or_else(|| {
        if matches.opt_present("n") {
            return Some(NumberingMode::default());
        }
        return None;
    });

    let line_separator: String = if matches.opt_present("d") {
        "\n\n".to_string()
    } else {
        "\n".to_string()
    };

    let last_modified_time = if path.eq("-") {
        current_time()
    } else {
        file_last_modified_time(path)
    };

    OutputOptions {
        number: numbering_options,
        header: header.to_string(),
        double_spaced: matches.opt_present("d"),
        line_separator,
        last_modified_time,
    }
}

fn open(path: &str) -> Result<Box<Read>, PrError> {
    match get_input_type(path) {
        Some(InputType::Directory) => Err(PrError::IsDirectory(path.to_string())),
        #[cfg(unix)]
        Some(InputType::Socket) => {
            // TODO Add reading from socket
            Err(PrError::EncounteredErrors("Reading from socket not supported yet".to_string()))
        }
        Some(InputType::StdIn) => {
            let stdin = stdin();
            Ok(Box::new(stdin) as Box<Read>)
        }
        Some(_) => Ok(Box::new(File::open(path).context(path)?) as Box<Read>),
        None => Err(PrError::UnknownFiletype(path.to_string()))
    }
}

fn pr(path: &str, options: &OutputOptions) -> i32 {
    let mut i = 0;
    let mut page: usize = 0;
    let mut buffered_content: Vec<String> = Vec::new();
    match open(path) {
        Ok(reader) => {
            // TODO Replace the loop
            for line in BufReader::new(reader).lines() {
                if i == CONTENT_LINES_PER_PAGE {
                    page = page + 1;
                    i = 0;
                    prepare_page(&mut buffered_content, options, &page);
                }
                match line {
                    Ok(content) => buffered_content.push(content),
                    Err(error) => {
                        writeln!(&mut stderr(), "pr: Unable to read from input type {}\n{}", path, error.to_string());
                        return -1;
                    }
                }
                i = i + 1;
            }

            if i != 0 {
                page = page + 1;
                prepare_page(&mut buffered_content, options, &page);
            }
        }
        Err(error) => {
            writeln!(&mut stderr(), "{}", error);
            return -1;
        }
    }
    return 0;
}

fn print_page(header_content: &Vec<String>, lines: &Vec<String>, options: &OutputOptions, page: &usize) {
    let trailer_content: Vec<String> = trailer_content();
    assert_eq!(lines.len() <= CONTENT_LINES_PER_PAGE, true, "Only {} lines of content allowed in a pr output page", CONTENT_LINES_PER_PAGE);
    assert_eq!(header_content.len(), HEADER_LINES_PER_PAGE, "Only {} lines of content allowed in a pr header", HEADER_LINES_PER_PAGE);
    assert_eq!(trailer_content.len(), TRAILER_LINES_PER_PAGE, "Only {} lines of content allowed in a pr trailer", TRAILER_LINES_PER_PAGE);
    let out: &mut Stdout = &mut stdout();
    let line_separator = options.as_ref().line_separator.as_bytes();

    out.lock();
    for x in header_content {
        out.write(x.as_bytes());
        out.write(line_separator);
    }

    let width: usize = options.as_ref()
        .number.as_ref()
        .map(|i| i.width)
        .unwrap_or(0);
    let separator: String = options.as_ref()
        .number.as_ref()
        .map(|i| i.separator.to_string())
        .unwrap_or(NumberingMode::default().separator);

    let prev_lines = CONTENT_LINES_PER_PAGE * (page - 1);
    let mut i = 1;
    for x in lines {
        if options.number.is_none() {
            out.write(x.as_bytes());
        } else {
            let fmtd_line_number: String = get_fmtd_line_number(&width, prev_lines + i, &separator);
            out.write(format!("{}{}", fmtd_line_number, x).as_bytes());
        }
        out.write(line_separator);
        i = i + 1;
    }
    for x in trailer_content {
        out.write(x.as_bytes());
        out.write(line_separator);
    }
    out.flush();
}

fn get_fmtd_line_number(width: &usize, line_number: usize, separator: &String) -> String {
    let line_str = line_number.to_string();
    if line_str.len() >= *width {
        format!("{:>width$}{}", &line_str[line_str.len() - *width..], separator, width = width)
    } else {
        format!("{:>width$}{}", line_str, separator, width = width)
    }
}


fn prepare_page(buffered_content: &mut Vec<String>, options: &OutputOptions, page: &usize) {
    let header: Vec<String> = header_content(&options, &page);
    print_page(&header, buffered_content, &options, &page);
    buffered_content.clear();
}

fn header_content(options: &OutputOptions, page: &usize) -> Vec<String> {
    let first_line: String = format!("{} {} Page {}", options.last_modified_time, options.header, page);
    vec!["".to_string(), "".to_string(), first_line, "".to_string(), "".to_string()]
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

fn trailer_content() -> Vec<String> {
    vec!["".to_string(), "".to_string(), "".to_string(), "".to_string(), "".to_string()]
}

fn get_input_type(path: &str) -> Option<InputType> {
    if path == "-" {
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
