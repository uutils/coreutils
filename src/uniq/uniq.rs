#![crate_name = "uniq"]
#![feature(collections, core, io, path, rustc_private, std_misc)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Chirag B Jadwani <chirag.jadwani@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 *
 */

extern crate getopts;

use std::ascii::OwnedAsciiExt;
use std::cmp::min;
use std::str::FromStr;
use std::old_io as io;

#[path = "../common/util.rs"]
#[macro_use]
mod util;

static NAME: &'static str = "uniq";
static VERSION: &'static str = "1.0.0";

struct Uniq {
    repeats_only: bool,
    uniques_only: bool,
    all_repeated: bool,
    delimiters: String,
    show_counts: bool,
    slice_start: Option<usize>,
    slice_stop: Option<usize>,
    ignore_case: bool,
}

impl Uniq {
    pub fn print_uniq<R: Reader, W: Writer>(&self, reader: &mut io::BufferedReader<R>, writer: &mut io::BufferedWriter<W>) {
        let mut lines: Vec<String> = vec!();
        let mut first_line_printed = false;
        let delimiters = self.delimiters.as_slice();

        for io_line in reader.lines() {
            let line = crash_if_err!(1, io_line);
            if !lines.is_empty() && self.cmp_key(&lines[0]) != self.cmp_key(&line) {
                let print_delimiter = delimiters == "prepend" || (delimiters == "separate" && first_line_printed);
                first_line_printed |= self.print_lines(writer, &lines, print_delimiter);
                lines.truncate(0);
            }
            lines.push(line);
        }
        if !lines.is_empty() {
            let print_delimiter = delimiters == "prepend" || (delimiters == "separate" && first_line_printed);
            self.print_lines(writer, &lines, print_delimiter);
        }
    }

    fn cmp_key(&self, line: &String) -> String {
        let len = line.len();
        if len > 0 {
            let slice_start = match self.slice_start {
                Some(i) => min(i, len - 1),
                None => 0
            };
            let slice_stop = match self.slice_stop {
                Some(i) => min(slice_start + i, len),
                None => len
            };
            let sliced = line.as_slice()[slice_start..slice_stop].to_string();
            if self.ignore_case {
                sliced.into_ascii_uppercase()
            } else {
                sliced
            }
        } else {
            line.clone()
        }
    }

    fn print_lines<W: Writer>(&self, writer: &mut io::BufferedWriter<W>, lines: &Vec<String>, print_delimiter: bool) -> bool {
        let mut first_line_printed = false;
        let mut count = if self.all_repeated { 1 } else { lines.len() };
        if lines.len() == 1 && !self.repeats_only
                || lines.len() > 1 && !self.uniques_only {
            self.print_line(writer, &lines[0], count, print_delimiter);
            first_line_printed = true;
            count += 1;
        }
        if self.all_repeated {
            for line in lines.tail().iter() {
                self.print_line(writer, line, count, print_delimiter && !first_line_printed);
                first_line_printed = true;
                count += 1;
            }
        }
        first_line_printed
    }

    fn print_line<W: Writer>(&self, writer: &mut io::BufferedWriter<W>, line: &String, count: usize, print_delimiter: bool) {
        let output_line = if self.show_counts {
            format!("{:7} {}", count, line)
        } else {
            line.clone()
        };
        if print_delimiter {
            crash_if_err!(1, writer.write_line(""));
        }
        crash_if_err!(1, writer.write_str(output_line.as_slice()));
    }
}

fn opt_parsed<T: FromStr>(opt_name: &str, matches: &getopts::Matches) -> Option<T> {
    matches.opt_str(opt_name).map(|arg_str| {
        let opt_val: Option<T> = arg_str.parse().ok();
        opt_val.unwrap_or_else(||
            crash!(1, "Invalid argument for {}: {}", opt_name, arg_str))
    })
}

pub fn uumain(args: Vec<String>) -> isize {
    let program_path = Path::new(args[0].clone());
    let program = program_path.filename_str().unwrap_or(NAME);

    let opts = [
        getopts::optflag("c", "count", "prefix lines by the number of occurrences"),
        getopts::optflag("d", "repeated", "only print duplicate lines"),
        getopts::optflagopt(
            "D",
            "all-repeated",
            "print all duplicate lines delimit-method={none(default),prepend,separate} Delimiting is done with blank lines",
            "delimit-method"
        ),
        getopts::optopt("s", "skip-chars", "avoid comparing the first N characters", "N"),
        getopts::optopt("w", "check-chars", "compare no more than N characters in lines", "N"),
        getopts::optflag("i", "ignore-case", "ignore differences in case when comparing"),
        getopts::optflag("u", "unique", "only print unique lines"),
        getopts::optflag("h", "help", "display this help and exit"),
        getopts::optflag("V", "version", "output version information and exit")
    ];
    let matches = match getopts::getopts(args.tail(), &opts) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f)
    };

    if matches.opt_present("help") {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {0} [OPTION]... [FILE]...", program);
        println!("");
        print!("{}", getopts::usage("Filter adjacent matching lines from INPUT (or standard input),\n\
                                    writing to OUTPUT (or standard output).", &opts));
        println!("");
        println!("Note: '{0}' does not detect repeated lines unless they are adjacent.\n\
                  You may want to sort the input first, or use 'sort -u' without '{0}'.\n", program);
    } else if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
    } else {
        let (in_file_name, out_file_name) = match matches.free.len() {
            0 => ("-".to_string(), "-".to_string()),
            1 => (matches.free[0].clone(), "-".to_string()),
            2 => (matches.free[0].clone(), matches.free[1].clone()),
            _ => {
                crash!(1, "Extra operand: {}", matches.free[2]);
            }
        };
        let uniq = Uniq {
            repeats_only: matches.opt_present("repeated") || matches.opt_present("all-repeated"),
            uniques_only: matches.opt_present("unique"),
            all_repeated: matches.opt_present("all-repeated"),
            delimiters: match matches.opt_default("all-repeated", "none") {
                Some(ref opt_arg) if opt_arg.as_slice() != "none" => {
                    let rep_args = ["prepend".to_string(), "separate".to_string()];
                    if !rep_args.contains(opt_arg) {
                        crash!(1, "Incorrect argument for all-repeated: {}", opt_arg.clone());
                    }
                    opt_arg.clone()
                },
                _ => "".to_string()
            },
            show_counts: matches.opt_present("count"),
            slice_start: opt_parsed("skip-chars", &matches),
            slice_stop: opt_parsed("check-chars", &matches),
            ignore_case: matches.opt_present("ignore-case"),
        };
        uniq.print_uniq(&mut open_input_file(in_file_name),
                        &mut open_output_file(out_file_name));
    }
    0
}

fn open_input_file(in_file_name: String) -> io::BufferedReader<Box<Reader+'static>> {
    let in_file = if in_file_name.as_slice() == "-" {
        Box::new(io::stdio::stdin_raw()) as Box<Reader>
    } else {
        let path = Path::new(in_file_name);
        let in_file = io::File::open(&path);
        let r = crash_if_err!(1, in_file);
        Box::new(r) as Box<Reader>
    };
    io::BufferedReader::new(in_file)
}

fn open_output_file(out_file_name: String) -> io::BufferedWriter<Box<Writer+'static>> {
    let out_file = if out_file_name.as_slice() == "-" {
        Box::new(io::stdio::stdout_raw()) as Box<Writer>
    } else {
        let path = Path::new(out_file_name);
        let in_file = io::File::create(&path);
        let w = crash_if_err!(1, in_file);
        Box::new(w) as Box<Writer>
    };
    io::BufferedWriter::new(out_file)
}
