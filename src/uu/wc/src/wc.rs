//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Boden Garman <bpgarman@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

#[macro_use]
extern crate uucore;

mod count_bytes;
mod countable;
mod word_count;
use count_bytes::count_bytes_fast;
use countable::WordCountable;
use word_count::{TitledWordCount, WordCount};

use clap::{crate_version, App, Arg, ArgMatches};
use thiserror::Error;

use std::fs::{self, File};
use std::io::{self, ErrorKind, Write};
use std::path::Path;

/// The minimum character width for formatting counts when reading from stdin.
const MINIMUM_WIDTH: usize = 7;

#[derive(Error, Debug)]
pub enum WcError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("Expected a file, found directory {0}")]
    IsDirectory(String),
}

type WcResult<T> = Result<T, WcError>;

struct Settings {
    show_bytes: bool,
    show_chars: bool,
    show_lines: bool,
    show_words: bool,
    show_max_line_length: bool,
}

impl Settings {
    fn new(matches: &ArgMatches) -> Settings {
        let settings = Settings {
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

        Settings {
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
more than one FILE is specified.";

pub mod options {
    pub static BYTES: &str = "bytes";
    pub static CHAR: &str = "chars";
    pub static LINES: &str = "lines";
    pub static MAX_LINE_LENGTH: &str = "max-line-length";
    pub static WORDS: &str = "words";
}

static ARG_FILES: &str = "files";

fn usage() -> String {
    format!(
        "{0} [OPTION]... [FILE]...
 With no FILE, or when FILE is -, read standard input.",
        executable!()
    )
}

enum StdinKind {
    /// Stdin specified on command-line with "-".
    Explicit,

    /// Stdin implicitly specified on command-line by not passing any positional argument.
    Implicit,
}

/// Supported inputs.
enum Input {
    /// A regular file.
    Path(String),

    /// Standard input.
    Stdin(StdinKind),
}

impl Input {
    /// Converts input to title that appears in stats.
    fn to_title(&self) -> Option<&str> {
        match self {
            Input::Path(path) => Some(path),
            Input::Stdin(StdinKind::Explicit) => Some("-"),
            Input::Stdin(StdinKind::Implicit) => None,
        }
    }
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = usage();

    let matches = uu_app().usage(&usage[..]).get_matches_from(args);

    let mut inputs: Vec<Input> = matches
        .values_of(ARG_FILES)
        .map(|v| {
            v.map(|i| {
                if i == "-" {
                    Input::Stdin(StdinKind::Explicit)
                } else {
                    Input::Path(ToString::to_string(i))
                }
            })
            .collect()
        })
        .unwrap_or_default();

    if inputs.is_empty() {
        inputs.push(Input::Stdin(StdinKind::Implicit));
    }

    let settings = Settings::new(&matches);

    if wc(inputs, &settings).is_ok() {
        0
    } else {
        1
    }
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(util_name!())
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::BYTES)
                .short("c")
                .long(options::BYTES)
                .help("print the byte counts"),
        )
        .arg(
            Arg::with_name(options::CHAR)
                .short("m")
                .long(options::CHAR)
                .help("print the character counts"),
        )
        .arg(
            Arg::with_name(options::LINES)
                .short("l")
                .long(options::LINES)
                .help("print the newline counts"),
        )
        .arg(
            Arg::with_name(options::MAX_LINE_LENGTH)
                .short("L")
                .long(options::MAX_LINE_LENGTH)
                .help("print the length of the longest line"),
        )
        .arg(
            Arg::with_name(options::WORDS)
                .short("w")
                .long(options::WORDS)
                .help("print the word counts"),
        )
        .arg(Arg::with_name(ARG_FILES).multiple(true).takes_value(true))
}

fn word_count_from_reader<T: WordCountable>(
    mut reader: T,
    settings: &Settings,
    path: &str,
) -> WcResult<WordCount> {
    let only_count_bytes = settings.show_bytes
        && (!(settings.show_chars
            || settings.show_lines
            || settings.show_max_line_length
            || settings.show_words));
    if only_count_bytes {
        return Ok(WordCount {
            bytes: count_bytes_fast(&mut reader)?,
            ..WordCount::default()
        });
    }

    // we do not need to decode the byte stream if we're only counting bytes/newlines
    let decode_chars = settings.show_chars || settings.show_words || settings.show_max_line_length;

    // Sum the WordCount for each line. Show a warning for each line
    // that results in an IO error when trying to read it.
    let total = reader
        .lines()
        .filter_map(|res| match res {
            Ok(line) => Some(line),
            Err(e) => {
                show_warning!("Error while reading {}: {}", path, e);
                None
            }
        })
        .map(|line| WordCount::from_line(&line, decode_chars))
        .sum();
    Ok(total)
}

fn word_count_from_input(input: &Input, settings: &Settings) -> WcResult<WordCount> {
    match input {
        Input::Stdin(_) => {
            let stdin = io::stdin();
            let stdin_lock = stdin.lock();
            word_count_from_reader(stdin_lock, settings, "-")
        }
        Input::Path(path) => {
            let path_obj = Path::new(path);
            if path_obj.is_dir() {
                Err(WcError::IsDirectory(path.to_owned()))
            } else {
                let file = File::open(path)?;
                word_count_from_reader(file, settings, path)
            }
        }
    }
}

/// Print a message appropriate for the particular error to `stderr`.
///
/// # Examples
///
/// This will print `wc: /tmp: Is a directory` to `stderr`.
///
/// ```rust,ignore
/// show_error(Input::Path("/tmp"), WcError::IsDirectory("/tmp"))
/// ```
fn show_error(input: &Input, err: WcError) {
    match (input, err) {
        (_, WcError::IsDirectory(path)) => {
            show_error_custom_description!(path, "Is a directory");
        }
        (Input::Path(path), WcError::Io(e)) if e.kind() == ErrorKind::NotFound => {
            show_error_custom_description!(path, "No such file or directory");
        }
        (_, e) => {
            show_error!("{}", e);
        }
    };
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
fn digit_width(input: &Input) -> WcResult<Option<usize>> {
    match input {
        Input::Stdin(_) => Ok(Some(MINIMUM_WIDTH)),
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
                Ok(Some(num_digits))
            } else {
                Ok(None)
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
        match digit_width(input) {
            Ok(maybe_n) => {
                if let Some(n) = maybe_n {
                    result = result.max(n);
                }
            }
            Err(_) => continue,
        }
    }
    result
}

fn wc(inputs: Vec<Input>, settings: &Settings) -> Result<(), u32> {
    // Compute the width, in digits, to use when formatting counts.
    //
    // The width is the number of digits needed to print the number of
    // bytes in the largest file. This is true regardless of whether
    // the `settings` indicate that the bytes will be displayed.
    let mut error_count = 0;
    let max_width = max_width(&inputs);

    let mut total_word_count = WordCount::default();

    let num_inputs = inputs.len();

    for input in &inputs {
        let word_count = word_count_from_input(input, settings).unwrap_or_else(|err| {
            show_error(input, err);
            error_count += 1;
            WordCount::default()
        });
        total_word_count += word_count;
        let result = word_count.with_title(input.to_title());
        if let Err(err) = print_stats(settings, &result, max_width) {
            show_warning!(
                "failed to print result for {}: {}",
                result.title.unwrap_or("<stdin>"),
                err
            );
            error_count += 1;
        }
    }

    if num_inputs > 1 {
        let total_result = total_word_count.with_title(Some("total"));
        if let Err(err) = print_stats(settings, &total_result, max_width) {
            show_warning!("failed to print total: {}", err);
            error_count += 1;
        }
    }

    if error_count == 0 {
        Ok(())
    } else {
        Err(error_count)
    }
}

fn print_stats(
    settings: &Settings,
    result: &TitledWordCount,
    mut min_width: usize,
) -> WcResult<()> {
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    if settings.number_enabled() <= 1 {
        // Prevent a leading space in case we only need to display a single
        // number.
        min_width = 0;
    }

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
    if settings.show_bytes {
        if !is_first {
            write!(stdout_lock, " ")?;
        }
        write!(stdout_lock, "{:1$}", result.count.bytes, min_width)?;
        is_first = false;
    }
    if settings.show_chars {
        if !is_first {
            write!(stdout_lock, " ")?;
        }
        write!(stdout_lock, "{:1$}", result.count.chars, min_width)?;
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
        writeln!(stdout_lock, " {}", title)?;
    } else {
        writeln!(stdout_lock)?;
    }

    Ok(())
}
