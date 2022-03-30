//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Konstantin Pospelov <kupospelov@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) autoformat FILENUM whitespaces pairable unpairable nocheck

#[macro_use]
extern crate uucore;

use clap::{crate_version, Arg, Command};
use memchr::{memchr3_iter, memchr_iter};
use std::cmp::Ordering;
use std::convert::From;
use std::error::Error;
use std::fmt::Display;
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Split, Stdin, Write};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use uucore::display::Quotable;
use uucore::error::{set_exit_code, UError, UResult, USimpleError};

static NAME: &str = "join";

#[derive(Debug)]
enum JoinError {
    IOError(std::io::Error),
    UnorderedInput(String),
}

impl UError for JoinError {
    fn code(&self) -> i32 {
        1
    }
}

impl Error for JoinError {}

impl Display for JoinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JoinError::IOError(e) => write!(f, "io error: {}", e),
            JoinError::UnorderedInput(e) => f.write_str(e),
        }
    }
}

impl From<std::io::Error> for JoinError {
    fn from(error: std::io::Error) -> Self {
        Self::IOError(error)
    }
}

#[derive(Copy, Clone, PartialEq)]
enum FileNum {
    File1,
    File2,
}

#[repr(u8)]
#[derive(Copy, Clone)]
enum LineEnding {
    Nul = 0,
    Newline = b'\n',
}

#[derive(Copy, Clone, PartialEq)]
enum Sep {
    Char(u8),
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
    print_unpaired1: bool,
    print_unpaired2: bool,
    print_joined: bool,
    ignore_case: bool,
    line_ending: LineEnding,
    separator: Sep,
    autoformat: bool,
    format: Vec<Spec>,
    empty: Vec<u8>,
    check_order: CheckOrder,
    headers: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            key1: 0,
            key2: 0,
            print_unpaired1: false,
            print_unpaired2: false,
            print_joined: true,
            ignore_case: false,
            line_ending: LineEnding::Newline,
            separator: Sep::Whitespaces,
            autoformat: false,
            format: vec![],
            empty: vec![],
            check_order: CheckOrder::Default,
            headers: false,
        }
    }
}

/// Output representation.
struct Repr<'a> {
    line_ending: LineEnding,
    separator: u8,
    format: &'a [Spec],
    empty: &'a [u8],
}

impl<'a> Repr<'a> {
    fn new(
        line_ending: LineEnding,
        separator: u8,
        format: &'a [Spec],
        empty: &'a [u8],
    ) -> Repr<'a> {
        Repr {
            line_ending,
            separator,
            format,
            empty,
        }
    }

    fn uses_format(&self) -> bool {
        !self.format.is_empty()
    }

    /// Print the field or empty filler if the field is not set.
    fn print_field(
        &self,
        writer: &mut impl Write,
        field: Option<&[u8]>,
    ) -> Result<(), std::io::Error> {
        let value = match field {
            Some(field) => field,
            None => self.empty,
        };

        writer.write_all(value)
    }

    /// Print each field except the one at the index.
    fn print_fields(
        &self,
        writer: &mut impl Write,
        line: &Line,
        index: usize,
    ) -> Result<(), std::io::Error> {
        for i in 0..line.field_ranges.len() {
            if i != index {
                writer.write_all(&[self.separator])?;
                writer.write_all(line.get_field(i).unwrap())?;
            }
        }
        Ok(())
    }

    /// Print each field or the empty filler if the field is not set.
    fn print_format<F>(&self, writer: &mut impl Write, f: F) -> Result<(), std::io::Error>
    where
        F: Fn(&Spec) -> Option<&'a [u8]>,
    {
        for i in 0..self.format.len() {
            if i > 0 {
                writer.write_all(&[self.separator])?;
            }

            let field = match f(&self.format[i]) {
                Some(value) => value,
                None => self.empty,
            };

            writer.write_all(field)?;
        }
        Ok(())
    }

    fn print_line_ending(&self, writer: &mut impl Write) -> Result<(), std::io::Error> {
        writer.write_all(&[self.line_ending as u8])
    }
}

/// Input processing parameters.
struct Input {
    separator: Sep,
    ignore_case: bool,
    check_order: CheckOrder,
}

impl Input {
    fn new(separator: Sep, ignore_case: bool, check_order: CheckOrder) -> Self {
        Self {
            separator,
            ignore_case,
            check_order,
        }
    }

    fn compare(&self, field1: Option<&[u8]>, field2: Option<&[u8]>) -> Ordering {
        if let (Some(field1), Some(field2)) = (field1, field2) {
            if self.ignore_case {
                field1
                    .to_ascii_lowercase()
                    .cmp(&field2.to_ascii_lowercase())
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
    fn parse(format: &str) -> UResult<Self> {
        let mut chars = format.chars();

        let file_num = match chars.next() {
            Some('0') => {
                // Must be all alone without a field specifier.
                if chars.next().is_none() {
                    return Ok(Self::Key);
                }
                return Err(USimpleError::new(
                    1,
                    format!("invalid field specifier: {}", format.quote()),
                ));
            }
            Some('1') => FileNum::File1,
            Some('2') => FileNum::File2,
            _ => {
                return Err(USimpleError::new(
                    1,
                    format!("invalid file number in field spec: {}", format.quote()),
                ));
            }
        };

        if let Some('.') = chars.next() {
            return Ok(Self::Field(file_num, parse_field_number(chars.as_str())?));
        }

        Err(USimpleError::new(
            1,
            format!("invalid field specifier: {}", format.quote()),
        ))
    }
}

struct Line {
    field_ranges: Vec<(usize, usize)>,
    string: Vec<u8>,
}

impl Line {
    fn new(string: Vec<u8>, separator: Sep, len_guess: usize) -> Self {
        let mut field_ranges = Vec::with_capacity(len_guess);
        let mut last_end = 0;
        if separator == Sep::Whitespaces {
            // GNU join uses Bourne shell field splitters by default
            for i in memchr3_iter(b' ', b'\t', b'\n', &string) {
                if i > last_end {
                    field_ranges.push((last_end, i));
                }
                last_end = i + 1;
            }
        } else if let Sep::Char(sep) = separator {
            for i in memchr_iter(sep, &string) {
                field_ranges.push((last_end, i));
                last_end = i + 1;
            }
        }
        field_ranges.push((last_end, string.len()));

        Self {
            field_ranges,
            string,
        }
    }

    /// Get field at index.
    fn get_field(&self, index: usize) -> Option<&[u8]> {
        if index < self.field_ranges.len() {
            let (low, high) = self.field_ranges[index];
            Some(&self.string[low..high])
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
    lines: Split<Box<dyn BufRead + 'a>>,
    max_len: usize,
    seq: Vec<Line>,
    line_num: usize,
    has_failed: bool,
    has_unpaired: bool,
}

impl<'a> State<'a> {
    fn new(
        file_num: FileNum,
        name: &'a str,
        stdin: &'a Stdin,
        key: usize,
        line_ending: LineEnding,
        print_unpaired: bool,
    ) -> State<'a> {
        let f = if name == "-" {
            Box::new(stdin.lock()) as Box<dyn BufRead>
        } else {
            match File::open(name) {
                Ok(file) => Box::new(BufReader::new(file)) as Box<dyn BufRead>,
                Err(err) => crash!(1, "{}: {}", name.maybe_quote(), err),
            }
        };

        State {
            key,
            file_name: name,
            file_num,
            print_unpaired,
            lines: f.split(line_ending as u8),
            max_len: 1,
            seq: Vec::new(),
            line_num: 0,
            has_failed: false,
            has_unpaired: false,
        }
    }

    /// Skip the current unpaired line.
    fn skip_line(
        &mut self,
        writer: &mut impl Write,
        input: &Input,
        repr: &Repr,
    ) -> Result<(), JoinError> {
        if self.print_unpaired {
            self.print_first_line(writer, repr)?;
        }

        self.reset_next_line(input)?;
        Ok(())
    }

    /// Keep reading line sequence until the key does not change, return
    /// the first line whose key differs.
    fn extend(&mut self, input: &Input) -> Result<Option<Line>, JoinError> {
        while let Some(line) = self.next_line(input)? {
            let diff = input.compare(self.get_current_key(), line.get_field(self.key));

            if diff == Ordering::Equal {
                self.seq.push(line);
            } else {
                return Ok(Some(line));
            }
        }

        Ok(None)
    }

    /// Print lines in the buffers as headers.
    fn print_headers(
        &self,
        writer: &mut impl Write,
        other: &State,
        repr: &Repr,
    ) -> Result<(), std::io::Error> {
        if self.has_line() {
            if other.has_line() {
                self.combine(writer, other, repr)?;
            } else {
                self.print_first_line(writer, repr)?;
            }
        } else if other.has_line() {
            other.print_first_line(writer, repr)?;
        }

        Ok(())
    }

    /// Combine two line sequences.
    fn combine(
        &self,
        writer: &mut impl Write,
        other: &State,
        repr: &Repr,
    ) -> Result<(), std::io::Error> {
        let key = self.get_current_key();

        for line1 in &self.seq {
            for line2 in &other.seq {
                if repr.uses_format() {
                    repr.print_format(writer, |spec| match *spec {
                        Spec::Key => key,
                        Spec::Field(file_num, field_num) => {
                            if file_num == self.file_num {
                                return line1.get_field(field_num);
                            }

                            if file_num == other.file_num {
                                return line2.get_field(field_num);
                            }

                            None
                        }
                    })?;
                } else {
                    repr.print_field(writer, key)?;
                    repr.print_fields(writer, line1, self.key)?;
                    repr.print_fields(writer, line2, other.key)?;
                }

                repr.print_line_ending(writer)?;
            }
        }

        Ok(())
    }

    /// Reset with the next line.
    fn reset(&mut self, next_line: Option<Line>) {
        self.seq.clear();

        if let Some(line) = next_line {
            self.seq.push(line);
        }
    }

    fn reset_read_line(&mut self, input: &Input) -> Result<(), std::io::Error> {
        let line = self.read_line(input.separator)?;
        self.reset(line);
        Ok(())
    }

    fn reset_next_line(&mut self, input: &Input) -> Result<(), JoinError> {
        let line = self.next_line(input)?;
        self.reset(line);
        Ok(())
    }

    fn has_line(&self) -> bool {
        !self.seq.is_empty()
    }

    fn initialize(&mut self, read_sep: Sep, autoformat: bool) -> usize {
        if let Some(line) = crash_if_err!(1, self.read_line(read_sep)) {
            self.seq.push(line);

            if autoformat {
                return self.seq[0].field_ranges.len();
            }
        }
        0
    }

    fn finalize(
        &mut self,
        writer: &mut impl Write,
        input: &Input,
        repr: &Repr,
    ) -> Result<(), JoinError> {
        if self.has_line() {
            if self.print_unpaired {
                self.print_first_line(writer, repr)?;
            }

            let mut next_line = self.next_line(input)?;
            while let Some(line) = &next_line {
                if self.print_unpaired {
                    self.print_line(writer, line, repr)?;
                }
                self.reset(next_line);
                next_line = self.next_line(input)?;
            }
        }

        Ok(())
    }

    /// Get the next line without the order check.
    fn read_line(&mut self, sep: Sep) -> Result<Option<Line>, std::io::Error> {
        match self.lines.next() {
            Some(value) => {
                self.line_num += 1;
                let line = Line::new(value?, sep, self.max_len);
                if line.field_ranges.len() > self.max_len {
                    self.max_len = line.field_ranges.len();
                }
                Ok(Some(line))
            }
            None => Ok(None),
        }
    }

    /// Get the next line with the order check.
    fn next_line(&mut self, input: &Input) -> Result<Option<Line>, JoinError> {
        if let Some(line) = self.read_line(input.separator)? {
            if input.check_order == CheckOrder::Disabled {
                return Ok(Some(line));
            }

            let diff = input.compare(self.get_current_key(), line.get_field(self.key));

            if diff == Ordering::Greater
                && (input.check_order == CheckOrder::Enabled
                    || (self.has_unpaired && !self.has_failed))
            {
                let err_msg = format!(
                    "{}:{}: is not sorted: {}",
                    self.file_name.maybe_quote(),
                    self.line_num,
                    String::from_utf8_lossy(&line.string)
                );
                // This is fatal if the check is enabled.
                if input.check_order == CheckOrder::Enabled {
                    return Err(JoinError::UnorderedInput(err_msg));
                }
                eprintln!("{}: {}", uucore::execution_phrase(), err_msg);
                self.has_failed = true;
            }

            Ok(Some(line))
        } else {
            Ok(None)
        }
    }

    /// Gets the key value of the lines stored in seq.
    fn get_current_key(&self) -> Option<&[u8]> {
        self.seq[0].get_field(self.key)
    }

    fn print_line(
        &self,
        writer: &mut impl Write,
        line: &Line,
        repr: &Repr,
    ) -> Result<(), std::io::Error> {
        if repr.uses_format() {
            repr.print_format(writer, |spec| match *spec {
                Spec::Key => line.get_field(self.key),
                Spec::Field(file_num, field_num) => {
                    if file_num == self.file_num {
                        line.get_field(field_num)
                    } else {
                        None
                    }
                }
            })?;
        } else {
            repr.print_field(writer, line.get_field(self.key))?;
            repr.print_fields(writer, line, self.key)?;
        }

        repr.print_line_ending(writer)
    }

    fn print_first_line(&self, writer: &mut impl Write, repr: &Repr) -> Result<(), std::io::Error> {
        self.print_line(writer, &self.seq[0], repr)
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    let keys = parse_field_number_option(matches.value_of("j"))?;
    let key1 = parse_field_number_option(matches.value_of("1"))?;
    let key2 = parse_field_number_option(matches.value_of("2"))?;

    let mut settings: Settings = Default::default();

    let v_values = matches.values_of("v");
    if v_values.is_some() {
        settings.print_joined = false;
    }

    let unpaired = v_values
        .unwrap_or_default()
        .chain(matches.values_of("a").unwrap_or_default());
    for file_num in unpaired {
        match parse_file_number(file_num)? {
            FileNum::File1 => settings.print_unpaired1 = true,
            FileNum::File2 => settings.print_unpaired2 = true,
        }
    }

    settings.ignore_case = matches.is_present("i");
    settings.key1 = get_field_number(keys, key1)?;
    settings.key2 = get_field_number(keys, key2)?;

    if let Some(value_os) = matches.value_of_os("t") {
        #[cfg(unix)]
        let value = value_os.as_bytes();
        #[cfg(not(unix))]
        let value = match value_os.to_str() {
            Some(value) => value.as_bytes(),
            None => {
                return Err(USimpleError::new(
                    1,
                    "unprintable field separators are only supported on unix-like platforms",
                ))
            }
        };
        settings.separator = match value.len() {
            0 => Sep::Line,
            1 => Sep::Char(value[0]),
            2 if value[0] == b'\\' && value[1] == b'0' => Sep::Char(0),
            _ => {
                return Err(USimpleError::new(
                    1,
                    format!("multi-character tab {}", value_os.to_string_lossy()),
                ))
            }
        };
    }

    if let Some(format) = matches.value_of("o") {
        if format == "auto" {
            settings.autoformat = true;
        } else {
            let mut specs = vec![];
            for part in format.split(|c| c == ' ' || c == ',' || c == '\t') {
                specs.push(Spec::parse(part)?);
            }
            settings.format = specs;
        }
    }

    if let Some(empty) = matches.value_of("e") {
        settings.empty = empty.as_bytes().to_vec();
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

    if matches.is_present("z") {
        settings.line_ending = LineEnding::Nul;
    }

    let file1 = matches.value_of("file1").unwrap();
    let file2 = matches.value_of("file2").unwrap();

    if file1 == "-" && file2 == "-" {
        return Err(USimpleError::new(1, "both files cannot be standard input"));
    }

    match exec(file1, file2, settings) {
        Ok(_) => Ok(()),
        Err(e) => Err(USimpleError::new(1, format!("{}", e))),
    }
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(NAME)
        .version(crate_version!())
        .about(
            "For each pair of input lines with identical join fields, write a line to
standard output. The default join field is the first, delimited by blanks.

When FILE1 or FILE2 (not both) is -, read standard input.",
        )
        .infer_long_args(true)
        .arg(
            Arg::new("a")
                .short('a')
                .multiple_occurrences(true)
                .number_of_values(1)
                .possible_values(&["1", "2"])
                .value_name("FILENUM")
                .help(
                    "also print unpairable lines from file FILENUM, where
FILENUM is 1 or 2, corresponding to FILE1 or FILE2",
                ),
        )
        .arg(
            Arg::new("v")
                .short('v')
                .multiple_occurrences(true)
                .number_of_values(1)
                .possible_values(&["1", "2"])
                .value_name("FILENUM")
                .help("like -a FILENUM, but suppress joined output lines"),
        )
        .arg(
            Arg::new("e")
                .short('e')
                .takes_value(true)
                .value_name("EMPTY")
                .help("replace missing input fields with EMPTY"),
        )
        .arg(
            Arg::new("i")
                .short('i')
                .long("ignore-case")
                .help("ignore differences in case when comparing fields"),
        )
        .arg(
            Arg::new("j")
                .short('j')
                .takes_value(true)
                .value_name("FIELD")
                .help("equivalent to '-1 FIELD -2 FIELD'"),
        )
        .arg(
            Arg::new("o")
                .short('o')
                .takes_value(true)
                .value_name("FORMAT")
                .help("obey FORMAT while constructing output line"),
        )
        .arg(
            Arg::new("t")
                .short('t')
                .takes_value(true)
                .value_name("CHAR")
                .allow_invalid_utf8(true)
                .help("use CHAR as input and output field separator"),
        )
        .arg(
            Arg::new("1")
                .short('1')
                .takes_value(true)
                .value_name("FIELD")
                .help("join on this FIELD of file 1"),
        )
        .arg(
            Arg::new("2")
                .short('2')
                .takes_value(true)
                .value_name("FIELD")
                .help("join on this FIELD of file 2"),
        )
        .arg(Arg::new("check-order").long("check-order").help(
            "check that the input is correctly sorted, \
             even if all input lines are pairable",
        ))
        .arg(
            Arg::new("nocheck-order")
                .long("nocheck-order")
                .help("do not check that the input is correctly sorted"),
        )
        .arg(Arg::new("header").long("header").help(
            "treat the first line in each file as field headers, \
             print them without trying to pair them",
        ))
        .arg(
            Arg::new("z")
                .short('z')
                .long("zero-terminated")
                .help("line delimiter is NUL, not newline"),
        )
        .arg(
            Arg::new("file1")
                .required(true)
                .value_name("FILE1")
                .hide(true),
        )
        .arg(
            Arg::new("file2")
                .required(true)
                .value_name("FILE2")
                .hide(true),
        )
}

fn exec(file1: &str, file2: &str, settings: Settings) -> Result<(), JoinError> {
    let stdin = stdin();

    let mut state1 = State::new(
        FileNum::File1,
        file1,
        &stdin,
        settings.key1,
        settings.line_ending,
        settings.print_unpaired1,
    );

    let mut state2 = State::new(
        FileNum::File2,
        file2,
        &stdin,
        settings.key2,
        settings.line_ending,
        settings.print_unpaired2,
    );

    let input = Input::new(
        settings.separator,
        settings.ignore_case,
        settings.check_order,
    );

    let format = if settings.autoformat {
        let mut format = vec![Spec::Key];
        let mut initialize = |state: &mut State| {
            let max_fields = state.initialize(settings.separator, settings.autoformat);
            for i in 0..max_fields {
                if i != state.key {
                    format.push(Spec::Field(state.file_num, i));
                }
            }
        };
        initialize(&mut state1);
        initialize(&mut state2);
        format
    } else {
        state1.initialize(settings.separator, settings.autoformat);
        state2.initialize(settings.separator, settings.autoformat);
        settings.format
    };

    let repr = Repr::new(
        settings.line_ending,
        match settings.separator {
            Sep::Char(sep) => sep,
            _ => b' ',
        },
        &format,
        &settings.empty,
    );

    let stdout = stdout();
    let mut writer = BufWriter::new(stdout.lock());

    if settings.headers {
        state1.print_headers(&mut writer, &state2, &repr)?;
        state1.reset_read_line(&input)?;
        state2.reset_read_line(&input)?;
    }

    while state1.has_line() && state2.has_line() {
        let diff = input.compare(state1.get_current_key(), state2.get_current_key());

        match diff {
            Ordering::Less => {
                if let Err(e) = state1.skip_line(&mut writer, &input, &repr) {
                    writer.flush()?;
                    return Err(e);
                }
                state1.has_unpaired = true;
                state2.has_unpaired = true;
            }
            Ordering::Greater => {
                if let Err(e) = state2.skip_line(&mut writer, &input, &repr) {
                    writer.flush()?;
                    return Err(e);
                }
                state1.has_unpaired = true;
                state2.has_unpaired = true;
            }
            Ordering::Equal => {
                let next_line1 = match state1.extend(&input) {
                    Ok(line) => line,
                    Err(e) => {
                        writer.flush()?;
                        return Err(e);
                    }
                };
                let next_line2 = match state2.extend(&input) {
                    Ok(line) => line,
                    Err(e) => {
                        writer.flush()?;
                        return Err(e);
                    }
                };

                if settings.print_joined {
                    state1.combine(&mut writer, &state2, &repr)?;
                }

                state1.reset(next_line1);
                state2.reset(next_line2);
            }
        }
    }

    if let Err(e) = state1.finalize(&mut writer, &input, &repr) {
        writer.flush()?;
        return Err(e);
    };
    if let Err(e) = state2.finalize(&mut writer, &input, &repr) {
        writer.flush()?;
        return Err(e);
    };

    writer.flush()?;

    if state1.has_failed || state2.has_failed {
        eprintln!(
            "{}: input is not in sorted order",
            uucore::execution_phrase()
        );
        set_exit_code(1);
    }
    Ok(())
}

/// Check that keys for both files and for a particular file are not
/// contradictory and return the key index.
fn get_field_number(keys: Option<usize>, key: Option<usize>) -> UResult<usize> {
    if let Some(keys) = keys {
        if let Some(key) = key {
            if keys != key {
                // Show zero-based field numbers as one-based.
                return Err(USimpleError::new(
                    1,
                    format!("incompatible join fields {}, {}", keys + 1, key + 1),
                ));
            }
        }

        return Ok(keys);
    }

    Ok(key.unwrap_or(0))
}

/// Parse the specified field string as a natural number and return
/// the zero-based field number.
fn parse_field_number(value: &str) -> UResult<usize> {
    // TODO: use ParseIntError.kind() once MSRV >= 1.55
    // For now, store an overflow Err from parsing a value 10x 64 bit usize::MAX
    // Adapted from https://github.com/rust-lang/rust/issues/22639
    let overflow = "184467440737095516150".parse::<usize>().err().unwrap();
    match value.parse::<usize>() {
        Ok(result) if result > 0 => Ok(result - 1),
        Err(ref e) if *e == overflow => Ok(usize::MAX),
        _ => Err(USimpleError::new(
            1,
            format!("invalid field number: {}", value.quote()),
        )),
    }
}

fn parse_file_number(value: &str) -> UResult<FileNum> {
    match value {
        "1" => Ok(FileNum::File1),
        "2" => Ok(FileNum::File2),
        value => Err(USimpleError::new(
            1,
            format!("invalid file number: {}", value.quote()),
        )),
    }
}

fn parse_field_number_option(value: Option<&str>) -> UResult<Option<usize>> {
    match value {
        None => Ok(None),
        Some(val) => Ok(Some(parse_field_number(val)?)),
    }
}
