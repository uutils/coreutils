#![crate_name = "expand"]
#![feature(collections, core, io, libc, path, rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Virgile Andreani <virgile.andreani@anbuco.fr>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#![feature(box_syntax)]

extern crate getopts;
extern crate libc;

use std::old_io as io;
use std::str::StrExt;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "expand";
static VERSION: &'static str = "0.0.1";

static DEFAULT_TABSTOP: usize = 8;

fn tabstops_parse(s: String) -> Vec<usize> {
    let words = s.as_slice().split(',').collect::<Vec<&str>>();

    let nums = words.into_iter()
        .map(|sn| sn.parse::<usize>()
            .unwrap_or_else(
                |_| crash!(1, "{}\n", "tab size contains invalid character(s)"))
            )
        .collect::<Vec<usize>>();

    if nums.iter().any(|&n| n == 0) {
        crash!(1, "{}\n", "tab size cannot be 0");
    }

    match nums.iter().fold((true, 0), |(acc, last), &n| (acc && last <= n, n)) {
        (false, _) => crash!(1, "{}\n", "tab sizes must be ascending"),
        _ => {}
    }

    nums
}

struct Options {
    files: Vec<String>,
    tabstops: Vec<usize>,
    iflag: bool
}

impl Options {
    fn new(matches: getopts::Matches) -> Options {
        let tabstops = match matches.opt_str("t") {
            None => vec!(DEFAULT_TABSTOP),
            Some(s) => tabstops_parse(s)
        };

        let iflag = matches.opt_present("i");

        let files =
            if matches.free.is_empty() {
                vec!("-".to_string())
            } else {
                matches.free
            };

        Options { files: files, tabstops: tabstops, iflag: iflag }
    }
}

pub fn uumain(args: Vec<String>) -> isize {
    let opts = [
        getopts::optflag("i", "initial", "do not convert tabs after non blanks"),
        getopts::optopt("t", "tabs", "have tabs NUMBER characters apart, not 8", "NUMBER"),
        getopts::optopt("t", "tabs", "use comma separated list of explicit tab positions", "LIST"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f)
    };

    if matches.opt_present("help") {
        println!("Usage: {} [OPTION]... [FILE]...", NAME);
        io::print(getopts::usage(
            "Convert tabs in each FILE to spaces, writing to standard output.\n\
            With no FILE, or when FILE is -, read standard input.", &opts).as_slice());
        return 0;
    }

    if matches.opt_present("V") {
        println!("{} v{}", NAME, VERSION);
        return 0;
    }

    expand(Options::new(matches));

    return 0;
}

fn open(path: String) -> io::BufferedReader<Box<Reader+'static>> {
    let mut file_buf;
    if path.as_slice() == "-" {
        io::BufferedReader::new(box io::stdio::stdin_raw() as Box<Reader>)
    } else {
        file_buf = match io::File::open(&Path::new(path.as_slice())) {
            Ok(a) => a,
            _ => crash!(1, "{}: {}\n", path, "No such file or directory")
        };
        io::BufferedReader::new(box file_buf as Box<Reader>)
    }
}

fn to_next_stop(tabstops: &[usize], col: usize) -> usize {
    match tabstops.as_slice() {
        [tabstop] => tabstop - col % tabstop,
        tabstops => match tabstops.iter().skip_while(|&t| *t <= col).next() {
            Some(&tabstop) => tabstop - col % tabstop,
            None => 1
        }
    }
}

fn expand(options: Options) {
    let mut output = io::stdout();

    for file in options.files.into_iter() {
        let mut col = 0;
        let mut init = true;
        for c in open(file).chars() {
            match c {
                Ok('\t') if init || !options.iflag => {
                    let nb_spaces = to_next_stop(options.tabstops.as_slice(), col);
                    col += nb_spaces;
                    safe_write!(&mut output, "{:1$}", "", nb_spaces);
                }
                Ok('\x08') => {
                    if col > 0 {
                        col -= 1;
                    }
                    init = false;
                    safe_write!(&mut output, "{}", '\x08');
                }
                Ok('\n') =>  {
                    col = 0;
                    init = true;
                    safe_write!(&mut output, "{}", '\n');
                }
                Ok(c) => {
                    col += 1;
                    if c != ' ' {
                        init = false;
                    }
                    safe_write!(&mut output, "{}", c);
                }
                Err(_) => break
            }
        }
    }
}

