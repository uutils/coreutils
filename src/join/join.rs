#![crate_name = "uu_join"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Konstantin Pospelov <kupospelov@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate clap;

#[macro_use]
extern crate uucore;

use std::fs::File;
use std::io::{stdin, BufRead, BufReader, Lines, Stdin};
use std::cmp::{min, Ordering};
use clap::{App, Arg};

static NAME: &'static str = "join";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(Copy, Clone, PartialEq)]
enum FileNum {
    None,
    File1,
    File2,
}

#[derive(Copy, Clone)]
enum Sep {
    Char(char),
    Line,
    Whitespaces,
}

#[derive(Copy, Clone, PartialEq)]
enum CheckOrder {
    Default,
    Disabled,
    Enabled,
}

struct Settings {
    key1: usize,
    key2: usize,
    print_unpaired: FileNum,
    print_joined: bool,
    ignore_case: bool,
    separator: Sep,
    autoformat: bool,
    format: Vec<Spec>,
    empty: String,
    check_order: CheckOrder,
    headers: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            key1: 0,
            key2: 0,
            print_unpaired: FileNum::None,
            print_joined: true,
            ignore_case: false,
            separator: Sep::Whitespaces,
            autoformat: false,
            format: vec![],
            empty: String::new(),
            check_order: CheckOrder::Default,
            headers: false,
        }
    }
}

/// Output representation.
struct Repr<'a> {
    separator: char,
    format: &'a [Spec],
    empty: &'a str,
}

impl<'a> Repr<'a> {
    fn new(separator: char, format: &'a [Spec], empty: &'a str) -> Repr<'a> {
        Repr {
            separator,
            format,
            empty,
        }
    }

    fn uses_format(&self) -> bool {
        !self.format.is_empty()
    }

    /// Print the field or empty filler if the field is not set.
    fn print_field(&self, field: Option<&str>) {
        let value = match field {
            Some(field) => field,
            None => self.empty,
        };

        print!("{}", value);
    }

    /// Print each field except the one at the index.
    fn print_fields(&self, line: &Line, index: usize, max_fields: usize) {
        for i in 0..min(max_fields, line.fields.len()) {
            if i != index {
                print!("{}{}", self.separator, line.fields[i]);
            }
        }
    }

    /// Print each field or the empty filler if the field is not set.
    fn print_format<F>(&self, f: F)
    where
        F: Fn(&Spec) -> Option<&'a str>,
    {
        for i in 0..self.format.len() {
            if i > 0 {
                print!("{}", self.separator);
            }

            let field = match f(&self.format[i]) {
                Some(value) => value,
                None => self.empty,
            };

            print!("{}", field);
        }
    }
}

/// Input processing parameters.
struct Input {
    separator: Sep,
    ignore_case: bool,
    check_order: CheckOrder,
}

impl Input {
    fn new(separator: Sep, ignore_case: bool, check_order: CheckOrder) -> Input {
        Input {
            separator,
            ignore_case,
            check_order,
        }
    }

    fn compare(&self, field1: Option<&str>, field2: Option<&str>) -> Ordering {
        if let (Some(field1), Some(field2)) = (field1, field2) {
            if self.ignore_case {
                field1.to_lowercase().cmp(&field2.to_lowercase())
            } else {
                field1.cmp(field2)
            }
        } else {
            match field1 {
                Some(_) => Ordering::Greater,
                None => match field2 {
                    Some(_) => Ordering::Less,
                    None => Ordering::Equal,
                },
            }
        }
    }
}

enum Spec {
    Key,
    Field(FileNum, usize),
}

impl Spec {
    fn parse(format: &str) -> Spec {
        let mut chars = format.chars();

        let file_num = match chars.next() {
            Some('0') => {
                // Must be all alone without a field specifier.
                if let None = chars.next() {
                    return Spec::Key;
                }

                crash!(1, "invalid field specifier: '{}'", format);
            }
            Some('1') => FileNum::File1,
            Some('2') => FileNum::File2,
            _ => crash!(1, "invalid file number in field spec: '{}'", format),
        };

        if let Some('.') = chars.next() {
            return Spec::Field(file_num, parse_field_number(chars.as_str()));
        }

        crash!(1, "invalid field specifier: '{}'", format);
    }
}

struct Line {
    fields: Vec<String>,
}

impl Line {
    fn new(string: String, separator: Sep) -> Line {
        let fields = match separator {
            Sep::Whitespaces => string.split_whitespace().map(String::from).collect(),
            Sep::Char(sep) => string.split(sep).map(String::from).collect(),
            Sep::Line => vec![string],
        };

        Line { fields }
    }

    /// Get field at index.
    fn get_field(&self, index: usize) -> Option<&str> {
        if index < self.fields.len() {
            Some(&self.fields[index])
        } else {
            None
        }
    }
}

struct State<'a> {
    key: usize,
    file_name: &'a str,
    file_num: FileNum,
    print_unpaired: bool,
    lines: Lines<Box<BufRead + 'a>>,
    seq: Vec<Line>,
    max_fields: usize,
    line_num: usize,
    has_failed: bool,
}

impl<'a> State<'a> {
    fn new(
        file_num: FileNum,
        name: &'a str,
        stdin: &'a Stdin,
        key: usize,
        print_unpaired: FileNum,
    ) -> State<'a> {
        let f = if name == "-" {
            Box::new(stdin.lock()) as Box<BufRead>
        } else {
            match File::open(name) {
                Ok(file) => Box::new(BufReader::new(file)) as Box<BufRead>,
                Err(err) => crash!(1, "{}: {}", name, err),
            }
        };

        State {
            key: key,
            file_name: name,
            file_num: file_num,
            print_unpaired: print_unpaired == file_num,
            lines: f.lines(),
            seq: Vec::new(),
            max_fields: usize::max_value(),
            line_num: 0,
            has_failed: false,
        }
    }

    /// Skip the current unpaired line.
    fn skip_line(&mut self, input: &Input, repr: &Repr) {
        if self.print_unpaired {
            self.print_first_line(repr);
        }

        self.reset_next_line(input);
    }

    /// Keep reading line sequence until the key does not change, return
    /// the first line whose key differs.
    fn extend(&mut self, input: &Input) -> Option<Line> {
        while let Some(line) = self.next_line(input) {
            let diff = input.compare(self.get_current_key(), line.get_field(self.key));

            if diff == Ordering::Equal {
                self.seq.push(line);
            } else {
                return Some(line);
            }
        }

        return None;
    }

    /// Print lines in the buffers as headers.
    fn print_headers(&self, other: &State, repr: &Repr) {
        if self.has_line() {
            if other.has_line() {
                self.combine(other, repr);
            } else {
                self.print_first_line(repr);
            }
        } else if other.has_line() {
            other.print_first_line(repr);
        }
    }

    /// Combine two line sequences.
    fn combine(&self, other: &State, repr: &Repr) {
        let key = self.get_current_key();

        for line1 in &self.seq {
            for line2 in &other.seq {
                if repr.uses_format() {
                    repr.print_format(|spec| match spec {
                        &Spec::Key => key,
                        &Spec::Field(file_num, field_num) => {
                            if file_num == self.file_num {
                                return line1.get_field(field_num);
                            }

                            if file_num == other.file_num {
                                return line2.get_field(field_num);
                            }

                            None
                        }
                    });
                } else {
                    repr.print_field(key);
                    repr.print_fields(&line1, self.key, self.max_fields);
                    repr.print_fields(&line2, other.key, other.max_fields);
                }

                println!();
            }
        }
    }

    /// Reset with the next line.
    fn reset(&mut self, next_line: Option<Line>) {
        self.seq.clear();

        if let Some(line) = next_line {
            self.seq.push(line);
        }
    }

    fn reset_read_line(&mut self, input: &Input) {
        let line = self.read_line(input.separator);
        self.reset(line);
    }

    fn reset_next_line(&mut self, input: &Input) {
        let line = self.next_line(input);
        self.reset(line);
    }

    fn has_line(&self) -> bool {
        !self.seq.is_empty()
    }

    fn initialize(&mut self, read_sep: Sep, autoformat: bool) {
        if let Some(line) = self.read_line(read_sep) {
            if autoformat {
                self.max_fields = line.fields.len();
            }

            self.seq.push(line);
        }
    }

    fn finalize(&mut self, input: &Input, repr: &Repr) {
        if self.has_line() && self.print_unpaired {
            self.print_first_line(repr);

            while let Some(line) = self.next_line(input) {
                self.print_line(&line, repr);
            }
        }
    }

    /// Get the next line without the order check.
    fn read_line(&mut self, sep: Sep) -> Option<Line> {
        let value = self.lines.next()?;
        self.line_num += 1;
        Some(Line::new(crash_if_err!(1, value), sep))
    }

    /// Get the next line with the order check.
    fn next_line(&mut self, input: &Input) -> Option<Line> {
        let line = self.read_line(input.separator)?;

        if input.check_order == CheckOrder::Disabled {
            return Some(line);
        }

        let diff = input.compare(self.get_current_key(), line.get_field(self.key));

        if diff == Ordering::Greater {
            eprintln!("{}:{}: is not sorted", self.file_name, self.line_num);

            // This is fatal if the check is enabled.
            if input.check_order == CheckOrder::Enabled {
                exit!(1);
            }

            self.has_failed = true;
        }

        Some(line)
    }

    /// Gets the key value of the lines stored in seq.
    fn get_current_key(&self) -> Option<&str> {
        self.seq[0].get_field(self.key)
    }

    fn print_line(&self, line: &Line, repr: &Repr) {
        if repr.uses_format() {
            repr.print_format(|spec| match spec {
                &Spec::Key => line.get_field(self.key),
                &Spec::Field(file_num, field_num) => if file_num == self.file_num {
                    line.get_field(field_num)
                } else {
                    None
                },
            });
        } else {
            repr.print_field(line.get_field(self.key));
            repr.print_fields(line, self.key, self.max_fields);
        }

        println!();
    }

    fn print_first_line(&self, repr: &Repr) {
        self.print_line(&self.seq[0], repr);
    }
}

pub fn uumain(args: Vec<String>) -> i32 {
    let matches = App::new(NAME)
        .version(VERSION)
        .about(
            "For each pair of input lines with identical join fields, write a line to
standard output. The default join field is the first, delimited by blanks.

When FILE1 or FILE2 (not both) is -, read standard input.",
        )
        .help_message("display this help and exit")
        .version_message("display version and exit")
        .arg(
            Arg::with_name("a")
                .short("a")
                .takes_value(true)
                .possible_values(&["1", "2"])
                .value_name("FILENUM")
                .help(
                    "also print unpairable lines from file FILENUM, where
FILENUM is 1 or 2, corresponding to FILE1 or FILE2",
                ),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .value_name("FILENUM")
                .help("like -a FILENUM, but suppress joined output lines"),
        )
        .arg(
            Arg::with_name("e")
                .short("e")
                .takes_value(true)
                .value_name("EMPTY")
                .help("replace missing input fields with EMPTY"),
        )
        .arg(
            Arg::with_name("i")
                .short("i")
                .long("ignore-case")
                .help("ignore differences in case when comparing fields"),
        )
        .arg(
            Arg::with_name("j")
                .short("j")
                .takes_value(true)
                .value_name("FIELD")
                .help("equivalent to '-1 FIELD -2 FIELD'"),
        )
        .arg(
            Arg::with_name("o")
                .short("o")
                .takes_value(true)
                .value_name("FORMAT")
                .help("obey FORMAT while constructing output line"),
        )
        .arg(
            Arg::with_name("t")
                .short("t")
                .takes_value(true)
                .value_name("CHAR")
                .help("use CHAR as input and output field separator"),
        )
        .arg(
            Arg::with_name("1")
                .short("1")
                .takes_value(true)
                .value_name("FIELD")
                .help("join on this FIELD of file 1"),
        )
        .arg(
            Arg::with_name("2")
                .short("2")
                .takes_value(true)
                .value_name("FIELD")
                .help("join on this FIELD of file 2"),
        )
        .arg(Arg::with_name("check-order").long("check-order").help(
            "check that the input is correctly sorted, \
             even if all input lines are pairable",
        ))
        .arg(
            Arg::with_name("nocheck-order")
                .long("nocheck-order")
                .help("do not check that the input is correctly sorted"),
        )
        .arg(Arg::with_name("header").long("header").help(
            "treat the first line in each file as field headers, \
             print them without trying to pair them",
        ))
        .arg(
            Arg::with_name("file1")
                .required(true)
                .value_name("FILE1")
                .hidden(true),
        )
        .arg(
            Arg::with_name("file2")
                .required(true)
                .value_name("FILE2")
                .hidden(true),
        )
        .get_matches_from(args);

    let keys = parse_field_number_option(matches.value_of("j"));
    let key1 = parse_field_number_option(matches.value_of("1"));
    let key2 = parse_field_number_option(matches.value_of("2"));

    let mut settings: Settings = Default::default();

    if let Some(value) = matches.value_of("v") {
        settings.print_unpaired = parse_file_number(value);
        settings.print_joined = false;
    } else if let Some(value) = matches.value_of("a") {
        settings.print_unpaired = parse_file_number(value);
    }

    settings.ignore_case = matches.is_present("i");
    settings.key1 = get_field_number(keys, key1);
    settings.key2 = get_field_number(keys, key2);

    if let Some(value) = matches.value_of("t") {
        settings.separator = match value.len() {
            0 => Sep::Line,
            1 => Sep::Char(value.chars().nth(0).unwrap()),
            _ => crash!(1, "multi-character tab {}", value),
        };
    }

    if let Some(format) = matches.value_of("o") {
        if format == "auto" {
            settings.autoformat = true;
        } else {
            settings.format = format
                .split(|c| c == ' ' || c == ',' || c == '\t')
                .map(Spec::parse)
                .collect();
        }
    }

    if let Some(empty) = matches.value_of("e") {
        settings.empty = empty.to_string();
    }

    if matches.is_present("nocheck-order") {
        settings.check_order = CheckOrder::Disabled;
    }

    if matches.is_present("check-order") {
        settings.check_order = CheckOrder::Enabled;
    }

    if matches.is_present("header") {
        settings.headers = true;
    }

    let file1 = matches.value_of("file1").unwrap();
    let file2 = matches.value_of("file2").unwrap();

    if file1 == "-" && file2 == "-" {
        crash!(1, "both files cannot be standard input");
    }

    exec(file1, file2, &settings)
}

fn exec(file1: &str, file2: &str, settings: &Settings) -> i32 {
    let stdin = stdin();

    let mut state1 = State::new(
        FileNum::File1,
        &file1,
        &stdin,
        settings.key1,
        settings.print_unpaired,
    );

    let mut state2 = State::new(
        FileNum::File2,
        &file2,
        &stdin,
        settings.key2,
        settings.print_unpaired,
    );

    let input = Input::new(
        settings.separator,
        settings.ignore_case,
        settings.check_order,
    );

    let repr = Repr::new(
        match settings.separator {
            Sep::Char(sep) => sep,
            _ => ' ',
        },
        &settings.format,
        &settings.empty,
    );

    state1.initialize(settings.separator, settings.autoformat);
    state2.initialize(settings.separator, settings.autoformat);

    if settings.headers {
        state1.print_headers(&state2, &repr);
        state1.reset_read_line(&input);
        state2.reset_read_line(&input);
    }

    while state1.has_line() && state2.has_line() {
        let diff = input.compare(state1.get_current_key(), state2.get_current_key());

        match diff {
            Ordering::Less => {
                state1.skip_line(&input, &repr);
            }
            Ordering::Greater => {
                state2.skip_line(&input, &repr);
            }
            Ordering::Equal => {
                let next_line1 = state1.extend(&input);
                let next_line2 = state2.extend(&input);

                if settings.print_joined {
                    state1.combine(&state2, &repr);
                }

                state1.reset(next_line1);
                state2.reset(next_line2);
            }
        }
    }

    state1.finalize(&input, &repr);
    state2.finalize(&input, &repr);

    (state1.has_failed || state2.has_failed) as i32
}

/// Check that keys for both files and for a particular file are not
/// contradictory and return the key index.
fn get_field_number(keys: Option<usize>, key: Option<usize>) -> usize {
    if let Some(keys) = keys {
        if let Some(key) = key {
            if keys != key {
                // Show zero-based field numbers as one-based.
                crash!(1, "incompatible join fields {}, {}", keys + 1, key + 1);
            }
        }

        return keys;
    }

    match key {
        Some(key) => key,
        None => 0,
    }
}

/// Parse the specified field string as a natural number and return
/// the zero-based field number.
fn parse_field_number(value: &str) -> usize {
    match value.parse::<usize>() {
        Ok(result) if result > 0 => result - 1,
        _ => crash!(1, "invalid field number: '{}'", value),
    }
}

fn parse_file_number(value: &str) -> FileNum {
    match value {
        "1" => FileNum::File1,
        "2" => FileNum::File2,
        value => crash!(1, "invalid file number: '{}'", value),
    }
}

fn parse_field_number_option(value: Option<&str>) -> Option<usize> {
    Some(parse_field_number(value?))
}
