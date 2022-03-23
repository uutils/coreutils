//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Boden Garman <bpgarman@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

#[macro_use]
extern crate uucore;

mod count_fast;
mod countable;
mod word_count;
use count_fast::{count_bytes_and_lines_fast, count_bytes_fast};
use countable::WordCountable;
use unicode_width::UnicodeWidthChar;
use utf8::{BufReadDecoder, BufReadDecoderError};
use uucore::format_usage;
use word_count::{TitledWordCount, WordCount};

use clap::{crate_version, Arg, ArgMatches, Command};

use std::cmp::max;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt::Display;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use uucore::display::{Quotable, Quoted};
use uucore::error::{UError, UResult, USimpleError};

/// The minimum character width for formatting counts when reading from stdin.
const MINIMUM_WIDTH: usize = 7;

struct Settings {
    show_bytes: bool,
    show_chars: bool,
    show_lines: bool,
    show_words: bool,
    show_max_line_length: bool,
}

impl Settings {
    fn new(matches: &ArgMatches) -> Self {
        let settings = Self {
            show_bytes: matches.is_present(options::BYTES),
            show_chars: matches.is_present(options::CHAR),
            show_lines: matches.is_present(options::LINES),
            show_words: matches.is_present(options::WORDS),
            show_max_line_length: matches.is_present(options::MAX_LINE_LENGTH),
        };

        if settings.show_bytes
            || settings.show_chars
            || settings.show_lines
            || settings.show_words
            || settings.show_max_line_length
        {
            return settings;
        }

        Self {
            show_bytes: true,
            show_chars: false,
            show_lines: true,
            show_words: true,
            show_max_line_length: false,
        }
    }

    fn number_enabled(&self) -> u32 {
        let mut result = 0;
        result += self.show_bytes as u32;
        result += self.show_chars as u32;
        result += self.show_lines as u32;
        result += self.show_max_line_length as u32;
        result += self.show_words as u32;
        result
    }
}

static ABOUT: &str = "Display newline, word, and byte counts for each FILE, and a total line if
more than one FILE is specified. With no FILE, or when FILE is -, read standard input.";
const USAGE: &str = "{} [OPTION]... [FILE]...";

pub mod options {
    pub static BYTES: &str = "bytes";
    pub static CHAR: &str = "chars";
    pub static FILES0_FROM: &str = "files0-from";
    pub static LINES: &str = "lines";
    pub static MAX_LINE_LENGTH: &str = "max-line-length";
    pub static WORDS: &str = "words";
}

static ARG_FILES: &str = "files";
static STDIN_REPR: &str = "-";

enum StdinKind {
    /// Stdin specified on command-line with "-".
    Explicit,

    /// Stdin implicitly specified on command-line by not passing any positional argument.
    Implicit,
}

/// Supported inputs.
enum Input {
    /// A regular file.
    Path(PathBuf),

    /// Standard input.
    Stdin(StdinKind),
}

impl From<&OsStr> for Input {
    fn from(input: &OsStr) -> Self {
        if input == STDIN_REPR {
            Self::Stdin(StdinKind::Explicit)
        } else {
            Self::Path(input.into())
        }
    }
}

impl Input {
    /// Converts input to title that appears in stats.
    fn to_title(&self) -> Option<&Path> {
        match self {
            Input::Path(path) => Some(path),
            Input::Stdin(StdinKind::Explicit) => Some(STDIN_REPR.as_ref()),
            Input::Stdin(StdinKind::Implicit) => None,
        }
    }

    fn path_display(&self) -> Quoted<'_> {
        match self {
            Input::Path(path) => path.maybe_quote(),
            Input::Stdin(_) => "standard input".maybe_quote(),
        }
    }
}

#[derive(Debug)]
enum WcError {
    FilesDisabled(String),
    StdinReprNotAllowed(String),
}

impl UError for WcError {
    fn code(&self) -> i32 {
        match self {
            WcError::FilesDisabled(_) | WcError::StdinReprNotAllowed(_) => 1,
        }
    }

    fn usage(&self) -> bool {
        matches!(self, WcError::FilesDisabled(_))
    }
}

impl Error for WcError {}

impl Display for WcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WcError::FilesDisabled(message) | WcError::StdinReprNotAllowed(message) => {
                write!(f, "{}", message)
            }
        }
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    let inputs = inputs(&matches)?;

    let settings = Settings::new(&matches);

    wc(&inputs, &settings)
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::BYTES)
                .short('c')
                .long(options::BYTES)
                .help("print the byte counts"),
        )
        .arg(
            Arg::new(options::CHAR)
                .short('m')
                .long(options::CHAR)
                .help("print the character counts"),
        )
        .arg(
            Arg::new(options::FILES0_FROM)
                .long(options::FILES0_FROM)
                .takes_value(true)
                .value_name("F")
                .help(
                    "read input from the files specified by
    NUL-terminated names in file F;
    If F is - then read names from standard input",
                ),
        )
        .arg(
            Arg::new(options::LINES)
                .short('l')
                .long(options::LINES)
                .help("print the newline counts"),
        )
        .arg(
            Arg::new(options::MAX_LINE_LENGTH)
                .short('L')
                .long(options::MAX_LINE_LENGTH)
                .help("print the length of the longest line"),
        )
        .arg(
            Arg::new(options::WORDS)
                .short('w')
                .long(options::WORDS)
                .help("print the word counts"),
        )
        .arg(
            Arg::new(ARG_FILES)
                .multiple_occurrences(true)
                .takes_value(true)
                .allow_invalid_utf8(true),
        )
}

fn inputs(matches: &ArgMatches) -> UResult<Vec<Input>> {
    match matches.values_of_os(ARG_FILES) {
        Some(os_values) => {
            if matches.is_present(options::FILES0_FROM) {
                return Err(WcError::FilesDisabled(
                    "file operands cannot be combined with --files0-from".into(),
                )
                .into());
            }

            Ok(os_values.map(Input::from).collect())
        }
        None => match matches.value_of(options::FILES0_FROM) {
            Some(files_0_from) => create_paths_from_files0(files_0_from),
            None => Ok(vec![Input::Stdin(StdinKind::Implicit)]),
        },
    }
}

fn create_paths_from_files0(files_0_from: &str) -> UResult<Vec<Input>> {
    let mut paths = String::new();
    let read_from_stdin = files_0_from == STDIN_REPR;

    if read_from_stdin {
        io::stdin().lock().read_to_string(&mut paths)?;
    } else {
        File::open(files_0_from)?.read_to_string(&mut paths)?;
    }

    let paths: Vec<&str> = paths.split_terminator('\0').collect();

    if read_from_stdin && paths.contains(&STDIN_REPR) {
        return Err(WcError::StdinReprNotAllowed(
            "when reading file names from stdin, no file name of '-' allowed".into(),
        )
        .into());
    }

    Ok(paths.iter().map(OsStr::new).map(Input::from).collect())
}

fn word_count_from_reader<T: WordCountable>(
    mut reader: T,
    settings: &Settings,
) -> (WordCount, Option<io::Error>) {
    let only_count_bytes = settings.show_bytes
        && (!(settings.show_chars
            || settings.show_lines
            || settings.show_max_line_length
            || settings.show_words));
    if only_count_bytes {
        let (bytes, error) = count_bytes_fast(&mut reader);
        return (
            WordCount {
                bytes,
                ..WordCount::default()
            },
            error,
        );
    }

    // we do not need to decode the byte stream if we're only counting bytes/newlines
    let decode_chars = settings.show_chars || settings.show_words || settings.show_max_line_length;

    if !decode_chars {
        return count_bytes_and_lines_fast(&mut reader);
    }

    let mut total = WordCount::default();
    let mut reader = BufReadDecoder::new(reader.buffered());
    let mut in_word = false;
    let mut current_len = 0;

    while let Some(chunk) = reader.next_strict() {
        match chunk {
            Ok(text) => {
                for ch in text.chars() {
                    if settings.show_words {
                        if ch.is_whitespace() {
                            in_word = false;
                        } else if ch.is_ascii_control() {
                            // These count as characters but do not affect the word state
                        } else if !in_word {
                            in_word = true;
                            total.words += 1;
                        }
                    }
                    if settings.show_max_line_length {
                        match ch {
                            '\n' | '\r' | '\x0c' => {
                                total.max_line_length = max(current_len, total.max_line_length);
                                current_len = 0;
                            }
                            '\t' => {
                                current_len -= current_len % 8;
                                current_len += 8;
                            }
                            _ => {
                                current_len += ch.width().unwrap_or(0);
                            }
                        }
                    }
                    if settings.show_lines && ch == '\n' {
                        total.lines += 1;
                    }
                    if settings.show_chars {
                        total.chars += 1;
                    }
                }
                total.bytes += text.len();
            }
            Err(BufReadDecoderError::InvalidByteSequence(bytes)) => {
                // GNU wc treats invalid data as neither word nor char nor whitespace,
                // so no other counters are affected
                total.bytes += bytes.len();
            }
            Err(BufReadDecoderError::Io(e)) => {
                return (total, Some(e));
            }
        }
    }

    total.max_line_length = max(current_len, total.max_line_length);

    (total, None)
}

enum CountResult {
    /// Nothing went wrong.
    Success(WordCount),
    /// Managed to open but failed to read.
    Interrupted(WordCount, io::Error),
    /// Didn't even manage to open.
    Failure(io::Error),
}

/// If we fail opening a file we only show the error. If we fail reading it
/// we show a count for what we managed to read.
///
/// Therefore the reading implementations always return a total and sometimes
/// return an error: (WordCount, Option<io::Error>).
fn word_count_from_input(input: &Input, settings: &Settings) -> CountResult {
    match input {
        Input::Stdin(_) => {
            let stdin = io::stdin();
            let stdin_lock = stdin.lock();
            match word_count_from_reader(stdin_lock, settings) {
                (total, Some(error)) => CountResult::Interrupted(total, error),
                (total, None) => CountResult::Success(total),
            }
        }
        Input::Path(path) => match File::open(path) {
            Err(error) => CountResult::Failure(error),
            Ok(file) => match word_count_from_reader(file, settings) {
                (total, Some(error)) => CountResult::Interrupted(total, error),
                (total, None) => CountResult::Success(total),
            },
        },
    }
}

/// Compute the number of digits needed to represent any count for this input.
///
/// If `input` is [`Input::Stdin`], then this function returns
/// [`MINIMUM_WIDTH`]. Otherwise, if metadata could not be read from
/// `input` then this function returns 1.
///
/// # Errors
///
/// This function will return an error if `input` is a [`Input::Path`]
/// and there is a problem accessing the metadata of the given `input`.
///
/// # Examples
///
/// A [`Input::Stdin`] gets a default minimum width:
///
/// ```rust,ignore
/// let input = Input::Stdin(StdinKind::Explicit);
/// assert_eq!(7, digit_width(input));
/// ```
fn digit_width(input: &Input) -> io::Result<usize> {
    match input {
        Input::Stdin(_) => Ok(MINIMUM_WIDTH),
        Input::Path(filename) => {
            let path = Path::new(filename);
            let metadata = fs::metadata(path)?;
            if metadata.is_file() {
                // TODO We are now computing the number of bytes in a file
                // twice: once here and once in `WordCount::from_line()` (or
                // in `count_bytes_fast()` if that function is called
                // instead). See GitHub issue #2201.
                let num_bytes = metadata.len();
                let num_digits = num_bytes.to_string().len();
                Ok(num_digits)
            } else {
                Ok(MINIMUM_WIDTH)
            }
        }
    }
}

/// Compute the number of digits needed to represent all counts in all inputs.
///
/// `inputs` may include zero or more [`Input::Stdin`] entries, each of
/// which represents reading from `stdin`. The presence of any such
/// entry causes this function to return a width that is at least
/// [`MINIMUM_WIDTH`].
///
/// If `input` is empty, then this function returns 1. If file metadata
/// could not be read from any of the [`Input::Path`] inputs and there
/// are no [`Input::Stdin`] inputs, then this function returns 1.
///
/// If there is a problem accessing the metadata, this function will
/// silently ignore the error and assume that the number of digits
/// needed to display the counts for that file is 1.
///
/// # Examples
///
/// An empty slice implies a width of 1:
///
/// ```rust,ignore
/// assert_eq!(1, max_width(&vec![]));
/// ```
///
/// The presence of [`Input::Stdin`] implies a minimum width:
///
/// ```rust,ignore
/// let inputs = vec![Input::Stdin(StdinKind::Explicit)];
/// assert_eq!(7, max_width(&inputs));
/// ```
fn max_width(inputs: &[Input]) -> usize {
    let mut result = 1;
    for input in inputs {
        if let Ok(n) = digit_width(input) {
            result = result.max(n);
        }
    }
    result
}

fn wc(inputs: &[Input], settings: &Settings) -> UResult<()> {
    // Compute the width, in digits, to use when formatting counts.
    //
    // The width is the number of digits needed to print the number of
    // bytes in the largest file. This is true regardless of whether
    // the `settings` indicate that the bytes will be displayed.
    //
    // If we only need to display a single number, set this to 0 to
    // prevent leading spaces.
    let max_width = if settings.number_enabled() <= 1 {
        0
    } else {
        max_width(inputs)
    };

    let mut total_word_count = WordCount::default();

    let num_inputs = inputs.len();

    for input in inputs {
        let word_count = match word_count_from_input(input, settings) {
            CountResult::Success(word_count) => word_count,
            CountResult::Interrupted(word_count, error) => {
                show!(USimpleError::new(
                    1,
                    format!("{}: {}", input.path_display(), error)
                ));
                word_count
            }
            CountResult::Failure(error) => {
                show!(USimpleError::new(
                    1,
                    format!("{}: {}", input.path_display(), error)
                ));
                continue;
            }
        };
        total_word_count += word_count;
        let result = word_count.with_title(input.to_title());
        if let Err(err) = print_stats(settings, &result, max_width) {
            show!(USimpleError::new(
                1,
                format!(
                    "failed to print result for {}: {}",
                    result
                        .title
                        .unwrap_or_else(|| "<stdin>".as_ref())
                        .maybe_quote(),
                    err,
                ),
            ));
        }
    }

    if num_inputs > 1 {
        let total_result = total_word_count.with_title(Some("total".as_ref()));
        if let Err(err) = print_stats(settings, &total_result, max_width) {
            show!(USimpleError::new(
                1,
                format!("failed to print total: {}", err)
            ));
        }
    }

    // Although this appears to be returning `Ok`, the exit code may
    // have been set to a non-zero value by a call to `show!()` above.
    Ok(())
}

fn print_stats(settings: &Settings, result: &TitledWordCount, min_width: usize) -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    let mut is_first: bool = true;

    if settings.show_lines {
        if !is_first {
            write!(stdout_lock, " ")?;
        }
        write!(stdout_lock, "{:1$}", result.count.lines, min_width)?;
        is_first = false;
    }
    if settings.show_words {
        if !is_first {
            write!(stdout_lock, " ")?;
        }
        write!(stdout_lock, "{:1$}", result.count.words, min_width)?;
        is_first = false;
    }
    if settings.show_chars {
        if !is_first {
            write!(stdout_lock, " ")?;
        }
        write!(stdout_lock, "{:1$}", result.count.chars, min_width)?;
        is_first = false;
    }
    if settings.show_bytes {
        if !is_first {
            write!(stdout_lock, " ")?;
        }
        write!(stdout_lock, "{:1$}", result.count.bytes, min_width)?;
        is_first = false;
    }
    if settings.show_max_line_length {
        if !is_first {
            write!(stdout_lock, " ")?;
        }
        write!(
            stdout_lock,
            "{:1$}",
            result.count.max_line_length, min_width
        )?;
    }

    if let Some(title) = result.title {
        writeln!(stdout_lock, " {}", title.maybe_quote())?;
    } else {
        writeln!(stdout_lock)?;
    }

    Ok(())
}
