#![crate_name = "uu_pr"]

// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.
//

extern crate getopts;

//#[macro_use]
//extern crate uucore;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::vec::Vec;
//use uucore::fs::is_stdin_interactive;


static NAME: &str = "pr";
static VERSION: &str = env!("CARGO_PKG_VERSION");
static LINES_PER_PAGE: usize = 66;
static HEADER_LINES_PER_PAGE: usize = 5;
static TRAILER_LINES_PER_PAGE: usize = 5;
static CONTENT_LINES_PER_PAGE: usize = LINES_PER_PAGE - HEADER_LINES_PER_PAGE - TRAILER_LINES_PER_PAGE;

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = getopts::Options::new();

    opts.optopt(
        "h",
        "",
        "Use the string header to replace the file name \
     in the header line.",
        "STRING",
    );
    opts.optflag("h", "help", "display this help and exit");
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
        println!(
            "{}",
            opts.usage(
                "The pr utility is a printing and pagination filter
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
     Lines are not truncated under single column output."
            )
        );
        if matches.free.is_empty() {
            return 1;
        }
        return 0;
    }

    let path = &matches.free[0];
    open(&path);

    0
}

fn open(path: &str) -> std::io::Result<()> {
    let file = File::open(path)?;
    let lines = BufReader::new(file).lines();
    let mut i = 0;
    let mut page: i32 = 0;
    let mut buffered_content: Vec<String> = Vec::new();
    for line in lines {
        if i == CONTENT_LINES_PER_PAGE {
            page = page + 1;
            i = 0;
            print!("{}", print_page(&buffered_content));
            buffered_content.clear();
        }
        i = i + 1;
        buffered_content.push(line?);
    }
    if i != 0 {
        print!("{}", print_page(&buffered_content));
        buffered_content.clear();
    }
    Ok(())
}

fn print_page(lines: &Vec<String>) -> String {
    let mut page_content: Vec<String> = Vec::new();
    let header_content = header_content();
    let trailer_content = trailer_content();
    assert_eq!(lines.len() <= CONTENT_LINES_PER_PAGE, true, "Only {} lines of content allowed in a pr output page", CONTENT_LINES_PER_PAGE.to_string());
    assert_eq!(header_content.len(), HEADER_LINES_PER_PAGE, "Only {} lines of content allowed in a pr header", HEADER_LINES_PER_PAGE.to_string());
    assert_eq!(trailer_content.len(), TRAILER_LINES_PER_PAGE, "Only {} lines of content allowed in a pr trailer", TRAILER_LINES_PER_PAGE.to_string());
    page_content.extend(header_content);
    for x in lines {
        page_content.push(x.to_string());
    }
    page_content.extend(trailer_content);
    page_content.join("\n")
}

fn header_content() -> Vec<String> {
    vec!["".to_string(), "".to_string(), "".to_string(), "".to_string(), "".to_string()]
}

fn trailer_content() -> Vec<String> {
    vec!["".to_string(), "".to_string(), "".to_string(), "".to_string(), "".to_string()]
}
