//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Boden Garman <bpgarman@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// cSpell:ignore wc wc's

mod count_fast;
mod countable;
mod utf8;
mod word_count;
use clap::builder::ValueParser;
use count_fast::{count_bytes_chars_and_lines_fast, count_bytes_fast};
use countable::WordCountable;
use unicode_width::UnicodeWidthChar;
use utf8::{BufReadDecoder, BufReadDecoderError};
use uucore::{format_usage, help_about, help_usage, show};
use word_count::WordCount;

use clap::{crate_version, Arg, ArgAction, ArgMatches, Command};

use std::cmp::max;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fmt::Display;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::PathBuf;

use uucore::error::{FromIo, UError, UResult};
use uucore::quoting_style::{escape_name, QuotingStyle};

/// The minimum character width for formatting counts when reading from stdin.
const MINIMUM_WIDTH: usize = 7;

struct Settings {
    show_bytes: bool,
    show_chars: bool,
    show_lines: bool,
    show_words: bool,
    show_max_line_length: bool,
    files0_from_stdin_mode: bool,
    total_when: TotalWhen,
}

impl Default for Settings {
    fn default() -> Self {
        // Defaults if none of -c, -m, -l, -w, nor -L are specified.
        Self {
            show_bytes: true,
            show_chars: false,
            show_lines: true,
            show_words: true,
            show_max_line_length: false,
            files0_from_stdin_mode: false,
            total_when: TotalWhen::default(),
        }
    }
}

impl Settings {
    fn new(matches: &ArgMatches) -> Self {
        let files0_from_stdin_mode = match matches.get_one::<String>(options::FILES0_FROM) {
            Some(files_0_from) => files_0_from == STDIN_REPR,
            None => false,
        };

        let total_when = matches
            .get_one::<String>(options::TOTAL)
            .map(Into::into)
            .unwrap_or_default();

        let settings = Self {
            show_bytes: matches.get_flag(options::BYTES),
            show_chars: matches.get_flag(options::CHAR),
            show_lines: matches.get_flag(options::LINES),
            show_words: matches.get_flag(options::WORDS),
            show_max_line_length: matches.get_flag(options::MAX_LINE_LENGTH),
            files0_from_stdin_mode,
            total_when,
        };

        if settings.number_enabled() > 0 {
            settings
        } else {
            Self {
                files0_from_stdin_mode,
                total_when,
                ..Default::default()
            }
        }
    }

    fn number_enabled(&self) -> u32 {
        [
            self.show_bytes,
            self.show_chars,
            self.show_lines,
            self.show_max_line_length,
            self.show_words,
        ]
        .into_iter()
        .map(Into::<u32>::into)
        .sum()
    }
}

const ABOUT: &str = help_about!("wc.md");
const USAGE: &str = help_usage!("wc.md");

mod options {
    pub static BYTES: &str = "bytes";
    pub static CHAR: &str = "chars";
    pub static FILES0_FROM: &str = "files0-from";
    pub static LINES: &str = "lines";
    pub static MAX_LINE_LENGTH: &str = "max-line-length";
    pub static TOTAL: &str = "total";
    pub static WORDS: &str = "words";
}

static ARG_FILES: &str = "files";
static STDIN_REPR: &str = "-";

static QS_ESCAPE: &QuotingStyle = &QuotingStyle::Shell {
    escape: true,
    always_quote: false,
    show_control: false,
};

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
    fn to_title(&self) -> Option<String> {
        match self {
            Self::Path(path) => Some(escape_name(path.as_os_str(), QS_ESCAPE)),
            Self::Stdin(StdinKind::Explicit) => Some(STDIN_REPR.into()),
            Self::Stdin(StdinKind::Implicit) => None,
        }
    }

    fn path_display(&self) -> String {
        match self {
            Self::Path(path) => escape_name(OsStr::new(&path), QS_ESCAPE),
            Self::Stdin(_) => escape_name(OsStr::new("standard input"), QS_ESCAPE),
        }
    }
}

/// When to show the "total" line
#[derive(Clone, Copy, Default, PartialEq)]
enum TotalWhen {
    #[default]
    Auto,
    Always,
    Only,
    Never,
}

impl<T: AsRef<str>> From<T> for TotalWhen {
    fn from(s: T) -> Self {
        match s.as_ref() {
            "auto" => Self::Auto,
            "always" => Self::Always,
            "only" => Self::Only,
            "never" => Self::Never,
            _ => unreachable!("Should have been caught by clap"),
        }
    }
}

impl TotalWhen {
    fn is_total_row_visible(&self, num_inputs: usize) -> bool {
        match self {
            Self::Auto => num_inputs > 1,
            Self::Always | Self::Only => true,
            Self::Never => false,
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
            Self::FilesDisabled(_) | Self::StdinReprNotAllowed(_) => 1,
        }
    }

    fn usage(&self) -> bool {
        matches!(self, Self::FilesDisabled(_))
    }
}

impl Error for WcError {}

impl Display for WcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FilesDisabled(message) | Self::StdinReprNotAllowed(message) => {
                write!(f, "{message}")
            }
        }
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let inputs = inputs(&matches)?;

    let settings = Settings::new(&matches);

    wc(&inputs, &settings)
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::BYTES)
                .short('c')
                .long(options::BYTES)
                .help("print the byte counts")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CHAR)
                .short('m')
                .long(options::CHAR)
                .help("print the character counts")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILES0_FROM)
                .long(options::FILES0_FROM)
                .value_name("F")
                .help(concat!(
                    "read input from the files specified by\n",
                    "  NUL-terminated names in file F;\n",
                    "  If F is - then read names from standard input"
                ))
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::LINES)
                .short('l')
                .long(options::LINES)
                .help("print the newline counts")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::MAX_LINE_LENGTH)
                .short('L')
                .long(options::MAX_LINE_LENGTH)
                .help("print the length of the longest line")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TOTAL)
                .long(options::TOTAL)
                .value_parser(["auto", "always", "only", "never"])
                .value_name("WHEN")
                .hide_possible_values(true)
                .help(concat!(
                    "when to print a line with total counts;\n",
                    "  WHEN can be: auto, always, only, never"
                )),
        )
        .arg(
            Arg::new(options::WORDS)
                .short('w')
                .long(options::WORDS)
                .help("print the word counts")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(ARG_FILES)
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::FilePath),
        )
}

fn inputs(matches: &ArgMatches) -> UResult<Vec<Input>> {
    match matches.get_many::<OsString>(ARG_FILES) {
        Some(os_values) => {
            if matches.contains_id(options::FILES0_FROM) {
                return Err(WcError::FilesDisabled(
                    "file operands cannot be combined with --files0-from".into(),
                )
                .into());
            }

            Ok(os_values.map(|s| Input::from(s.as_os_str())).collect())
        }
        None => match matches.get_one::<String>(options::FILES0_FROM) {
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
    match (
        settings.show_bytes,
        settings.show_chars,
        settings.show_lines,
        settings.show_max_line_length,
        settings.show_words,
    ) {
        // Specialize scanning loop to improve the performance.
        (false, false, false, false, false) => unreachable!(),

        // show_bytes
        (true, false, false, false, false) => {
            // Fast path when only show_bytes is true.
            let (bytes, error) = count_bytes_fast(&mut reader);
            (
                WordCount {
                    bytes,
                    ..WordCount::default()
                },
                error,
            )
        }

        // Fast paths that can be computed without Unicode decoding.
        // show_lines
        (false, false, true, false, false) => {
            count_bytes_chars_and_lines_fast::<_, false, false, true>(&mut reader)
        }
        // show_chars
        (false, true, false, false, false) => {
            count_bytes_chars_and_lines_fast::<_, false, true, false>(&mut reader)
        }
        // show_chars, show_lines
        (false, true, true, false, false) => {
            count_bytes_chars_and_lines_fast::<_, false, true, true>(&mut reader)
        }
        // show_bytes, show_lines
        (true, false, true, false, false) => {
            count_bytes_chars_and_lines_fast::<_, true, false, true>(&mut reader)
        }
        // show_bytes, show_chars
        (true, true, false, false, false) => {
            count_bytes_chars_and_lines_fast::<_, true, true, false>(&mut reader)
        }
        // show_bytes, show_chars, show_lines
        (true, true, true, false, false) => {
            count_bytes_chars_and_lines_fast::<_, true, true, true>(&mut reader)
        }
        // show_words
        (_, false, false, false, true) => {
            word_count_from_reader_specialized::<_, false, false, false, true>(reader)
        }
        // show_max_line_length
        (_, false, false, true, false) => {
            word_count_from_reader_specialized::<_, false, false, true, false>(reader)
        }
        // show_max_line_length, show_words
        (_, false, false, true, true) => {
            word_count_from_reader_specialized::<_, false, false, true, true>(reader)
        }
        // show_chars, show_words
        (_, false, true, false, true) => {
            word_count_from_reader_specialized::<_, false, true, false, true>(reader)
        }
        // show_chars, show_lines
        (_, false, true, true, false) => {
            word_count_from_reader_specialized::<_, false, true, true, false>(reader)
        }
        // show_lines, show_max_line_length, show_words
        (_, false, true, true, true) => {
            word_count_from_reader_specialized::<_, false, true, true, true>(reader)
        }
        // show_chars, show_words
        (_, true, false, false, true) => {
            word_count_from_reader_specialized::<_, true, false, false, true>(reader)
        }
        // show_chars, show_max_line_length
        (_, true, false, true, false) => {
            word_count_from_reader_specialized::<_, true, false, true, false>(reader)
        }
        // show_chars, show_max_line_length, show_words
        (_, true, false, true, true) => {
            word_count_from_reader_specialized::<_, true, false, true, true>(reader)
        }
        // show_chars, show_lines, show_words
        (_, true, true, false, true) => {
            word_count_from_reader_specialized::<_, true, true, false, true>(reader)
        }
        // show_chars, show_lines, show_max_line_length
        (_, true, true, true, false) => {
            word_count_from_reader_specialized::<_, true, true, true, false>(reader)
        }
        // show_chars, show_lines, show_max_line_length, show_words
        (_, true, true, true, true) => {
            word_count_from_reader_specialized::<_, true, true, true, true>(reader)
        }
    }
}

#[allow(clippy::cognitive_complexity)]
fn word_count_from_reader_specialized<
    T: WordCountable,
    const SHOW_CHARS: bool,
    const SHOW_LINES: bool,
    const SHOW_MAX_LINE_LENGTH: bool,
    const SHOW_WORDS: bool,
>(
    reader: T,
) -> (WordCount, Option<io::Error>) {
    let mut total = WordCount::default();
    let mut reader = BufReadDecoder::new(reader.buffered());
    let mut in_word = false;
    let mut current_len = 0;

    while let Some(chunk) = reader.next_strict() {
        match chunk {
            Ok(text) => {
                for ch in text.chars() {
                    if SHOW_WORDS {
                        if ch.is_whitespace() {
                            in_word = false;
                        } else if ch.is_ascii_control() {
                            // These count as characters but do not affect the word state
                        } else if !in_word {
                            in_word = true;
                            total.words += 1;
                        }
                    }
                    if SHOW_MAX_LINE_LENGTH {
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
                    if SHOW_LINES && ch == '\n' {
                        total.lines += 1;
                    }
                    if SHOW_CHARS {
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
            let count = word_count_from_reader(stdin_lock, settings);
            match count {
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

/// Compute the number of digits needed to represent all counts in all inputs.
///
/// `inputs` may include zero or more [`Input::Stdin`] entries, each of
/// which represents reading from `stdin`. The presence of any such
/// entry causes this function to return a width that is at least
/// [`MINIMUM_WIDTH`].
///
/// If `input` is empty, or if only one number needs to be printed (for just
/// one file) then this function is optimized to return 1 without making any
/// calls to get file metadata.
///
/// If file metadata could not be read from any of the [`Input::Path`] input,
/// that input does not affect number width computation
///
/// Otherwise, the file sizes in the file metadata are summed and the number of
/// digits in that total size is returned as the number width
///
/// To mirror GNU wc's behavior a special case is added. If --files0-from is
/// used and input is read from stdin and there is only one calculation enabled
/// columns will not be aligned. This is not exactly GNU wc's behavior, but it
/// is close enough to pass the GNU test suite.
fn compute_number_width(inputs: &[Input], settings: &Settings) -> usize {
    if inputs.is_empty()
        || (inputs.len() == 1 && settings.number_enabled() == 1)
        || (settings.files0_from_stdin_mode && settings.number_enabled() == 1)
    {
        return 1;
    }

    let mut minimum_width = 1;
    let mut total = 0;

    for input in inputs {
        match input {
            Input::Stdin(_) => {
                minimum_width = MINIMUM_WIDTH;
            }
            Input::Path(path) => {
                if let Ok(meta) = fs::metadata(path) {
                    if meta.is_file() {
                        total += meta.len();
                    } else {
                        minimum_width = MINIMUM_WIDTH;
                    }
                }
            }
        }
    }

    max(minimum_width, total.to_string().len())
}

fn wc(inputs: &[Input], settings: &Settings) -> UResult<()> {
    let mut total_word_count = WordCount::default();

    let (number_width, are_stats_visible) = match settings.total_when {
        TotalWhen::Only => (1, false),
        _ => (compute_number_width(inputs, settings), true),
    };
    let is_total_row_visible = settings.total_when.is_total_row_visible(inputs.len());

    for input in inputs {
        let word_count = match word_count_from_input(input, settings) {
            CountResult::Success(word_count) => word_count,
            CountResult::Interrupted(word_count, err) => {
                show!(err.map_err_context(|| input.path_display()));
                word_count
            }
            CountResult::Failure(err) => {
                show!(err.map_err_context(|| input.path_display()));
                continue;
            }
        };
        total_word_count += word_count;
        if are_stats_visible {
            let maybe_title = input.to_title();
            let maybe_title_str = maybe_title.as_deref();
            if let Err(err) = print_stats(settings, &word_count, maybe_title_str, number_width) {
                let title = maybe_title_str.unwrap_or("<stdin>");
                show!(err.map_err_context(|| format!("failed to print result for {title}")));
            }
        }
    }

    if is_total_row_visible {
        let title = are_stats_visible.then_some("total");
        if let Err(err) = print_stats(settings, &total_word_count, title, number_width) {
            show!(err.map_err_context(|| "failed to print total".into()));
        }
    }

    // Although this appears to be returning `Ok`, the exit code may have been set to a non-zero
    // value by a call to `record_error!()` above.
    Ok(())
}

fn print_stats(
    settings: &Settings,
    result: &WordCount,
    title: Option<&str>,
    number_width: usize,
) -> io::Result<()> {
    let mut stdout = io::stdout().lock();

    let maybe_cols = [
        (settings.show_lines, result.lines),
        (settings.show_words, result.words),
        (settings.show_chars, result.chars),
        (settings.show_bytes, result.bytes),
        (settings.show_max_line_length, result.max_line_length),
    ];

    let mut space = "";
    for (_, num) in maybe_cols.iter().filter(|(show, _)| *show) {
        write!(stdout, "{space}{num:number_width$}")?;
        space = " ";
    }

    if let Some(title) = title {
        writeln!(stdout, "{space}{title}")
    } else {
        writeln!(stdout)
    }
}
