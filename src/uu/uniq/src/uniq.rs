//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Chirag B Jadwani <chirag.jadwani@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

#[macro_use]
extern crate uucore;

use clap::ArgMatches;
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Read, Result, Write};
use std::path::Path;
use std::str::FromStr;

use crate::app::{get_app, options, Delimiters, ARG_FILES};

pub mod app;

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
        let mut first_line_printed = false;
        let mut group_count = 1;
        let line_terminator = self.get_line_terminator();
        let mut lines = reader.split(line_terminator).map(get_line_string);
        let mut line = match lines.next() {
            Some(l) => l,
            None => return,
        };

        // compare current `line` with consecutive lines (`next_line`) of the input
        // and if needed, print `line` based on the command line options provided
        for next_line in lines {
            if self.cmp_keys(&line, &next_line) {
                if (group_count == 1 && !self.repeats_only)
                    || (group_count > 1 && !self.uniques_only)
                {
                    self.print_line(writer, &line, group_count, first_line_printed);
                    first_line_printed = true;
                }
                line = next_line;
                group_count = 1;
            } else {
                if self.all_repeated {
                    self.print_line(writer, &line, group_count, first_line_printed);
                    first_line_printed = true;
                    line = next_line;
                }
                group_count += 1;
            }
        }
        if (group_count == 1 && !self.repeats_only) || (group_count > 1 && !self.uniques_only) {
            self.print_line(writer, &line, group_count, first_line_printed);
            first_line_printed = true;
        }
        if (self.delimiters == Delimiters::Append || self.delimiters == Delimiters::Both)
            && first_line_printed
        {
            crash_if_err!(1, writer.write_all(&[line_terminator]));
        }
    }

    fn skip_fields<'a>(&self, line: &'a str) -> &'a str {
        if let Some(skip_fields) = self.skip_fields {
            let mut i = 0;
            let mut char_indices = line.char_indices();
            for _ in 0..skip_fields {
                if char_indices.find(|(_, c)| !c.is_whitespace()) == None {
                    return "";
                }
                match char_indices.find(|(_, c)| c.is_whitespace()) {
                    None => return "",

                    Some((next_field_i, _)) => i = next_field_i,
                }
            }
            &line[i..]
        } else {
            line
        }
    }

    fn get_line_terminator(&self) -> u8 {
        if self.zero_terminated {
            0
        } else {
            b'\n'
        }
    }

    fn cmp_keys(&self, first: &str, second: &str) -> bool {
        self.cmp_key(first, |first_iter| {
            self.cmp_key(second, |second_iter| first_iter.ne(second_iter))
        })
    }

    fn cmp_key<F>(&self, line: &str, mut closure: F) -> bool
    where
        F: FnMut(&mut dyn Iterator<Item = char>) -> bool,
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
                return closure(&mut fields_to_check.chars().flat_map(|c| c.to_uppercase()));
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
                    .flat_map(|c| c.to_uppercase()),
            )
        } else {
            closure(&mut fields_to_check.chars())
        }
    }

    fn should_print_delimiter(&self, group_count: usize, first_line_printed: bool) -> bool {
        // if no delimiter option is selected then no other checks needed
        self.delimiters != Delimiters::None
            // print delimiter only before the first line of a group, not between lines of a group
            && group_count == 1
            // if at least one line has been output before current group then print delimiter
            && (first_line_printed
                // or if we need to prepend delimiter then print it even at the start of the output
                || self.delimiters == Delimiters::Prepend
                // the 'both' delimit mode should prepend and append delimiters
                || self.delimiters == Delimiters::Both)
    }

    fn print_line<W: Write>(
        &self,
        writer: &mut BufWriter<W>,
        line: &str,
        count: usize,
        first_line_printed: bool,
    ) {
        let line_terminator = self.get_line_terminator();

        if self.should_print_delimiter(count, first_line_printed) {
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

fn get_line_string(io_line: Result<Vec<u8>>) -> String {
    let line_bytes = crash_if_err!(1, io_line);
    crash_if_err!(1, String::from_utf8(line_bytes))
}

fn opt_parsed<T: FromStr>(opt_name: &str, matches: &ArgMatches) -> Option<T> {
    matches.value_of(opt_name).map(|arg_str| {
        let opt_val: Option<T> = arg_str.parse().ok();
        opt_val.unwrap_or_else(|| crash!(1, "Invalid argument for {}: {}", opt_name, arg_str))
    })
}

fn get_usage() -> String {
    format!("{0} [OPTION]... [INPUT [OUTPUT]]...", executable!())
}

fn get_long_usage() -> String {
    String::from(
        "Filter adjacent matching lines from INPUT (or standard input),\n\
        writing to OUTPUT (or standard output).
        Note: 'uniq' does not detect repeated lines unless they are adjacent.\n\
        You may want to sort the input first, or use 'sort -u' without 'uniq'.\n",
    )
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();
    let long_usage = get_long_usage();

    let matches = get_app(executable!())
        .usage(&usage[..])
        .after_help(&long_usage[..])
        .get_matches_from(args);

    let files: Vec<String> = matches
        .values_of(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let (in_file_name, out_file_name) = match files.len() {
        0 => ("-".to_owned(), "-".to_owned()),
        1 => (files[0].clone(), "-".to_owned()),
        2 => (files[0].clone(), files[1].clone()),
        _ => {
            // Cannot happen as clap will fail earlier
            crash!(1, "Extra operand: {}", files[2]);
        }
    };

    let uniq = Uniq {
        repeats_only: matches.is_present(options::REPEATED)
            || matches.is_present(options::ALL_REPEATED),
        uniques_only: matches.is_present(options::UNIQUE),
        all_repeated: matches.is_present(options::ALL_REPEATED)
            || matches.is_present(options::GROUP),
        delimiters: get_delimiter(&matches),
        show_counts: matches.is_present(options::COUNT),
        skip_fields: opt_parsed(options::SKIP_FIELDS, &matches),
        slice_start: opt_parsed(options::SKIP_CHARS, &matches),
        slice_stop: opt_parsed(options::CHECK_CHARS, &matches),
        ignore_case: matches.is_present(options::IGNORE_CASE),
        zero_terminated: matches.is_present(options::ZERO_TERMINATED),
    };
    uniq.print_uniq(
        &mut open_input_file(in_file_name),
        &mut open_output_file(out_file_name),
    );

    0
}

fn get_delimiter(matches: &ArgMatches) -> Delimiters {
    let value = matches
        .value_of(options::ALL_REPEATED)
        .or_else(|| matches.value_of(options::GROUP));
    if let Some(delimiter_arg) = value {
        crash_if_err!(1, Delimiters::from_str(delimiter_arg))
    } else if matches.is_present(options::GROUP) {
        Delimiters::Separate
    } else {
        Delimiters::None
    }
}

fn open_input_file(in_file_name: String) -> BufReader<Box<dyn Read + 'static>> {
    let in_file = if in_file_name == "-" {
        Box::new(stdin()) as Box<dyn Read>
    } else {
        let path = Path::new(&in_file_name[..]);
        let in_file = File::open(&path);
        let r = crash_if_err!(1, in_file);
        Box::new(r) as Box<dyn Read>
    };
    BufReader::new(in_file)
}

fn open_output_file(out_file_name: String) -> BufWriter<Box<dyn Write + 'static>> {
    let out_file = if out_file_name == "-" {
        Box::new(stdout()) as Box<dyn Write>
    } else {
        let path = Path::new(&out_file_name[..]);
        let in_file = File::create(&path);
        let w = crash_if_err!(1, in_file);
        Box::new(w) as Box<dyn Write>
    };
    BufWriter::new(out_file)
}
