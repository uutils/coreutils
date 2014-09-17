#![crate_name = "expand"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Virgile Andreani <virgile.andreani@anbuco.fr>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#![feature(macro_rules)]

extern crate getopts;
extern crate libc;

use std::io;
use std::from_str;

#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "expand";
static VERSION: &'static str = "0.0.1";

static DEFAULT_TABSTOP: uint = 8;

fn tabstops_parse(s: String) -> Vec<uint> {
    let words = s.as_slice().split(',').collect::<Vec<&str>>();

    let nums = words.into_iter()
        .map(|sn| from_str::from_str::<uint>(sn)
            .unwrap_or_else(
                || crash!(1, "{}\n", "tab size contains invalid character(s)"))
            )
        .collect::<Vec<uint>>();

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
    tabstops: Vec<uint>,
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

pub fn uumain(args: Vec<String>) -> int {
    let opts = [
        getopts::optflag("i", "initial", "do not convert tabs after non blanks"),
        getopts::optopt("t", "tabs", "have tabs NUMBER characters apart, not 8", "NUMBER"),
        getopts::optopt("t", "tabs", "use comma separated list of explicit tab positions", "LIST"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit"),
    ];

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f)
    };

    if matches.opt_present("help") {
        println!("Usage: {:s} [OPTION]... [FILE]...", NAME);
        io::print(getopts::usage(
            "Convert tabs in each FILE to spaces, writing to standard output.\n\
            With no FILE, or when FILE is -, read standard input.", opts).as_slice());
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

fn to_next_stop(tabstops: &[uint], col: uint) -> uint {
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
                    safe_write!(output, "{:1$s}", "", nb_spaces);
                }
                Ok('\x08') => {
                    if col > 0 {
                        col -= 1;
                    }
                    init = false;
                    safe_write!(output, "{:c}", '\x08');
                }
                Ok('\n') =>  {
                    col = 0;
                    init = true;
                    safe_write!(output, "{:c}", '\n');
                }
                Ok(c) => {
                    col += 1;
                    if c != ' ' {
                        init = false;
                    }
                    safe_write!(output, "{:c}", c);
                }
                Err(_) => break
            }
        }
    }
}

