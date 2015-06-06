#![crate_name = "uniq"]

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

use getopts::{Matches, Options};
use std::cmp::min;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, stdin, stdout, Write};
use std::path::Path;
use std::str::FromStr;

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
    pub fn print_uniq<R: Read, W: Write>(&self, reader: &mut BufReader<R>, writer: &mut BufWriter<W>) {
        let mut lines: Vec<String> = vec!();
        let mut first_line_printed = false;
        let delimiters = &self.delimiters[..];

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

            line[slice_start..slice_stop].chars()
                .map(|c| match c {
                    'a' ... 'z' if self.ignore_case => ((c as u8) - 32) as char,
                    _ => c,
                }).collect()
        } else {
            line.clone()
        }
    }

    fn print_lines<W: Write>(&self, writer: &mut BufWriter<W>, lines: &Vec<String>, print_delimiter: bool) -> bool {
        let mut first_line_printed = false;
        let mut count = if self.all_repeated { 1 } else { lines.len() };
        if lines.len() == 1 && !self.repeats_only
                || lines.len() > 1 && !self.uniques_only {
            self.print_line(writer, &lines[0], count, print_delimiter);
            first_line_printed = true;
            count += 1;
        }
        if self.all_repeated {
            for line in lines[1..].iter() {
                self.print_line(writer, line, count, print_delimiter && !first_line_printed);
                first_line_printed = true;
                count += 1;
            }
        }
        first_line_printed
    }

    fn print_line<W: Write>(&self, writer: &mut BufWriter<W>, line: &String, count: usize, print_delimiter: bool) {
        if print_delimiter {
            crash_if_err!(1, writer.write_all(&['\n' as u8]));
        }

        crash_if_err!(1, if self.show_counts {
            writer.write_all(format!("{:7} {}", count, line).as_bytes())
        } else {
            writer.write_all(line.as_bytes())
        });
        crash_if_err!(1, writer.write_all("\n".as_bytes()));
    }
}

fn opt_parsed<T: FromStr>(opt_name: &str, matches: &Matches) -> Option<T> {
    matches.opt_str(opt_name).map(|arg_str| {
        let opt_val: Option<T> = arg_str.parse().ok();
        opt_val.unwrap_or_else(||
            crash!(1, "Invalid argument for {}: {}", opt_name, arg_str))
    })
}

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();

    opts.optflag("c", "count", "prefix lines by the number of occurrences");
    opts.optflag("d", "repeated", "only print duplicate lines");
    opts.optflagopt(
        "D",
        "all-repeated",
        "print all duplicate lines delimit-method={none(default),prepend,separate} Delimiting is done with blank lines",
        "delimit-method"
    );
    opts.optopt("s", "skip-chars", "avoid comparing the first N characters", "N");
    opts.optopt("w", "check-chars", "compare no more than N characters in lines", "N");
    opts.optflag("i", "ignore-case", "ignore differences in case when comparing");
    opts.optflag("u", "unique", "only print unique lines");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f)
    };

    if matches.opt_present("help") {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {0} [OPTION]... [FILE]...", NAME);
        println!("");
        print!("{}", opts.usage("Filter adjacent matching lines from INPUT (or standard input),\n\
                                    writing to OUTPUT (or standard output)."));
        println!("");
        println!("Note: '{0}' does not detect repeated lines unless they are adjacent.\n\
                  You may want to sort the input first, or use 'sort -u' without '{0}'.\n", NAME);
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
                Some(ref opt_arg) if opt_arg != "none" => {
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

fn open_input_file(in_file_name: String) -> BufReader<Box<Read+'static>> {
    let in_file = if in_file_name == "-" {
        Box::new(stdin()) as Box<Read>
    } else {
        let path = Path::new(&in_file_name[..]);
        let in_file = File::open(&path);
        let r = crash_if_err!(1, in_file);
        Box::new(r) as Box<Read>
    };
    BufReader::new(in_file)
}

fn open_output_file(out_file_name: String) -> BufWriter<Box<Write+'static>> {
    let out_file = if out_file_name == "-" {
        Box::new(stdout()) as Box<Write>
    } else {
        let path = Path::new(&out_file_name[..]);
        let in_file = File::create(&path);
        let w = crash_if_err!(1, in_file);
        Box::new(w) as Box<Write>
    };
    BufWriter::new(out_file)
}
