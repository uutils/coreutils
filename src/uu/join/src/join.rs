// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) autoformat FILENUM whitespaces pairable unpairable nocheck memmem

use clap::builder::ValueParser;
use clap::{Arg, ArgAction, Command};
use memchr::{Memchr3, memchr_iter, memmem::Finder};
use std::cmp::Ordering;
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Split, Stdin, Write, stdin, stdout};
use std::num::IntErrorKind;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::{FromIo, UError, UResult, USimpleError, set_exit_code};
use uucore::format_usage;
use uucore::line_ending::LineEnding;
use uucore::translate;

#[derive(Debug, Error)]
enum JoinError {
    #[error("{}", translate!("join-error-io", "error" => .0))]
    IOError(#[from] std::io::Error),

    #[error("{0}")]
    UnorderedInput(String),
}

// If you still need the UError implementation for compatibility:
impl UError for JoinError {
    fn code(&self) -> i32 {
        1
    }
}

#[derive(Copy, Clone, PartialEq)]
enum FileNum {
    File1,
    File2,
}

#[derive(Clone)]
enum SepSetting {
    /// Any single-byte separator.
    Byte(u8),
    /// A single character more than one byte long.
    Char(Vec<u8>),
    /// No separators, join on the entire line.
    Line,
    /// Whitespace separators.
    Whitespaces,
}

trait Separator: Clone {
    /// Using this separator, return the start and end index of all fields in the haystack.
    fn field_ranges(&self, haystack: &[u8], len_guess: usize) -> Vec<(usize, usize)>;
    /// The separator as it appears when in the output.
    fn output_separator(&self) -> &[u8];
}

/// Simple separators one byte in length.
#[derive(Copy, Clone)]
struct OneByteSep {
    byte: [u8; 1],
}

impl Separator for OneByteSep {
    fn field_ranges(&self, haystack: &[u8], len_guess: usize) -> Vec<(usize, usize)> {
        let mut field_ranges = Vec::with_capacity(len_guess);
        let mut last_end = 0;

        for i in memchr_iter(self.byte[0], haystack) {
            field_ranges.push((last_end, i));
            last_end = i + 1;
        }
        field_ranges.push((last_end, haystack.len()));
        field_ranges
    }

    fn output_separator(&self) -> &[u8] {
        &self.byte
    }
}

/// Multi-byte (but still single character) separators.
#[derive(Clone)]
struct MultiByteSep<'a> {
    finder: Finder<'a>,
}

impl Separator for MultiByteSep<'_> {
    fn field_ranges(&self, haystack: &[u8], len_guess: usize) -> Vec<(usize, usize)> {
        let mut field_ranges = Vec::with_capacity(len_guess);
        let mut last_end = 0;

        for i in self.finder.find_iter(haystack) {
            field_ranges.push((last_end, i));
            last_end = i + self.finder.needle().len();
        }
        field_ranges.push((last_end, haystack.len()));
        field_ranges
    }

    fn output_separator(&self) -> &[u8] {
        self.finder.needle()
    }
}

/// Whole-line separator.
#[derive(Copy, Clone)]
struct LineSep {}

impl Separator for LineSep {
    fn field_ranges(&self, haystack: &[u8], _len_guess: usize) -> Vec<(usize, usize)> {
        vec![(0, haystack.len())]
    }

    fn output_separator(&self) -> &[u8] {
        &[]
    }
}

/// Default whitespace separator.
#[derive(Copy, Clone)]
struct WhitespaceSep {}

impl Separator for WhitespaceSep {
    fn field_ranges(&self, haystack: &[u8], len_guess: usize) -> Vec<(usize, usize)> {
        let mut field_ranges = Vec::with_capacity(len_guess);
        let mut last_end = 0;

        // GNU join used Bourne shell field splitters by default
        // FIXME: but now uses locale-dependent whitespace
        for i in Memchr3::new(b' ', b'\t', b'\n', haystack) {
            // leading whitespace should be dropped, contiguous whitespace merged
            if i > last_end {
                field_ranges.push((last_end, i));
            }
            last_end = i + 1;
        }
        field_ranges.push((last_end, haystack.len()));
        field_ranges
    }

    fn output_separator(&self) -> &[u8] {
        b" "
    }
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
    separator: SepSetting,
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
            separator: SepSetting::Whitespaces,
            autoformat: false,
            format: vec![],
            empty: vec![],
            check_order: CheckOrder::Default,
            headers: false,
        }
    }
}

/// Output representation.
struct Repr<'a, Sep: Separator> {
    line_ending: LineEnding,
    separator: Sep,
    format: Vec<Spec>,
    empty: &'a [u8],
}

impl<'a, Sep: Separator> Repr<'a, Sep> {
    fn new(line_ending: LineEnding, separator: Sep, format: Vec<Spec>, empty: &'a [u8]) -> Self {
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
                writer.write_all(self.separator.output_separator())?;
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
                writer.write_all(self.separator.output_separator())?;
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

/// Byte slice wrapper whose Ord implementation is case-insensitive on ASCII.
#[derive(Eq)]
struct CaseInsensitiveSlice<'a> {
    v: &'a [u8],
}

impl Ord for CaseInsensitiveSlice<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        if let Some((s, o)) =
            std::iter::zip(self.v.iter(), other.v.iter()).find(|(s, o)| !s.eq_ignore_ascii_case(o))
        {
            // first characters that differ, return the case-insensitive comparison
            let s = s.to_ascii_lowercase();
            let o = o.to_ascii_lowercase();
            s.cmp(&o)
        } else {
            // one of the strings is a substring or equal of the other
            self.v.len().cmp(&other.v.len())
        }
    }
}

impl PartialOrd for CaseInsensitiveSlice<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for CaseInsensitiveSlice<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.v.eq_ignore_ascii_case(other.v)
    }
}

/// Input processing parameters.
struct Input<Sep: Separator> {
    separator: Sep,
    ignore_case: bool,
    check_order: CheckOrder,
}

impl<Sep: Separator> Input<Sep> {
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
                let field1 = CaseInsensitiveSlice { v: field1 };
                let field2 = CaseInsensitiveSlice { v: field2 };
                field1.cmp(&field2)
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
                    translate!("join-error-invalid-field-specifier", "spec" => format.quote()),
                ));
            }
            Some('1') => FileNum::File1,
            Some('2') => FileNum::File2,
            _ => {
                return Err(USimpleError::new(
                    1,
                    translate!("join-error-invalid-file-number", "spec" => format.quote()),
                ));
            }
        };

        if let Some('.') = chars.next() {
            return Ok(Self::Field(file_num, parse_field_number(chars.as_str())?));
        }

        Err(USimpleError::new(
            1,
            translate!("join-error-invalid-field-specifier", "spec" => format.quote()),
        ))
    }
}

struct Line {
    field_ranges: Vec<(usize, usize)>,
    string: Vec<u8>,
}

impl Line {
    fn new<Sep: Separator>(string: Vec<u8>, separator: &Sep, len_guess: usize) -> Self {
        let field_ranges = separator.field_ranges(&string, len_guess);

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
    file_name: &'a OsString,
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
        name: &'a OsString,
        stdin: &'a Stdin,
        key: usize,
        line_ending: LineEnding,
        print_unpaired: bool,
    ) -> UResult<Self> {
        let file_buf = if name == "-" {
            Box::new(stdin.lock()) as Box<dyn BufRead>
        } else {
            let file = File::open(name).map_err_context(|| format!("{}", name.maybe_quote()))?;
            Box::new(BufReader::new(file)) as Box<dyn BufRead>
        };

        Ok(State {
            key,
            file_name: name,
            file_num,
            print_unpaired,
            lines: file_buf.split(line_ending as u8),
            max_len: 1,
            seq: Vec::new(),
            line_num: 0,
            has_failed: false,
            has_unpaired: false,
        })
    }

    /// Skip the current unpaired line.
    fn skip_line<Sep: Separator>(
        &mut self,
        writer: &mut impl Write,
        input: &Input<Sep>,
        repr: &Repr<'a, Sep>,
    ) -> UResult<()> {
        if self.print_unpaired {
            self.print_first_line(writer, repr)?;
        }

        self.reset_next_line(input)?;
        Ok(())
    }

    /// Keep reading line sequence until the key does not change, return
    /// the first line whose key differs.
    fn extend<Sep: Separator>(&mut self, input: &Input<Sep>) -> UResult<Option<Line>> {
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
    fn print_headers<Sep: Separator>(
        &self,
        writer: &mut impl Write,
        other: &State,
        repr: &Repr<'a, Sep>,
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
    fn combine<Sep: Separator>(
        &self,
        writer: &mut impl Write,
        other: &State,
        repr: &Repr<'a, Sep>,
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

    fn reset_read_line<Sep: Separator>(
        &mut self,
        input: &Input<Sep>,
    ) -> Result<(), std::io::Error> {
        let line = self.read_line(&input.separator)?;
        self.reset(line);
        Ok(())
    }

    fn reset_next_line<Sep: Separator>(&mut self, input: &Input<Sep>) -> Result<(), JoinError> {
        let line = self.next_line(input)?;
        self.reset(line);
        Ok(())
    }

    fn has_line(&self) -> bool {
        !self.seq.is_empty()
    }

    fn initialize<Sep: Separator>(
        &mut self,
        read_sep: &Sep,
        autoformat: bool,
    ) -> std::io::Result<usize> {
        if let Some(line) = self.read_line(read_sep)? {
            self.seq.push(line);

            if autoformat {
                return Ok(self.seq[0].field_ranges.len());
            }
        }
        Ok(0)
    }

    fn finalize<Sep: Separator>(
        &mut self,
        writer: &mut impl Write,
        input: &Input<Sep>,
        repr: &Repr<'a, Sep>,
    ) -> UResult<()> {
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
    fn read_line<Sep: Separator>(&mut self, sep: &Sep) -> Result<Option<Line>, std::io::Error> {
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
    fn next_line<Sep: Separator>(&mut self, input: &Input<Sep>) -> Result<Option<Line>, JoinError> {
        if let Some(line) = self.read_line(&input.separator)? {
            if input.check_order == CheckOrder::Disabled {
                return Ok(Some(line));
            }

            let diff = input.compare(self.get_current_key(), line.get_field(self.key));

            if diff == Ordering::Greater
                && (input.check_order == CheckOrder::Enabled
                    || (self.has_unpaired && !self.has_failed))
            {
                let err_msg = translate!("join-error-not-sorted", "file" => self.file_name.maybe_quote(), "line_num" => self.line_num, "content" => String::from_utf8_lossy(&line.string));
                // This is fatal if the check is enabled.
                if input.check_order == CheckOrder::Enabled {
                    return Err(JoinError::UnorderedInput(err_msg));
                }
                eprintln!("{}: {err_msg}", uucore::execution_phrase());
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

    fn print_line<Sep: Separator>(
        &self,
        writer: &mut impl Write,
        line: &Line,
        repr: &Repr<'a, Sep>,
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

    fn print_first_line<Sep: Separator>(
        &self,
        writer: &mut impl Write,
        repr: &Repr<'a, Sep>,
    ) -> Result<(), std::io::Error> {
        self.print_line(writer, &self.seq[0], repr)
    }
}

fn parse_separator(value_os: &OsString) -> UResult<SepSetting> {
    // Five possible separator values:
    // No argument supplied, separate on whitespace; handled implicitly as the default elsewhere
    // An empty string arg, whole line separation
    // On unix-likes only, a single arbitrary byte
    // The two-character "\0" string, interpreted as a single 0 byte
    // A single scalar valid in the locale encoding (currently only UTF-8)

    if value_os.is_empty() {
        return Ok(SepSetting::Line);
    }

    #[cfg(unix)]
    {
        let value = value_os.as_bytes();
        if value.len() == 1 {
            return Ok(SepSetting::Byte(value[0]));
        }
    }

    let Some(value) = value_os.to_str() else {
        #[cfg(unix)]
        return Err(USimpleError::new(1, translate!("join-error-non-utf8-tab")));
        #[cfg(not(unix))]
        return Err(USimpleError::new(
            1,
            translate!("join-error-unprintable-separators"),
        ));
    };

    let mut chars = value.chars();
    let c = chars.next().expect("valid string with at least one byte");
    match chars.next() {
        None => Ok(SepSetting::Char(value.into())),
        Some('0') if c == '\\' => Ok(SepSetting::Byte(0)),
        _ => Err(USimpleError::new(
            1,
            translate!("join-error-multi-character-tab", "value" => value),
        )),
    }
}

fn parse_print_settings(matches: &clap::ArgMatches) -> UResult<(bool, bool, bool)> {
    let mut print_joined = true;
    let mut print_unpaired1 = false;
    let mut print_unpaired2 = false;

    let v_values = matches.get_many::<String>("v");
    if v_values.is_some() {
        print_joined = false;
    }

    let unpaired = v_values
        .unwrap_or_default()
        .chain(matches.get_many("a").unwrap_or_default());
    for file_num in unpaired {
        match parse_file_number(file_num)? {
            FileNum::File1 => print_unpaired1 = true,
            FileNum::File2 => print_unpaired2 = true,
        }
    }

    Ok((print_joined, print_unpaired1, print_unpaired2))
}

fn get_and_parse_field_number(matches: &clap::ArgMatches, key: &str) -> UResult<Option<usize>> {
    let value = matches.get_one::<String>(key).map(|s| s.as_str());
    parse_field_number_option(value)
}

/// Parses the command-line arguments and constructs a `Settings` struct.
///
/// This function takes the matches from the command-line arguments, processes them,
/// and returns a `Settings` struct that encapsulates the configuration for the program.
#[allow(clippy::field_reassign_with_default)]
fn parse_settings(matches: &clap::ArgMatches) -> UResult<Settings> {
    let keys = get_and_parse_field_number(matches, "j")?;
    let key1 = get_and_parse_field_number(matches, "1")?;
    let key2 = get_and_parse_field_number(matches, "2")?;

    let (print_joined, print_unpaired1, print_unpaired2) = parse_print_settings(matches)?;

    let mut settings = Settings::default();

    settings.print_joined = print_joined;
    settings.print_unpaired1 = print_unpaired1;
    settings.print_unpaired2 = print_unpaired2;

    settings.ignore_case = matches.get_flag("i");
    settings.key1 = get_field_number(keys, key1)?;
    settings.key2 = get_field_number(keys, key2)?;
    if let Some(value_os) = matches.get_one::<OsString>("t") {
        settings.separator = parse_separator(value_os)?;
    }
    if let Some(format) = matches.get_one::<String>("o") {
        if format == "auto" {
            settings.autoformat = true;
        } else {
            let mut specs = vec![];
            for part in format.split([' ', ',', '\t']) {
                specs.push(Spec::parse(part)?);
            }
            settings.format = specs;
        }
    }

    if let Some(empty) = matches.get_one::<String>("e") {
        settings.empty = empty.as_bytes().to_vec();
    }

    if matches.get_flag("nocheck-order") {
        settings.check_order = CheckOrder::Disabled;
    }

    if matches.get_flag("check-order") {
        settings.check_order = CheckOrder::Enabled;
    }

    if matches.get_flag("header") {
        settings.headers = true;
    }

    settings.line_ending = LineEnding::from_zero_flag(matches.get_flag("z"));

    Ok(settings)
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let settings = parse_settings(&matches)?;

    let file1 = matches.get_one::<OsString>("file1").unwrap();
    let file2 = matches.get_one::<OsString>("file2").unwrap();

    if file1 == "-" && file2 == "-" {
        return Err(USimpleError::new(
            1,
            translate!("join-error-both-files-stdin"),
        ));
    }

    let sep = settings.separator.clone();
    match sep {
        SepSetting::Byte(byte) => exec(file1, file2, settings, OneByteSep { byte: [byte] }),
        SepSetting::Char(c) => exec(
            file1,
            file2,
            settings,
            MultiByteSep {
                finder: Finder::new(&c),
            },
        ),
        SepSetting::Whitespaces => exec(file1, file2, settings, WhitespaceSep {}),
        SepSetting::Line => exec(file1, file2, settings, LineSep {}),
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("join-about"))
        .override_usage(format_usage(&translate!("join-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new("a")
                .short('a')
                .action(ArgAction::Append)
                .num_args(1)
                .value_parser(["1", "2"])
                .value_name("FILENUM")
                .help(translate!("join-help-a")),
        )
        .arg(
            Arg::new("v")
                .short('v')
                .action(ArgAction::Append)
                .num_args(1)
                .value_parser(["1", "2"])
                .value_name("FILENUM")
                .help(translate!("join-help-v")),
        )
        .arg(
            Arg::new("e")
                .short('e')
                .value_name("EMPTY")
                .help(translate!("join-help-e")),
        )
        .arg(
            Arg::new("i")
                .short('i')
                .long("ignore-case")
                .help(translate!("join-help-i"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("j")
                .short('j')
                .value_name("FIELD")
                .help(translate!("join-help-j")),
        )
        .arg(
            Arg::new("o")
                .short('o')
                .value_name("FORMAT")
                .help(translate!("join-help-o")),
        )
        .arg(
            Arg::new("t")
                .short('t')
                .value_name("CHAR")
                .value_parser(ValueParser::os_string())
                .help(translate!("join-help-t")),
        )
        .arg(
            Arg::new("1")
                .short('1')
                .value_name("FIELD")
                .help(translate!("join-help-1")),
        )
        .arg(
            Arg::new("2")
                .short('2')
                .value_name("FIELD")
                .help(translate!("join-help-2")),
        )
        .arg(
            Arg::new("check-order")
                .long("check-order")
                .help(translate!("join-help-check-order"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("nocheck-order")
                .long("nocheck-order")
                .help(translate!("join-help-nocheck-order"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("header")
                .long("header")
                .help(translate!("join-help-header"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("z")
                .short('z')
                .long("zero-terminated")
                .help(translate!("join-help-z"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("file1")
                .required(true)
                .value_name("FILE1")
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString))
                .hide(true),
        )
        .arg(
            Arg::new("file2")
                .required(true)
                .value_name("FILE2")
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString))
                .hide(true),
        )
}

fn exec<Sep: Separator>(
    file1: &OsString,
    file2: &OsString,
    settings: Settings,
    sep: Sep,
) -> UResult<()> {
    let stdin = stdin();

    let mut state1 = State::new(
        FileNum::File1,
        file1,
        &stdin,
        settings.key1,
        settings.line_ending,
        settings.print_unpaired1,
    )?;

    let mut state2 = State::new(
        FileNum::File2,
        file2,
        &stdin,
        settings.key2,
        settings.line_ending,
        settings.print_unpaired2,
    )?;

    let input = Input::new(sep.clone(), settings.ignore_case, settings.check_order);

    let format = if settings.autoformat {
        let mut format = vec![Spec::Key];
        let mut initialize = |state: &mut State| -> UResult<()> {
            let max_fields = state.initialize(&sep, settings.autoformat)?;
            for i in 0..max_fields {
                if i != state.key {
                    format.push(Spec::Field(state.file_num, i));
                }
            }
            Ok(())
        };
        initialize(&mut state1)?;
        initialize(&mut state2)?;
        format
    } else {
        state1.initialize(&sep, settings.autoformat)?;
        state2.initialize(&sep, settings.autoformat)?;
        settings.format
    };

    let repr = Repr::new(settings.line_ending, sep, format, &settings.empty);

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
    }
    if let Err(e) = state2.finalize(&mut writer, &input, &repr) {
        writer.flush()?;
        return Err(e);
    }

    writer.flush()?;

    if state1.has_failed || state2.has_failed {
        eprintln!(
            "{}: {}",
            uucore::execution_phrase(),
            translate!("join-error-input-not-sorted")
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
                    translate!("join-error-incompatible-fields", "field1" => (keys + 1), "field2" => (key + 1)),
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
    match value.parse::<usize>() {
        Ok(result) if result > 0 => Ok(result - 1),
        Err(e) if e.kind() == &IntErrorKind::PosOverflow => Ok(usize::MAX),
        _ => Err(USimpleError::new(
            1,
            translate!("join-error-invalid-field-number", "value" => value.quote()),
        )),
    }
}

fn parse_file_number(value: &str) -> UResult<FileNum> {
    match value {
        "1" => Ok(FileNum::File1),
        "2" => Ok(FileNum::File2),
        value => Err(USimpleError::new(
            1,
            translate!("join-error-invalid-file-number-simple", "value" => value.quote()),
        )),
    }
}

fn parse_field_number_option(value: Option<&str>) -> UResult<Option<usize>> {
    match value {
        None => Ok(None),
        Some(val) => Ok(Some(parse_field_number(val)?)),
    }
}
