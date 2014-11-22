#![crate_name = "unexpand"]

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
use std::str::from_str;

#[path = "../common/util.rs"]
mod util;

static NAME: &'static str = "unexpand";
static VERSION: &'static str = "0.0.1";

static DEFAULT_TABSTOP: uint = 8;

fn tabstops_parse(s: String) -> Vec<uint> {
    let words = s.as_slice().split(',').collect::<Vec<&str>>();

    let nums = words.into_iter()
        .map(|sn| from_str::<uint>(sn)
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
    aflag: bool
}

impl Options {
    fn new(matches: getopts::Matches) -> Options {
        let tabstops = match matches.opt_str("t") {
            None => vec!(DEFAULT_TABSTOP),
            Some(s) => tabstops_parse(s)
        };

        let aflag = (matches.opt_present("all") || matches.opt_present("tabs"))
                    && !matches.opt_present("first-only");

        let files =
            if matches.free.is_empty() {
                vec!("-".to_string())
            } else {
                matches.free
            };

        Options { files: files, tabstops: tabstops, aflag: aflag }
    }
}

pub fn uumain(args: Vec<String>) -> int {
    let opts = [
        getopts::optflag("a", "all", "convert all blanks, instead of just initial blanks"),
        getopts::optflag("", "first-only", "convert only leading sequences of blanks (overrides -a)"),
        getopts::optopt("t", "tabs", "have tabs N characters apart instead of 8 (enables -a)", "N"),
        getopts::optopt("t", "tabs", "use comma separated LIST of tab positions (enables -a)", "LIST"),
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
            "Convert blanks in each FILE to tabs, writing to standard output.\n\
            With no FILE, or when FILE is -, read standard input.", &opts).as_slice());
        return 0;
    }

    if matches.opt_present("V") {
        println!("{} v{}", NAME, VERSION);
        return 0;
    }

    unexpand(Options::new(matches));

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

fn is_tabstop(tabstops: &[uint], col: uint) -> bool {
    match tabstops {
        [tabstop] => col % tabstop == 0,
        tabstops => tabstops.binary_search(|&e| e.cmp(&col)).found().is_some()
    }
}

fn to_next_stop(tabstops: &[uint], col: uint) -> Option<uint> {
    match tabstops {
        [tabstop] => Some(tabstop - col % tabstop),
        tabstops => tabstops.iter().skip_while(|&t| *t <= col).next()
            .map(|&tabstop| tabstop - col % tabstop)
    }
}

fn unexpandspan(mut output: &mut io::LineBufferedWriter<io::stdio::StdWriter>,
                tabstops: &[uint], nspaces: uint, col: uint, init: bool) {
    let mut cur = col - nspaces;
    if nspaces > 1 || init {
        loop {
            match to_next_stop(tabstops, cur) {
                Some(to_next) if cur + to_next <= col => {
                        safe_write!(&mut output, "{}", '\t');
                        cur += to_next;
                    }
                _ => break
            }
        }
    }
    safe_write!(&mut output, "{:1$}", "", col - cur);
}

fn unexpand(options: Options) {
    let mut output = io::stdout();
    let ts = options.tabstops.as_slice();

    for file in options.files.into_iter() {
        let mut col = 0;
        let mut nspaces = 0;
        let mut init = true;
        for c in open(file).chars() {
            match c {
                Ok(' ') => {
                    if init || options.aflag {
                        nspaces += 1;
                    } else {
                        nspaces = 0;
                        safe_write!(&mut output, "{}", ' ');
                    }
                    col += 1;
                }
                Ok('\t') if nspaces > 0 => {
                    if is_tabstop(ts, col) {
                        nspaces = 0;
                        col += 1;
                        safe_write!(&mut output, "{}", '\t');
                    }
                    match to_next_stop(ts, col) {
                        Some(to_next) => {
                            nspaces += to_next;
                            col += to_next;
                        }
                        None => {
                            col += 1;
                            unexpandspan(&mut output, ts, nspaces, col, init);
                            nspaces = 0;
                            safe_write!(&mut output, "{}", '\t');
                        }
                    }
                }
                Ok('\x08') => { // '\b'
                    if init || options.aflag {
                        unexpandspan(&mut output, ts, nspaces, col, init)
                    }
                    nspaces = 0;
                    if col > 0 { col -= 1; }
                    init = false;
                    safe_write!(&mut output, "{}", '\x08');
                }
                Ok('\n') => {
                    if init || options.aflag {
                        unexpandspan(&mut output, ts, nspaces, col, init)
                    }
                    nspaces = 0;
                    col = 0;
                    init = true;
                    safe_write!(&mut output, "{}", '\n');
                }
                Ok(c) => {
                    if init || options.aflag {
                        unexpandspan(&mut output, ts, nspaces, col, init)
                    }
                    nspaces = 0;
                    col += 1;
                    init = false;
                    safe_write!(&mut output, "{}", c);
                }
                Err(_) => break
            }
        }
        if init || options.aflag {
            unexpandspan(&mut output, ts, nspaces, col, init)
        }
    }
}

