#![crate_name = "uu_pr"]

// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.
//

extern crate getopts;
extern crate chrono;

//#[macro_use]
//extern crate uucore;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::vec::Vec;
use chrono::offset::Local;
use chrono::DateTime;
use getopts::Matches;

//use uucore::fs::is_stdin_interactive;


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


pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optopt(
        "h",
        "",
        "Use the string header to replace the file name \
     in the header line.",
        "STRING",
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


    if matches.opt_present("help") || matches.free.is_empty() {
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


    let mut files = matches.free.clone();
    if files.is_empty() {
        //For stdin
        files.push("-".to_owned());
    }

    for f in files {
        let header: String = matches.opt_str("h").unwrap_or(f.to_string());
        let options = build_options(&matches, header);
        pr(&f, options);
    }

    0
}

fn build_options(matches: &Matches, header: String) -> OutputOptions {
    let numbering_options = matches.opt_str("n").map(|i| {
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
    OutputOptions {
        number: numbering_options,
        header,
    }
}

fn pr(path: &str, options: OutputOptions) -> std::io::Result<()> {
    let file = File::open(path)?;
    let file_last_modified_time = file_last_modified_time(path);
    let lines = BufReader::new(file).lines();
    let mut i = 0;
    let mut page: usize = 0;
    let mut buffered_content: Vec<String> = Vec::new();
    for line in lines {
        if i == CONTENT_LINES_PER_PAGE {
            page = page + 1;
            i = 0;
            flush_buffered_page(&file_last_modified_time, &mut buffered_content, &options, page);
        }
        i = i + 1;
        buffered_content.push(line?);
    }
    if i != 0 {
        page = page + 1;
        flush_buffered_page(&file_last_modified_time, &mut buffered_content, &options, page);
    }
    Ok(())
}

fn print_page(header_content: &Vec<String>, lines: &Vec<String>, options: &OutputOptions, page: usize) -> String {
    let mut page_content: Vec<String> = Vec::new();
    let trailer_content = trailer_content();
    assert_eq!(lines.len() <= CONTENT_LINES_PER_PAGE, true, "Only {} lines of content allowed in a pr output page", CONTENT_LINES_PER_PAGE.to_string());
    assert_eq!(header_content.len(), HEADER_LINES_PER_PAGE, "Only {} lines of content allowed in a pr header", HEADER_LINES_PER_PAGE.to_string());
    assert_eq!(trailer_content.len(), TRAILER_LINES_PER_PAGE, "Only {} lines of content allowed in a pr trailer", TRAILER_LINES_PER_PAGE.to_string());
    for x in header_content {
        page_content.push(x.to_string());
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
            page_content.push(x.to_string());
        } else {
            let fmtd_line_number: String = get_fmtd_line_number(width, prev_lines + i, &separator);
            page_content.push(format!("{}{}", fmtd_line_number, x.to_string()));
        }
        i = i + 1;
    }
    page_content.extend(trailer_content);
    page_content.join("\n")
}

fn get_fmtd_line_number(width: usize, line_number: usize, separator: &String) -> String {
    format!("{:>width$}{}", take_last_n(&line_number.to_string(), width), separator, width = width)
}

fn take_last_n(s: &String, n: usize) -> &str {
    if s.len() >= n {
        &s[s.len() - n..]
    } else {
        s
    }
}

fn flush_buffered_page(file_last_modified_time: &String, buffered_content: &mut Vec<String>, options: &OutputOptions, page: usize) {
    let header = header_content(file_last_modified_time, &options.header, page);
    print!("{}", print_page(&header, buffered_content, &options, page));
    buffered_content.clear();
}

fn header_content(last_modified: &String, header: &String, page: usize) -> Vec<String> {
    let first_line: String = format!("{} {} Page {}", last_modified, header, page.to_string());
    vec!["".to_string(), "".to_string(), first_line, "".to_string(), "".to_string()]
}

fn file_last_modified_time(path: &str) -> String {
    let file_metadata = fs::metadata(path);
    return file_metadata.map(|i| {
        return i.modified().map(|x| {
            let datetime: DateTime<Local> = x.into();
            datetime.format("%b %d %H:%M %Y").to_string()
        }).unwrap_or(String::new());
    }).unwrap_or(String::new());
}

fn trailer_content() -> Vec<String> {
    vec!["".to_string(), "".to_string(), "".to_string(), "".to_string(), "".to_string()]
}
