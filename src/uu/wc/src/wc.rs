// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// cSpell:ignore ilog wc wc's

mod count_fast;
mod countable;
mod utf8;
mod word_count;

use std::{
    borrow::{Borrow, Cow},
    cmp::max,
    ffi::OsString,
    fs::{self, File},
    io::{self, Write},
    iter,
    path::{Path, PathBuf},
};

use clap::{builder::ValueParser, crate_version, Arg, ArgAction, ArgMatches, Command};
use thiserror::Error;
use unicode_width::UnicodeWidthChar;
use utf8::{BufReadDecoder, BufReadDecoderError};

use uucore::{
    error::{FromIo, UError, UResult},
    format_usage, help_about, help_usage,
    quoting_style::{escape_name, QuotingStyle},
    shortcut_value_parser::ShortcutValueParser,
    show,
};

use crate::{
    count_fast::{count_bytes_chars_and_lines_fast, count_bytes_fast},
    countable::WordCountable,
    word_count::WordCount,
};

/// The minimum character width for formatting counts when reading from stdin.
const MINIMUM_WIDTH: usize = 7;

struct Settings<'a> {
    show_bytes: bool,
    show_chars: bool,
    show_lines: bool,
    show_words: bool,
    show_max_line_length: bool,
    files0_from: Option<Input<'a>>,
    total_when: TotalWhen,
}

impl Default for Settings<'_> {
    fn default() -> Self {
        // Defaults if none of -c, -m, -l, -w, nor -L are specified.
        Self {
            show_bytes: true,
            show_chars: false,
            show_lines: true,
            show_words: true,
            show_max_line_length: false,
            files0_from: None,
            total_when: TotalWhen::default(),
        }
    }
}

impl<'a> Settings<'a> {
    fn new(matches: &'a ArgMatches) -> Self {
        let files0_from = matches
            .get_one::<OsString>(options::FILES0_FROM)
            .map(Into::into);

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
            files0_from,
            total_when,
        };

        if settings.number_enabled() > 0 {
            settings
        } else {
            Self {
                files0_from: settings.files0_from,
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
static QS_QUOTE_ESCAPE: &QuotingStyle = &QuotingStyle::Shell {
    escape: true,
    always_quote: true,
    show_control: false,
};

/// Supported inputs.
#[derive(Debug)]
enum Inputs<'a> {
    /// Default Standard input, i.e. no arguments.
    Stdin,
    /// Files; "-" means stdin, possibly multiple times!
    Paths(Vec<Input<'a>>),
    /// --files0-from; "-" means stdin.
    Files0From(Input<'a>),
}

impl<'a> Inputs<'a> {
    fn new(matches: &'a ArgMatches) -> UResult<Self> {
        let arg_files = matches.get_many::<OsString>(ARG_FILES);
        let files0_from = matches.get_one::<OsString>(options::FILES0_FROM);

        match (arg_files, files0_from) {
            (None, None) => Ok(Self::Stdin),
            (Some(files), None) => Ok(Self::Paths(files.map(Into::into).collect())),
            (None, Some(path)) => {
                // If path is a file, and the file isn't too large, we'll load it ahead
                // of time. Every path within the file will have its length checked to
                // hopefully better align the output columns.
                let input = Input::from(path);
                match input.try_as_files0()? {
                    Some(paths) => Ok(Self::Paths(paths)),
                    None => Ok(Self::Files0From(input)),
                }
            }
            (Some(mut files), Some(_)) => {
                Err(WcError::files_disabled(files.next().unwrap()).into())
            }
        }
    }

    // Creates an iterator which yields values borrowed from the command line arguments.
    // Returns an error if the file specified in --files0-from cannot be opened.
    fn try_iter(
        &'a self,
        settings: &'a Settings<'a>,
    ) -> UResult<impl Iterator<Item = InputIterItem<'a>>> {
        let base: Box<dyn Iterator<Item = _>> = match self {
            Self::Stdin => Box::new(iter::once(Ok(Input::Stdin(StdinKind::Implicit)))),
            Self::Paths(inputs) => Box::new(inputs.iter().map(|i| Ok(i.as_borrowed()))),
            Self::Files0From(input) => match input {
                Input::Path(path) => Box::new(files0_iter_file(path)?),
                Input::Stdin(_) => Box::new(files0_iter_stdin()),
            },
        };

        // The 1-based index of each yielded item must be tracked for error reporting.
        let mut with_idx = base.enumerate().map(|(i, v)| (i + 1, v));
        let files0_from_path = settings.files0_from.as_ref().map(Input::as_borrowed);

        let iter = iter::from_fn(move || {
            let (idx, next) = with_idx.next()?;
            match next {
                // filter zero length file names...
                Ok(Input::Path(p)) if p.as_os_str().is_empty() => Some(Err({
                    let maybe_ctx = files0_from_path.as_ref().map(|p| (p, idx));
                    WcError::zero_len(maybe_ctx).into()
                })),
                _ => Some(next),
            }
        });
        Ok(iter)
    }
}

#[derive(Clone, Copy, Debug)]
enum StdinKind {
    /// Specified on command-line with "-" (STDIN_REPR)
    Explicit,
    /// Implied by the lack of any arguments
    Implicit,
}

/// Represents a single input, either to be counted or processed for other files names via
/// --files0-from.
#[derive(Debug)]
enum Input<'a> {
    Path(Cow<'a, Path>),
    Stdin(StdinKind),
}

impl From<PathBuf> for Input<'_> {
    fn from(p: PathBuf) -> Self {
        if p.as_os_str() == STDIN_REPR {
            Self::Stdin(StdinKind::Explicit)
        } else {
            Self::Path(Cow::Owned(p))
        }
    }
}

impl<'a, T: AsRef<Path> + ?Sized> From<&'a T> for Input<'a> {
    fn from(p: &'a T) -> Self {
        let p = p.as_ref();
        if p.as_os_str() == STDIN_REPR {
            Self::Stdin(StdinKind::Explicit)
        } else {
            Self::Path(Cow::Borrowed(p))
        }
    }
}

impl<'a> Input<'a> {
    /// Translates Path(Cow::Owned(_)) to Path(Cow::Borrowed(_)).
    fn as_borrowed(&'a self) -> Self {
        match self {
            Self::Path(p) => Self::Path(Cow::Borrowed(p.borrow())),
            Self::Stdin(k) => Self::Stdin(*k),
        }
    }

    /// Converts input to title that appears in stats.
    fn to_title(&self) -> Option<Cow<str>> {
        match self {
            Self::Path(path) => Some(match path.to_str() {
                Some(s) if !s.contains('\n') => Cow::Borrowed(s),
                _ => Cow::Owned(escape_name(path.as_os_str(), QS_ESCAPE)),
            }),
            Self::Stdin(StdinKind::Explicit) => Some(Cow::Borrowed(STDIN_REPR)),
            Self::Stdin(StdinKind::Implicit) => None,
        }
    }

    /// Converts input into the form that appears in errors.
    fn path_display(&self) -> String {
        match self {
            Self::Path(path) => escape_name(path.as_os_str(), QS_ESCAPE),
            Self::Stdin(_) => String::from("standard input"),
        }
    }

    /// When given --files0-from, we may be given a path or stdin. Either may be a stream or
    /// a regular file. If given a file less than 10 MiB, it will be consumed and turned into
    /// a Vec of Input::Paths which can be scanned to determine the widths of the columns that
    /// will ultimately be printed.
    fn try_as_files0(&self) -> UResult<Option<Vec<Input<'static>>>> {
        match self {
            Self::Path(path) => match fs::metadata(path) {
                Ok(meta) if meta.is_file() && meta.len() <= (10 << 20) => Ok(Some(
                    files0_iter_file(path)?.collect::<Result<Vec<_>, _>>()?,
                )),
                _ => Ok(None),
            },
            Self::Stdin(_) if is_stdin_small_file() => {
                Ok(Some(files0_iter_stdin().collect::<Result<Vec<_>, _>>()?))
            }
            Self::Stdin(_) => Ok(None),
        }
    }
}

#[cfg(unix)]
fn is_stdin_small_file() -> bool {
    use std::os::unix::io::{AsRawFd, FromRawFd};
    // Safety: we'll rely on Rust to give us a valid RawFd for stdin with which we can attempt to
    // open a File, but only for the sake of fetching .metadata().  ManuallyDrop will ensure we
    // don't do anything else to the FD if anything unexpected happens.
    let f = std::mem::ManuallyDrop::new(unsafe { File::from_raw_fd(io::stdin().as_raw_fd()) });
    matches!(f.metadata(), Ok(meta) if meta.is_file() && meta.len() <= (10 << 20))
}

#[cfg(not(unix))]
// Windows presents a piped stdin as a "normal file" with a length equal to however many bytes
// have been buffered at the time it's checked. To be safe, we must never assume it's a file.
fn is_stdin_small_file() -> bool {
    false
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

#[derive(Debug, Error)]
enum WcError {
    #[error("extra operand '{extra}'\nfile operands cannot be combined with --files0-from")]
    FilesDisabled { extra: Cow<'static, str> },
    #[error("when reading file names from stdin, no file name of '-' allowed")]
    StdinReprNotAllowed,
    #[error("invalid zero-length file name")]
    ZeroLengthFileName,
    #[error("{path}:{idx}: invalid zero-length file name")]
    ZeroLengthFileNameCtx { path: Cow<'static, str>, idx: usize },
}

impl WcError {
    fn zero_len(ctx: Option<(&Input, usize)>) -> Self {
        match ctx {
            Some((input, idx)) => {
                let path = match input {
                    Input::Stdin(_) => STDIN_REPR.into(),
                    Input::Path(path) => escape_name(path.as_os_str(), QS_ESCAPE).into(),
                };
                Self::ZeroLengthFileNameCtx { path, idx }
            }
            None => Self::ZeroLengthFileName,
        }
    }
    fn files_disabled(first_extra: &OsString) -> Self {
        let extra = first_extra.to_string_lossy().into_owned().into();
        Self::FilesDisabled { extra }
    }
}

impl UError for WcError {
    fn usage(&self) -> bool {
        matches!(self, Self::FilesDisabled { .. })
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let settings = Settings::new(&matches);
    let inputs = Inputs::new(&matches)?;

    wc(&inputs, &settings)
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .args_override_self(true)
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
                .value_parser(ValueParser::os_string())
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
                .value_parser(ShortcutValueParser::new([
                    "auto", "always", "only", "never",
                ]))
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
        // show_lines, show_words
        (_, false, true, false, true) => {
            word_count_from_reader_specialized::<_, false, true, false, true>(reader)
        }
        // show_lines, show_max_line_length
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

fn process_chunk<
    const SHOW_CHARS: bool,
    const SHOW_LINES: bool,
    const SHOW_MAX_LINE_LENGTH: bool,
    const SHOW_WORDS: bool,
>(
    total: &mut WordCount,
    text: &str,
    current_len: &mut usize,
    in_word: &mut bool,
) {
    for ch in text.chars() {
        if SHOW_WORDS {
            if ch.is_whitespace() {
                *in_word = false;
            } else if !(*in_word) {
                // This also counts control characters! (As of GNU coreutils 9.5)
                *in_word = true;
                total.words += 1;
            }
        }
        if SHOW_MAX_LINE_LENGTH {
            match ch {
                '\n' | '\r' | '\x0c' => {
                    total.max_line_length = max(*current_len, total.max_line_length);
                    *current_len = 0;
                }
                '\t' => {
                    *current_len -= *current_len % 8;
                    *current_len += 8;
                }
                _ => {
                    *current_len += ch.width().unwrap_or(0);
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

    total.max_line_length = max(*current_len, total.max_line_length);
}

fn handle_error(error: BufReadDecoderError<'_>, total: &mut WordCount) -> Option<io::Error> {
    match error {
        BufReadDecoderError::InvalidByteSequence(bytes) => {
            total.bytes += bytes.len();
        }
        BufReadDecoderError::Io(e) => return Some(e),
    }
    None
}

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
                process_chunk::<SHOW_CHARS, SHOW_LINES, SHOW_MAX_LINE_LENGTH, SHOW_WORDS>(
                    &mut total,
                    text,
                    &mut current_len,
                    &mut in_word,
                );
            }
            Err(e) => {
                if let Some(e) = handle_error(e, &mut total) {
                    return (total, Some(e));
                }
            }
        }
    }

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

/// If we fail opening a file, we only show the error. If we fail reading the
/// file, we show a count for what we managed to read.
///
/// Therefore, the reading implementations always return a total and sometimes
/// return an error: (WordCount, Option<io::Error>).
fn word_count_from_input(input: &Input<'_>, settings: &Settings) -> CountResult {
    let (total, maybe_err) = match input {
        Input::Stdin(_) => word_count_from_reader(io::stdin().lock(), settings),
        Input::Path(path) => match File::open(path) {
            Ok(f) => word_count_from_reader(f, settings),
            Err(err) => return CountResult::Failure(err),
        },
    };
    match maybe_err {
        None => CountResult::Success(total),
        Some(err) => CountResult::Interrupted(total, err),
    }
}

/// Compute the number of digits needed to represent all counts in all inputs.
///
/// For [`Inputs::Stdin`], [`MINIMUM_WIDTH`] is returned, unless there is only one counter number
/// to be printed, in which case 1 is returned.
///
/// For [`Inputs::Files0From`], [`MINIMUM_WIDTH`] is returned.
///
/// An [`Inputs::Paths`] may include zero or more "-" entries, each of which represents reading
/// from `stdin`. The presence of any such entry causes this function to return a width that is at
/// least [`MINIMUM_WIDTH`].
///
/// If an [`Inputs::Paths`] contains only one path and only one number needs to be printed then
/// this function is optimized to return 1 without making any calls to get file metadata.
///
/// If file metadata could not be read from any of the [`Input::Path`] input, that input does not
/// affect number width computation.  Otherwise, the file sizes from the files' metadata are summed
/// and the number of digits in that total size is returned.
fn compute_number_width(inputs: &Inputs, settings: &Settings) -> usize {
    match inputs {
        Inputs::Stdin if settings.number_enabled() == 1 => 1,
        Inputs::Stdin => MINIMUM_WIDTH,
        Inputs::Files0From(_) => 1,
        Inputs::Paths(inputs) => {
            if settings.number_enabled() == 1 && inputs.len() == 1 {
                return 1;
            }

            let mut minimum_width = 1;
            let mut total: u64 = 0;
            for input in inputs {
                match input {
                    Input::Stdin(_) => minimum_width = MINIMUM_WIDTH,
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

            if total == 0 {
                minimum_width
            } else {
                let total_width = (1 + total.ilog10())
                    .try_into()
                    .expect("ilog of a u64 should fit into a usize");
                max(total_width, minimum_width)
            }
        }
    }
}

type InputIterItem<'a> = Result<Input<'a>, Box<dyn UError>>;

/// To be used with `--files0-from=-`, this applies a filter on the results of files0_iter to
/// translate '-' into the appropriate error.
fn files0_iter_stdin<'a>() -> impl Iterator<Item = InputIterItem<'a>> {
    files0_iter(io::stdin().lock(), STDIN_REPR.into()).map(|i| match i {
        Ok(Input::Stdin(_)) => Err(WcError::StdinReprNotAllowed.into()),
        _ => i,
    })
}

fn files0_iter_file<'a>(path: &Path) -> UResult<impl Iterator<Item = InputIterItem<'a>>> {
    match File::open(path) {
        Ok(f) => Ok(files0_iter(f, path.into())),
        Err(e) => Err(e.map_err_context(|| {
            format!(
                "cannot open {} for reading",
                escape_name(path.as_os_str(), QS_QUOTE_ESCAPE)
            )
        })),
    }
}

fn files0_iter<'a>(
    r: impl io::Read + 'static,
    err_path: OsString,
) -> impl Iterator<Item = InputIterItem<'a>> {
    use std::io::BufRead;
    let mut i = Some(
        io::BufReader::new(r)
            .split(b'\0')
            .map(move |res| match res {
                Ok(p) if p == STDIN_REPR.as_bytes() => Ok(Input::Stdin(StdinKind::Explicit)),
                Ok(p) => {
                    // On Unix systems, OsStrings are just strings of bytes, not necessarily UTF-8.
                    #[cfg(unix)]
                    {
                        use std::os::unix::ffi::OsStringExt;
                        Ok(Input::Path(PathBuf::from(OsString::from_vec(p)).into()))
                    }

                    // ...Windows does not, we must go through Strings.
                    #[cfg(not(unix))]
                    {
                        let s = String::from_utf8(p)
                            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                        Ok(Input::Path(PathBuf::from(s).into()))
                    }
                }
                Err(e) => Err(e.map_err_context(|| {
                    format!("{}: read error", escape_name(&err_path, QS_ESCAPE))
                }) as Box<dyn UError>),
            }),
    );
    // Loop until there is an error; yield that error and then nothing else.
    std::iter::from_fn(move || {
        let next = i.as_mut().and_then(Iterator::next);
        if matches!(next, Some(Err(_)) | None) {
            i = None;
        }
        next
    })
}

fn wc(inputs: &Inputs, settings: &Settings) -> UResult<()> {
    let mut total_word_count = WordCount::default();
    let mut num_inputs: usize = 0;

    let (number_width, are_stats_visible) = match settings.total_when {
        TotalWhen::Only => (1, false),
        _ => (compute_number_width(inputs, settings), true),
    };

    for maybe_input in inputs.try_iter(settings)? {
        num_inputs += 1;

        let input = match maybe_input {
            Ok(input) => input,
            Err(err) => {
                show!(err);
                continue;
            }
        };

        let word_count = match word_count_from_input(&input, settings) {
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

    if settings.total_when.is_total_row_visible(num_inputs) {
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
