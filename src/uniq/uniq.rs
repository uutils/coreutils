#![crate_name = "uu_uniq"]

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

#[macro_use]
extern crate uucore;

use getopts::{Matches, Options};
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::str::FromStr;

static NAME: &str = "uniq";
static VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(PartialEq)]
enum Delimiters {
    Prepend,
    Separate,
    None,
}

struct Uniq {
    repeats_only: bool,
    uniques_only: bool,
    all_repeated: bool,
    delimiters: Delimiters,
    show_counts: bool,
    skip_fields: Option<usize>,
    slice_start: Option<usize>,
    slice_stop: Option<usize>,
    ignore_case: bool,
    zero_terminated: bool,
}

impl Uniq {
    pub fn print_uniq<R: Read, W: Write>(
        &self,
        reader: &mut BufReader<R>,
        writer: &mut BufWriter<W>,
    ) {
        let mut lines: Vec<String> = vec![];
        let mut first_line_printed = false;
        let delimiters = &self.delimiters;
        let line_terminator = self.get_line_terminator();

        for io_line in reader.split(line_terminator) {
            let line = String::from_utf8(crash_if_err!(1, io_line)).unwrap();
            if !lines.is_empty() && self.cmp_keys(&lines[0], &line) {
                let print_delimiter = delimiters == &Delimiters::Prepend
                    || (delimiters == &Delimiters::Separate && first_line_printed);
                first_line_printed |= self.print_lines(writer, &lines, print_delimiter);
                lines.truncate(0);
            }
            lines.push(line);
        }
        if !lines.is_empty() {
            let print_delimiter = delimiters == &Delimiters::Prepend
                || (delimiters == &Delimiters::Separate && first_line_printed);
            self.print_lines(writer, &lines, print_delimiter);
        }
    }

    fn skip_fields<'a>(&self, line: &'a str) -> &'a str {
        if let Some(skip_fields) = self.skip_fields {
            if line.split_whitespace().count() > skip_fields {
                let mut field = 0;
                let mut i = 0;
                while field < skip_fields && i < line.len() {
                    while i < line.len() && line.chars().nth(i).unwrap().is_whitespace() {
                        i = i + 1;
                    }
                    while i < line.len() && !line.chars().nth(i).unwrap().is_whitespace() {
                        i = i + 1;
                    }
                    field = field + 1;
                }
                &line[i..]
            } else {
                ""
            }
        } else {
            line
        }
    }

    fn get_line_terminator(&self) -> u8 {
        if self.zero_terminated {
            0
        } else {
            '\n' as u8
        }
    }

    fn cmp_keys(&self, first: &str, second: &str) -> bool {
        self.cmp_key(first, |first_iter| {
            self.cmp_key(second, |second_iter| first_iter.ne(second_iter))
        })
    }

    fn cmp_key<F>(&self, line: &str, mut closure: F) -> bool
    where
        F: FnMut(&mut Iterator<Item = char>) -> bool,
    {
        let fields_to_check = self.skip_fields(line);
        let len = fields_to_check.len();
        let slice_start = self.slice_start.unwrap_or(0);
        let slice_stop = self.slice_stop.unwrap_or(len);
        if len > 0 {
            // fast path: avoid doing any work if there is no need to skip or map to lower-case
            if !self.ignore_case && slice_start == 0 && slice_stop == len {
                return closure(&mut fields_to_check.chars());
            }

            // fast path: avoid skipping
            if self.ignore_case && slice_start == 0 && slice_stop == len {
                return closure(&mut fields_to_check.chars().map(|c| match c {
                    'a'...'z' => ((c as u8) - 32) as char,
                    _ => c,
                }));
            }

            // fast path: we can avoid mapping chars to upper-case, if we don't want to ignore the case
            if !self.ignore_case {
                return closure(&mut fields_to_check.chars().skip(slice_start).take(slice_stop));
            }

            closure(
                &mut fields_to_check
                    .chars()
                    .skip(slice_start)
                    .take(slice_stop)
                    .map(|c| match c {
                        'a'...'z' => ((c as u8) - 32) as char,
                        _ => c,
                    }),
            )
        } else {
            closure(&mut fields_to_check.chars())
        }
    }

    fn print_lines<W: Write>(
        &self,
        writer: &mut BufWriter<W>,
        lines: &[String],
        print_delimiter: bool,
    ) -> bool {
        let mut first_line_printed = false;
        let mut count = if self.all_repeated { 1 } else { lines.len() };
        if lines.len() == 1 && !self.repeats_only || lines.len() > 1 && !self.uniques_only {
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

    fn print_line<W: Write>(
        &self,
        writer: &mut BufWriter<W>,
        line: &str,
        count: usize,
        print_delimiter: bool,
    ) {
        let line_terminator = self.get_line_terminator();

        if print_delimiter {
            crash_if_err!(1, writer.write_all(&[line_terminator]));
        }

        crash_if_err!(
            1,
            if self.show_counts {
                writer.write_all(format!("{:7} {}", count, line).as_bytes())
            } else {
                writer.write_all(line.as_bytes())
            }
        );
        crash_if_err!(1, writer.write_all(&[line_terminator]));
    }
}

fn opt_parsed<T: FromStr>(opt_name: &str, matches: &Matches) -> Option<T> {
    matches.opt_str(opt_name).map(|arg_str| {
        let opt_val: Option<T> = arg_str.parse().ok();
        opt_val.unwrap_or_else(|| crash!(1, "Invalid argument for {}: {}", opt_name, arg_str))
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
    opts.optopt(
        "f",
        "skip-fields",
        "avoid comparing the first N fields",
        "N",
    );
    opts.optopt(
        "s",
        "skip-chars",
        "avoid comparing the first N characters",
        "N",
    );
    opts.optopt(
        "w",
        "check-chars",
        "compare no more than N characters in lines",
        "N",
    );
    opts.optflag(
        "i",
        "ignore-case",
        "ignore differences in case when comparing",
    );
    opts.optflag("u", "unique", "only print unique lines");
    opts.optflag("z", "zero-terminated", "end lines with 0 byte, not newline");
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => crash!(1, "{}", f),
    };

    if matches.opt_present("help") {
        println!("{} {}", NAME, VERSION);
        println!("");
        println!("Usage:");
        println!("  {0} [OPTION]... [FILE]...", NAME);
        println!("");
        print!(
            "{}",
            opts.usage(
                "Filter adjacent matching lines from INPUT (or standard input),\n\
                 writing to OUTPUT (or standard output)."
            )
        );
        println!("");
        println!(
            "Note: '{0}' does not detect repeated lines unless they are adjacent.\n\
             You may want to sort the input first, or use 'sort -u' without '{0}'.\n",
            NAME
        );
    } else if matches.opt_present("version") {
        println!("{} {}", NAME, VERSION);
    } else {
        let (in_file_name, out_file_name) = match matches.free.len() {
            0 => ("-".to_owned(), "-".to_owned()),
            1 => (matches.free[0].clone(), "-".to_owned()),
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
                Some(ref opt_arg) if opt_arg != "none" => match &(*opt_arg.as_str()) {
                    "prepend" => Delimiters::Prepend,
                    "separate" => Delimiters::Separate,
                    _ => crash!(1, "Incorrect argument for all-repeated: {}", opt_arg),
                },
                _ => Delimiters::None,
            },
            show_counts: matches.opt_present("count"),
            skip_fields: opt_parsed("skip-fields", &matches),
            slice_start: opt_parsed("skip-chars", &matches),
            slice_stop: opt_parsed("check-chars", &matches),
            ignore_case: matches.opt_present("ignore-case"),
            zero_terminated: matches.opt_present("zero-terminated"),
        };
        uniq.print_uniq(
            &mut open_input_file(in_file_name),
            &mut open_output_file(out_file_name),
        );
    }
    0
}

fn open_input_file(in_file_name: String) -> BufReader<Box<Read + 'static>> {
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

fn open_output_file(out_file_name: String) -> BufWriter<Box<Write + 'static>> {
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
