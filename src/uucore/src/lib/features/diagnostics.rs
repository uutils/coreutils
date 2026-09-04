// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Render an error against the argument list it came from.
//!
//! Utilities whose arguments *are* the expression they evaluate — `test`, `expr`
//! — can only say so much in a single line of stderr. This module echoes the
//! arguments back as a source line and points a caret at the one that is at
//! fault.
//!
//! A utility is expected to map its own error type to an argument index,
//! optionally a label, and optionally a line of advice; everything user-facing
//! is passed in already localized, so this module holds no messages of its
//! own. A label should add something the message does not say — an
//! expectation, or a fix — never restate it: with no label the span is drawn
//! as a bare underline, and the message and advice carry the rest.
//!
//! Rendering only happens when stderr is a terminal, so anything reading our
//! output — a script, a pipe, a test suite — still sees the plain one-line
//! message it always did. `UUTILS_DIAG=always` or `never` overrides that
//! check.
//!
//! ```text
//! tr: range-endpoints of 'y-b' are in reverse collating sequence order
//!    ╭─[ tr:1:7 ]
//!    │
//!  1 │ tr qw[y-b] x
//!    │       ─┬─
//!    │        ╰─── did you mean 'b-y'?
//!    │
//!    │ Help: a range goes from the lower character to the higher one, as in a-z
//! ───╯
//! ```

// spell-checker:ignore étage replacen

use std::borrow::Cow;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt::Write as _;
use std::io::IsTerminal;
use std::ops::Range;
use std::sync::OnceLock;

use ariadne::{CharSet, Color, Config, IndexType, Label, Report, ReportKind, Source};

use crate::display::Quotable;
use crate::translate;

/// The variable that overrides the terminal check.
const MODE_VAR: &str = "UUTILS_DIAG";

/// When to render an error against its argument list.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    /// Let stderr decide.
    Auto,
    Always,
    Never,
}

impl Mode {
    /// What [`MODE_VAR`] asks for.
    ///
    /// Anything but `always` and `never` is [`Mode::Auto`] rather than an
    /// error — this gets exported from shell profiles, and no spelling of it
    /// should make a utility fail.
    fn from_env(value: Option<&OsStr>) -> Self {
        let Some(value) = value.and_then(OsStr::to_str) else {
            return Self::Auto;
        };
        if value.eq_ignore_ascii_case("always") {
            Self::Always
        } else if value.eq_ignore_ascii_case("never") {
            Self::Never
        } else {
            Self::Auto
        }
    }
}

/// Whether errors should be rendered against their argument list.
///
/// Callers should check this before doing any work that only a diagnostic needs,
/// so that the default path costs nothing.
///
/// # Returns
///
/// What `UUTILS_DIAG` asks for, when it asks. Otherwise `true` when stderr is
/// a terminal — a person is watching, and gets the rich form — and `false` in
/// a script or a pipe, where whatever reads stderr gets the plain message it
/// can grep for.
pub fn enabled() -> bool {
    // Read once: this runs before the argument capture of every caret.
    static MODE: OnceLock<Mode> = OnceLock::new();
    match MODE.get_or_init(|| Mode::from_env(env::var_os(MODE_VAR).as_deref())) {
        Mode::Always => true,
        Mode::Never => false,
        Mode::Auto => std::io::stderr().is_terminal(),
    }
}

/// Keep the arguments a diagnostic would point at, as they were typed.
///
/// Parsing moves, rewrites or consumes the argument list, so a utility that
/// may want a caret has to put a copy aside before that happens. This is the
/// one place that decides whether the copy is worth making.
///
/// # Arguments
///
/// * `args` - The arguments to keep, exactly as the caret should echo them.
///
/// # Returns
///
/// `None` when diagnostics are off, so that the copy is only paid for when
/// something is going to be rendered.
pub fn capture(args: &[OsString]) -> Option<Vec<OsString>> {
    enabled().then(|| args.to_vec())
}

/// Keep the arguments a diagnostic would point at, minus the program name.
///
/// # Arguments
///
/// * `args` - The whole argument list, `argv[0]` included. The program name is
///   dropped since it is never part of what a caret points at.
///
/// # Returns
///
/// As [`capture`], `None` when diagnostics are off.
pub fn operands(args: &[OsString]) -> Option<Vec<OsString>> {
    capture(args.get(1..).unwrap_or_default())
}

pub use crate::features::diagnostics_boundary::{
    OptionValue, ValueOptions, char_span, floor_boundary, list_items,
};

/// The error to raise for something a caret may have just explained.
///
/// Draws the report when the arguments as typed were kept, and quiets `error`
/// when it did: the report has already said everything the one-line message
/// would, and the exit code is all that is left to carry. Every caret
/// diagnostic ends this way, so it is written once here.
///
/// # Arguments
///
/// * `diag_args` - The arguments as typed, program name included, or `None`
///   when they were not kept — as [`capture`] returns them.
/// * `error` - The error to raise when nothing was drawn. It is lent to `draw`
///   rather than moved into it, since it is usually the error the report is
///   about as well.
/// * `draw` - Draws the report against the arguments, and returns `false` when
///   it could not — because the error is not about any one of them, or because
///   none of them turned out to carry what the caret would point at.
pub fn error_after_report<E: Into<Box<dyn crate::error::UError>>>(
    diag_args: Option<&[OsString]>,
    error: E,
    draw: impl FnOnce(&[OsString], &E) -> bool,
) -> Box<dyn crate::error::UError> {
    let reported = diag_args.is_some_and(|args| draw(args, &error));
    crate::error::quiet_if_reported(reported, error)
}

/// An argument list rendered as a single line, with the position of every
/// argument inside it.
pub struct Snapshot {
    /// The arguments joined by spaces, quoted where needed.
    text: String,
    /// Byte range of each argument inside `text`.
    spans: Vec<Range<usize>>,
    /// The arguments themselves, for [`Snapshot::index_of`].
    args: Vec<OsString>,
    /// Index of the first operand: 1 when argument 0 is the program name.
    first_operand: usize,
}

impl Snapshot {
    /// Build a snapshot of an argument list.
    ///
    /// # Arguments
    ///
    /// * `args` - The arguments as passed to the utility, without the utility
    ///   name itself.
    pub fn new<S: AsRef<OsStr>>(args: &[S]) -> Self {
        let mut snapshot = Self::with_capacity(args.len());
        for arg in args {
            snapshot.push(arg.as_ref().to_os_string());
        }
        snapshot
    }

    /// Build a snapshot of an argument list whose first argument is the
    /// program name.
    ///
    /// The program name is shown in the source line — the command reads as it
    /// was typed — but it is never matched by any locator, so an operand that
    /// happens to share its text cannot be mistaken for it.
    ///
    /// # Arguments
    ///
    /// * `args` - The whole argument list, `argv[0]` included.
    pub fn with_program<S: AsRef<OsStr>>(args: &[S]) -> Self {
        let mut snapshot = Self::new(args);
        snapshot.first_operand = args.len().min(1);
        snapshot
    }

    /// Build a snapshot of an argument list held as raw bytes.
    ///
    /// # Arguments
    ///
    /// * `args` - The arguments as raw bytes, as produced by
    ///   [`crate::os_string_to_vec`].
    pub fn from_bytes<S: AsRef<[u8]>>(args: &[S]) -> Self {
        let mut snapshot = Self::with_capacity(args.len());
        for arg in args {
            snapshot.push(match crate::os_str_from_bytes(arg.as_ref()) {
                Ok(arg) => arg.into_owned(),
                // Only reachable on platforms where `OsStr` is not raw bytes;
                // show the argument lossily rather than not at all.
                Err(_) => String::from_utf8_lossy(arg.as_ref()).into_owned().into(),
            });
        }
        snapshot
    }

    fn with_capacity(len: usize) -> Self {
        Self {
            text: String::new(),
            spans: Vec::with_capacity(len),
            args: Vec::with_capacity(len),
            first_operand: 0,
        }
    }

    fn push(&mut self, arg: OsString) {
        if !self.spans.is_empty() {
            self.text.push(' ');
        }
        let start = self.text.len();
        // Offsets are taken from the rendered text rather than from the
        // argument itself: quoting changes the length (a non-UTF-8 argument is
        // shown as `$'fo\x80o'`) and the caret has to line up with what is
        // actually printed.
        match arg.to_str() {
            // Operators such as `=` or `!=` are shell-special, but quoting them
            // here would only obscure the expression.
            Some(s) if !s.is_empty() && !s.chars().any(char::is_whitespace) => {
                self.text.push_str(s);
            }
            _ => {
                let _ = write!(self.text, "{}", arg.maybe_quote());
            }
        }
        self.spans.push(start..self.text.len());
        self.args.push(arg);
    }

    /// Whether there is anything to point at.
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    /// Index of the first argument equal to `arg`.
    ///
    /// Useful for errors raised after parsing, which know the value at fault but
    /// no longer know where it came from.
    ///
    /// # Arguments
    ///
    /// * `arg` - The argument to look for.
    ///
    /// # Returns
    ///
    /// The index of the first operand equal to `arg`, or `None` if there is
    /// none. A repeated value resolves to its first occurrence, and the
    /// program name is never matched.
    pub fn index_of(&self, arg: &OsStr) -> Option<usize> {
        self.args
            .iter()
            .enumerate()
            .skip(self.first_operand)
            .find(|(_, candidate)| *candidate == arg)
            .map(|(index, _)| index)
    }

    /// Index of the argument carrying `operand` as the value of an option.
    ///
    /// An option's value can be spelled many ways — `-k 2.3q`, `-k2.3q`,
    /// `-rk2.3q`, `--key 2.3q`, `--key=2.3q` — and a text search cannot tell
    /// the value apart from a file or another option that happens to end with
    /// the same characters. This walks the arguments and matches only the
    /// shapes an option value can actually take.
    ///
    /// # Arguments
    ///
    /// * `operand` - The value at fault, as the parser received it.
    /// * `short` - The option's short name (`'k'` for `-k`), if it has one.
    /// * `long` - The option's long name (`"key"` for `--key`), if it has one.
    ///
    /// # Returns
    ///
    /// The index of the first argument carrying `operand` — the argument
    /// itself for a detached value, the combined argument for a glued one —
    /// or `None` if no argument does. First match is the right one for
    /// repeatable options, since parsing stops at the first failing value.
    pub fn index_of_value(
        &self,
        operand: &str,
        short: Option<char>,
        long: Option<&str>,
    ) -> Option<usize> {
        // A run of short options the wanted one ends, as in `-r…k`.
        let ends_cluster = |arg: &str| {
            let Some(short) = short else { return false };
            arg.strip_prefix('-').is_some_and(|cluster| {
                cluster.ends_with(short) && cluster.chars().all(char::is_alphanumeric)
            })
        };
        // `-k` or `--key`, or a cluster of short options ending in `-…k`.
        let is_flag = |arg: &str| {
            if let Some(rest) = arg.strip_prefix("--") {
                return long == Some(rest);
            }
            ends_cluster(arg)
        };
        // `--key=<operand>`.
        let is_attached_long = |arg: &str| {
            let Some(long) = long else { return false };
            arg.strip_prefix("--")
                .and_then(|rest| rest.strip_prefix(long))
                .and_then(|rest| rest.strip_prefix('='))
                == Some(operand)
        };
        // `-k<operand>`, or glued to a cluster as `-r…k<operand>`. An empty
        // value cannot be glued: `-k` alone is the flag, not the flag carrying
        // nothing, and taking it for the value would point the caret at the
        // option instead of the empty argument that follows it.
        let is_attached_short = |arg: &str| {
            if operand.is_empty() {
                return false;
            }
            let Some(prefix) = arg
                .len()
                .checked_sub(operand.len())
                .and_then(|end| arg.get(..end))
            else {
                return false;
            };
            arg.ends_with(operand) && ends_cluster(prefix)
        };

        let args = self.args.iter().map(|arg| arg.to_str()).enumerate();
        let mut previous: Option<&str> = None;
        for (index, arg) in args.skip(self.first_operand) {
            let Some(arg) = arg else {
                previous = None;
                continue;
            };
            // Everything after a bare `--` is positional, never an option or
            // its value.
            if arg == "--" {
                break;
            }
            if is_attached_long(arg)
                || is_attached_short(arg)
                || (arg == operand && previous.is_some_and(is_flag))
            {
                return Some(index);
            }
            previous = Some(arg);
        }
        None
    }

    /// Index of the `n`-th positional argument, counting from zero.
    ///
    /// An argument is taken as positional when it follows a bare `--` or does
    /// not start with `-`; a lone `-` counts as positional. This only holds
    /// for utilities none of whose options take a *separate* value — an
    /// option's detached value would be miscounted as a positional.
    ///
    /// # Arguments
    ///
    /// * `n` - Zero-based rank among the positional arguments.
    ///
    /// # Returns
    ///
    /// The index of that positional, or `None` if there are not that many.
    pub fn index_of_positional(&self, n: usize) -> Option<usize> {
        self.index_of_operand(n, &ValueOptions::NONE)
    }

    /// Index of the `n`-th operand, stepping over the values of `options`.
    ///
    /// As [`Snapshot::index_of_positional`], but for a utility whose options
    /// take a separate value: `csplit -n 3 file 5` has two operands, not
    /// three, and naming `-n` is what keeps the caret off the `3`.
    ///
    /// # Arguments
    ///
    /// * `n` - Zero-based rank among the operands.
    /// * `options` - The options whose value is a separate argument.
    ///
    /// # Returns
    ///
    /// The index of that operand, or `None` if there are not that many.
    pub fn index_of_operand(&self, n: usize, options: &ValueOptions) -> Option<usize> {
        let mut options_ended = false;
        let mut skip_value = false;
        let mut rank = 0;

        for index in self.first_operand..self.args.len() {
            let arg = &self.args[index];
            if skip_value {
                skip_value = false;
                continue;
            }
            if !options_ended {
                let bytes = arg.as_encoded_bytes();
                if bytes == b"--" {
                    options_ended = true;
                    continue;
                }
                // A lone `-` is an operand, and so is an argument that is not
                // UTF-8, which no option spelling can be.
                if let Some(text) = arg.to_str()
                    && text.starts_with('-')
                    && text != "-"
                {
                    skip_value = options.takes_next(text);
                    continue;
                }
            }
            if rank == n {
                return Some(index);
            }
            rank += 1;
        }
        None
    }

    /// Byte range of the argument at `index`.
    ///
    /// An index past the end means "end of input" and is clamped to the last
    /// argument, so the caret never falls outside the text.
    fn span_at(&self, index: usize) -> Option<Range<usize>> {
        self.spans.get(index).or_else(|| self.spans.last()).cloned()
    }

    /// Write a report for the argument at `index` to stderr.
    ///
    /// # Arguments
    ///
    /// * `index` - Position of the argument at fault. An index past the end
    ///   points at the last argument.
    /// * `message` - The error message, already localized.
    /// * `label` - Text placed under the caret, already localized, or `None`
    ///   for a bare underline. A label should add something the message does
    ///   not say — an expectation, or a fix — never restate it.
    /// * `help` - An optional line of advice, already localized.
    ///
    /// # Returns
    ///
    /// `false` if nothing could be rendered, in which case the caller should
    /// fall back to a plain one-line message.
    pub fn render(
        &self,
        index: usize,
        message: &str,
        label: Option<&str>,
        help: Option<&str>,
    ) -> bool {
        let Some(span) = self.span_at(index) else {
            return false;
        };
        self.report(span, message, label, help)
    }

    /// Write a report pointing at `range`, a byte range *inside* `operand`,
    /// where `operand` is the tail (or the whole) of the argument at `index`.
    ///
    /// For utilities whose operands are small languages of their own — a `sort`
    /// key, a `chmod` mode — the argument as a whole is rarely the answer; the
    /// caret belongs under the one character that broke the parse. The argument
    /// is named rather than searched for: the caller has located it with
    /// [`Snapshot::index_of`], [`Snapshot::index_of_value`] or
    /// [`Snapshot::index_of_positional`], or tracked it itself, so an unrelated
    /// argument sharing the operand's text cannot draw the caret away. The
    /// operand may sit at the tail of a larger argument, so a key passed as
    /// `-k2.3x` and one passed as `-k 2.3x` both point at the same place.
    ///
    /// Falls back to underlining the whole argument when the operand is not
    /// its tail, or when quoting means an offset inside `operand` no longer
    /// lines up with what is printed.
    ///
    /// # Arguments
    ///
    /// * `index` - Position of the argument carrying `operand`.
    /// * `operand` - The operand at fault, as it was parsed.
    /// * `range` - Byte range inside `operand` to point at. An empty range
    ///   marks the character it starts at.
    /// * `message` - The error message, already localized.
    /// * `label` - Text placed under the caret, already localized, or `None`
    ///   for a bare underline. A label should add something the message does
    ///   not say — an expectation, or a fix — never restate it.
    /// * `help` - An optional line of advice, already localized.
    ///
    /// # Returns
    ///
    /// `false` if nothing could be rendered, in which case the caller should
    /// fall back to a plain one-line message.
    pub fn render_inside_at(
        &self,
        index: usize,
        operand: &str,
        range: Range<usize>,
        message: &str,
        label: Option<&str>,
        help: Option<&str>,
    ) -> bool {
        let Some(span) = self.locate_at(index, operand, range) else {
            return false;
        };
        self.report(span, message, label, help)
    }

    /// Write a report pointing at `range` inside `operand`, where `operand` is
    /// the value of an option.
    ///
    /// The pairing of [`Snapshot::index_of_value`] with
    /// [`Snapshot::render_inside_at`] is what almost every option value needs —
    /// a `chmod` mode, a `sort` key, a `cut` range, a `head` size — so it is
    /// written once here rather than in each utility.
    ///
    /// # Arguments
    ///
    /// * `operand` - The value at fault, as the parser received it.
    /// * `short` - The option's short name (`'k'` for `-k`), if it has one.
    /// * `long` - The option's long name (`"key"` for `--key`), if it has one.
    /// * `range` - Byte range inside `operand` to point at. An empty range
    ///   marks the character it starts at.
    /// * `message` - The error message, already localized.
    /// * `label` - Text placed under the caret, already localized, or `None`
    ///   for a bare underline.
    /// * `help` - An optional line of advice, already localized.
    ///
    /// # Returns
    ///
    /// `false` when no argument carries `operand` as that option's value, or
    /// when nothing could be rendered, in which case the caller should fall
    /// back to the plain one-line message.
    // One more parameter than clippy likes, but every one of them is already
    // part of `render_inside_at`; folding them into a struct would only move
    // the list to the call site.
    #[allow(clippy::too_many_arguments)]
    pub fn render_option_value(
        &self,
        operand: &str,
        short: Option<char>,
        long: Option<&str>,
        range: Range<usize>,
        message: &str,
        label: Option<&str>,
        help: Option<&str>,
    ) -> bool {
        let Some(index) = self.index_of_value(operand, short, long) else {
            return false;
        };
        self.render_inside_at(index, operand, range, message, label, help)
    }

    /// Write a report pointing at `range` inside the value of an option.
    ///
    /// As [`Snapshot::render_option_value`], for a value that travels with the
    /// option it was given to.
    ///
    /// # Arguments
    ///
    /// * `option` - The value at fault and the option it came from.
    /// * `range` - Byte range inside the value to point at. An empty range
    ///   marks the character it starts at.
    /// * `message` - The error message, already localized.
    /// * `label` - Text placed under the caret, already localized, or `None`
    ///   for a bare underline.
    /// * `help` - An optional line of advice, already localized.
    pub fn render_option(
        &self,
        option: &OptionValue,
        range: Range<usize>,
        message: &str,
        label: Option<&str>,
        help: Option<&str>,
    ) -> bool {
        self.render_option_value(
            &option.value,
            option.short,
            option.long,
            range,
            message,
            label,
            help,
        )
    }

    /// Byte range covered by `range` — an offset inside `operand` — within the
    /// argument at `index`.
    fn locate_at(&self, index: usize, operand: &str, range: Range<usize>) -> Option<Range<usize>> {
        let arg = self.args.get(index)?;
        let whole = self.spans[index].clone();
        // An empty operand has nothing of its own to point at, and would be
        // found at the end of the argument rather than where it was written.
        // The argument as echoed — a bare pair of quotes — is what was typed.
        if operand.is_empty() || !arg.as_encoded_bytes().ends_with(operand.as_bytes()) {
            return Some(whole);
        }
        // Where the operand's own text sits in what was printed for this
        // argument. An argument holding a space is echoed quoted, so the
        // operand no longer ends the span it is drawn in — but the quoting
        // only wraps it, and an offset inside it still holds wherever its
        // bytes turned up. An argument that had to be escaped to be printed
        // — a non-UTF-8 one, or one quoting cannot wrap — does not contain
        // them contiguously, and falls back to a plain underline.
        let Some(offset) = self.text[whole.clone()].rfind(operand) else {
            return Some(whole);
        };
        Some(self.locate_operand(whole.start + offset, operand, range))
    }

    /// Byte range covered by `range` — an offset inside `operand` — where
    /// `operand` is drawn starting at `base`.
    fn locate_operand(&self, base: usize, operand: &str, range: Range<usize>) -> Range<usize> {
        let whole = base..base + operand.len();
        // Offsets come from someone else's parser, so they are only trusted as
        // far as the text agrees with them.
        let start = self.floor_boundary(base + range.start.min(operand.len()));
        let end = self.floor_boundary(base + range.end.clamp(range.start, operand.len()));
        if start < end {
            return start..end;
        }
        // An empty range means something is missing rather than wrong; give the
        // caret the character it stopped at, or the whole operand if it ran
        // out.
        let stopped = char_span(&self.text, start);
        if start < whole.end && !stopped.is_empty() {
            return stopped;
        }
        whole
    }

    /// `offset`, moved back to the nearest character boundary.
    fn floor_boundary(&self, offset: usize) -> usize {
        floor_boundary(&self.text, offset)
    }

    /// Translate the word ariadne heads the advice line with.
    ///
    /// The advice itself arrives already localized, but ariadne writes its own
    /// `Help` in front of it, so a translated report would end on a line half
    /// in English. The word is replaced where ariadne wrote it — on the first
    /// row below the label, which is the only place it can be — rather than
    /// everywhere, since an argument the report echoes is free to contain it
    /// too.
    ///
    /// # Arguments
    ///
    /// * `body` - The rendered report, its headline already dropped.
    /// * `has_help` - Whether an advice line was asked for at all.
    fn localize_help(body: &str, has_help: bool) -> Cow<'_, str> {
        let localized = translate!("diagnostics-help-label");
        if !has_help || localized == "Help" {
            return Cow::Borrowed(body);
        }
        let mut lines: Vec<Cow<'_, str>> = body.lines().map(Cow::Borrowed).collect();
        // Rows 0..=2 are the header, the gutter and the source line; rows 3
        // and 4 belong to the label.
        let Some(row) = lines
            .iter()
            .skip(5)
            .position(|line| line.contains("Help"))
            .map(|row| row + 5)
        else {
            // The report is not shaped the way we assumed; hand it back
            // untouched rather than rebuilding it around a row we never found.
            return Cow::Borrowed(body);
        };
        lines[row] = Cow::Owned(lines[row].replacen("Help", &localized, 1));
        Cow::Owned(lines.join("\n") + "\n")
    }

    /// Whether `row` is one of the two rows a single label occupies.
    ///
    /// A report with one label on one source line always has the same shape,
    /// once its `Error:` headline has been dropped: the file header, a blank
    /// gutter, the source line, then the underline and the arm under it.
    fn is_label_row(row: usize) -> bool {
        /// Row holding the arguments as they were typed.
        const SOURCE: usize = 2;
        (SOURCE + 1..=SOURCE + 2).contains(&row)
    }

    /// The report ariadne draws for `span`, with its headline dropped.
    ///
    /// Both [`Self::localize_help`] and [`Self::is_label_row`] count on the
    /// rows of what this returns, so it is a step of its own: a test can render
    /// a known report and check the shape those two assume, rather than only
    /// the finished output.
    ///
    /// # Returns
    ///
    /// `None` when ariadne could not render, in which case the caller falls
    /// back to the plain one-line message.
    fn render_body(
        &self,
        span: Range<usize>,
        label: Option<&str>,
        help: Option<&str>,
        color: bool,
    ) -> Option<String> {
        let id = crate::util_name();
        let config = Config::default()
            // ariadne counts characters unless told otherwise, which would drift
            // on multi-byte arguments.
            .with_index_type(IndexType::Byte)
            .with_color(color)
            .with_char_set(CharSet::Unicode);

        // An empty message keeps the underline and the caret; ariadne draws
        // nothing at all for a label without a message.
        let caret = Label::new((id, span.clone()))
            .with_color(Color::Red)
            .with_message(label.unwrap_or_default());
        let mut report = Report::build(ReportKind::Error, (id, span))
            .with_config(config)
            .with_label(caret);

        if let Some(help) = help {
            report = report.with_help(help);
        }

        let mut rendered = Vec::new();
        if report
            .finish()
            .write((id, Source::from(self.text.as_str())), &mut rendered)
            .is_err()
        {
            return None;
        }

        // ariadne heads every report with its own line — a hardcoded, untranslated
        // "Error:". Drop it and write the utility name instead, so the first line
        // reads exactly like the plain one-line form.
        let rendered = String::from_utf8_lossy(&rendered);
        Some(
            rendered
                .split_once('\n')
                .map_or("", |(_, rest)| rest)
                .into(),
        )
    }

    fn report(
        &self,
        span: Range<usize>,
        message: &str,
        label: Option<&str>,
        help: Option<&str>,
    ) -> bool {
        let id = crate::util_name();
        let color = env::var_os("NO_COLOR").is_none() && std::io::stderr().is_terminal();
        let Some(body) = self.render_body(span, label, help, color) else {
            return false;
        };
        let body = Self::localize_help(&body, help.is_some());
        let body = body.as_ref();
        if label.is_some() {
            eprint!("{id}: {message}\n{body}");
        } else {
            // The empty message left an arrow arm pointing at nothing: drop it
            // and flatten the tee, so only the underline remains. Only the two
            // rows ariadne drew for the label are rewritten — the row above
            // them echoes the arguments, and an argument is free to contain a
            // `╰` or a `┬` of its own.
            let body = body
                .lines()
                .enumerate()
                .filter(|&(row, line)| !(Self::is_label_row(row) && line.contains('╰')))
                .map(|(row, line)| {
                    if Self::is_label_row(row) {
                        Cow::Owned(line.replace('┬', "─"))
                    } else {
                        Cow::Borrowed(line)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            eprintln!("{id}: {message}\n{body}");
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(args: &[&str]) -> Snapshot {
        Snapshot::new(args)
    }

    /// Everything but `always` and `never` leaves the terminal check in
    /// charge, rather than failing.
    #[test]
    fn the_mode_variable_decides_only_when_it_is_spelled_out() {
        let mode = |value| Mode::from_env(Some(OsStr::new(value)));

        assert_eq!(mode("always"), Mode::Always);
        assert_eq!(mode("ALWAYS"), Mode::Always);
        assert_eq!(mode("never"), Mode::Never);
        assert_eq!(mode("Never"), Mode::Never);

        for undecided in ["auto", "", "yes", "1", "sometimes"] {
            assert_eq!(mode(undecided), Mode::Auto, "{undecided:?}");
        }
        assert_eq!(Mode::from_env(None), Mode::Auto);
    }

    /// `localize_help` and `is_label_row` both address ariadne's output by row
    /// number, which only they know is right. Render a report whose shape is
    /// known and check it, so that an ariadne that adds or drops a row fails
    /// here rather than silently deleting a row of somebody's command line.
    #[test]
    fn the_rendered_report_has_the_row_shape_the_rewriting_assumes() {
        let snap = snapshot(&["-k", "1,0"]);
        let body = snap
            .render_body(3..6, Some("a label"), Some("some advice"), false)
            .unwrap();
        let rows: Vec<&str> = body.lines().collect();

        // Row 2 echoes the arguments; rewriting it would edit what was typed.
        assert!(rows[2].contains("-k 1,0"), "row 2: {:?}", rows[2]);
        assert!(!Snapshot::is_label_row(2));

        // Rows 3 and 4 are the underline and the arm carrying the label.
        assert!(Snapshot::is_label_row(3) && Snapshot::is_label_row(4));
        assert!(rows[3].contains('┬'), "row 3: {:?}", rows[3]);
        assert!(
            rows[4].contains('╰') && rows[4].contains("a label"),
            "row 4: {:?}",
            rows[4]
        );

        // The advice comes below those, where `localize_help` looks for it.
        assert!(
            rows.iter().skip(5).any(|row| row.contains("Help")),
            "no help row below row 4: {rows:?}"
        );
    }

    #[test]
    fn spans_cover_each_argument() {
        let snap = snapshot(&["a", "=", ""]);
        assert_eq!(snap.text, "a = ''");
        assert_eq!(snap.spans, vec![0..1, 2..3, 4..6]);
    }

    #[test]
    fn empty_argument_list_has_nothing_to_point_at() {
        let snap = snapshot(&[]);
        assert!(snap.is_empty());
        assert_eq!(snap.text, "");
        assert_eq!(snap.span_at(0), None);
    }

    #[test]
    fn spans_are_byte_ranges_for_multibyte_arguments() {
        let snap = snapshot(&["étage", "!=", "x"]);
        let spans = snap.spans.clone();
        assert_eq!(&snap.text[spans[0].clone()], "étage");
        assert_eq!(&snap.text[spans[1].clone()], "!=");
        assert_eq!(&snap.text[spans[2].clone()], "x");
    }

    #[test]
    fn arguments_with_whitespace_are_quoted() {
        let snap = snapshot(&["two words", "-lt", "5"]);
        assert_eq!(snap.text, "'two words' -lt 5");
        assert_eq!(&snap.text[snap.span_at(0).unwrap()], "'two words'");
    }

    #[cfg(unix)]
    #[test]
    fn spans_follow_the_quoted_form_of_invalid_utf8() {
        let snap = Snapshot::from_bytes(&[&b"qu\x91x"[..], b"-eq", b"7"]);
        let span = snap.span_at(0).unwrap();
        // The caret covers the escaped rendering, not the four raw bytes.
        assert_eq!(&snap.text[span.clone()], r"$'qu\x91x'");
        assert!(span.len() > 4);
    }

    #[test]
    fn byte_arguments_round_trip() {
        let snap = Snapshot::from_bytes(&[b"9", b"+", b"4"]);
        assert_eq!(snap.text, "9 + 4");
        assert_eq!(snap.index_of(OsStr::new("+")), Some(1));
        assert_eq!(snap.index_of(OsStr::new("4")), Some(2));
    }

    #[cfg(unix)]
    #[test]
    fn invalid_utf8_arguments_can_still_be_found() {
        use std::os::unix::ffi::OsStrExt;

        let snap = Snapshot::from_bytes(&[&b"ba\x80d"[..], b"+", b"1"]);
        // A lossy conversion would not compare equal to the raw argument.
        assert_eq!(snap.index_of(OsStr::from_bytes(b"ba\x80d")), Some(0));
    }

    #[test]
    fn index_past_the_end_clamps_to_the_last_argument() {
        let snap = snapshot(&["7", "-lt"]);
        assert_eq!(snap.span_at(99), Some(2..5));
    }

    #[test]
    fn an_empty_range_still_gets_one_character() {
        let snap = snapshot(&["u+rwx,g"]);
        let span = snap.locate_at(0, "u+rwx,g", 6..6).unwrap();
        assert_eq!(&snap.text[span], "g");
    }

    #[test]
    fn a_range_at_the_end_of_an_operand_falls_back_to_the_operand() {
        let snap = snapshot(&["-k1,"]);
        // Nothing left to point at, so the caret takes the operand — not the
        // `-k` in front of it, which is not what went wrong.
        assert_eq!(snap.locate_at(0, "1,", 2..2), Some(2..4));
        assert_eq!(&snap.text[2..4], "1,");
    }

    #[test]
    fn offsets_hold_inside_an_operand_the_report_had_to_quote() {
        let snap = snapshot(&["a b", "-k1"]);
        // The quoting only wraps the operand, so an offset into it still lands
        // on the same bytes.
        let span = snap.locate_at(0, "a b", 1..2).unwrap();
        assert_eq!(&snap.text[span], " ");
    }

    #[test]
    fn offsets_inside_a_multibyte_operand_stay_on_character_boundaries() {
        let snap = snapshot(&["--from=éx"]);
        // Mid-character offset, walked back rather than slicing a code point.
        let span = snap.locate_at(0, "éx", 1..3).unwrap();
        assert_eq!(&snap.text[span], "éx");
    }

    #[test]
    fn index_of_finds_the_first_match() {
        let snap = snapshot(&["dup", "-ef", "dup"]);
        assert_eq!(snap.index_of(OsStr::new("dup")), Some(0));
        assert_eq!(snap.index_of(OsStr::new("absent")), None);
    }

    #[test]
    fn the_program_name_is_never_the_argument_looked_for() {
        // `printf printf` prints its own name; the operand is the second one.
        let snap = Snapshot::with_program(&["printf", "printf"]);
        assert_eq!(snap.index_of(OsStr::new("printf")), Some(1));
    }

    #[test]
    fn an_operand_starting_with_a_dash_is_still_found() {
        // What `index_of_positional` cannot do: printf takes hyphen values, so
        // `-%y` is the format rather than an option.
        let snap = Snapshot::with_program(&["printf", "-%y", "arg"]);
        assert_eq!(snap.index_of(OsStr::new("-%y")), Some(1));
        assert_eq!(snap.index_of_positional(0), Some(2));
    }

    #[test]
    fn a_value_is_found_in_every_spelling_of_its_option() {
        for (args, expected) in [
            (&["-k", "2.3q", "f"][..], 1),
            (&["-k2.3q", "f"][..], 0),
            (&["-rk2.3q", "f"][..], 0),
            (&["--key", "2.3q", "f"][..], 1),
            (&["--key=2.3q", "f"][..], 0),
        ] {
            let snap = snapshot(args);
            assert_eq!(
                snap.index_of_value("2.3q", Some('k'), Some("key")),
                Some(expected),
                "in {args:?}"
            );
        }
    }

    #[test]
    fn the_program_name_is_never_an_option_value() {
        let snap = Snapshot::with_program(&["sort", "-k", "sort"]);
        assert_eq!(snap.index_of_value("sort", Some('k'), Some("key")), Some(2));
    }

    #[test]
    fn a_file_sharing_the_value_text_is_not_the_value() {
        let snap = Snapshot::with_program(&["sort", "data.2.3q", "-k2.3q"]);
        assert_eq!(snap.index_of_value("2.3q", Some('k'), Some("key")), Some(2));
    }

    #[test]
    fn another_option_sharing_the_value_suffix_is_not_the_value() {
        let snap = Snapshot::with_program(&["numfmt", "--delimiter=%q", "--format=%q", "1000"]);
        assert_eq!(snap.index_of_value("%q", None, Some("format")), Some(2));
    }

    #[test]
    fn a_long_option_is_not_mistaken_for_a_short_cluster() {
        // `--am=644` ends in `m` + the operand, but it is not a `-m` cluster.
        let snap = snapshot(&["--am=644", "-m644", "d"]);
        assert_eq!(snap.index_of_value("644", Some('m'), Some("mode")), Some(1));
    }

    #[test]
    fn a_value_after_a_double_dash_is_not_an_option_value() {
        let snap = snapshot(&["--", "-k", "2.3q"]);
        assert_eq!(snap.index_of_value("2.3q", Some('k'), Some("key")), None);
    }

    #[test]
    fn an_empty_value_is_the_argument_after_the_flag() {
        // `-k` ends in `k` and in the empty operand, but it is the flag.
        let snap = snapshot(&["-k", "", "f"]);
        assert_eq!(snap.index_of_value("", Some('k'), Some("key")), Some(1));
    }

    #[test]
    fn a_missing_value_cannot_be_located() {
        let snap = snapshot(&["-x", "unrelated"]);
        assert_eq!(snap.index_of_value("2.3q", Some('k'), Some("key")), None);
    }

    #[test]
    fn positionals_are_counted_across_options() {
        let snap = Snapshot::with_program(&["tr", "-c", "ab", "tr"]);
        assert_eq!(snap.index_of_positional(0), Some(2));
        assert_eq!(snap.index_of_positional(1), Some(3));
        assert_eq!(snap.index_of_positional(2), None);
    }

    #[test]
    fn a_double_dash_ends_options_for_positionals() {
        let snap = Snapshot::with_program(&["tr", "--", "-d", "x"]);
        assert_eq!(snap.index_of_positional(0), Some(2));
        assert_eq!(snap.index_of_positional(1), Some(3));
    }

    #[test]
    fn a_lone_dash_is_a_positional() {
        let snap = snapshot(&["-c", "-", "x"]);
        assert_eq!(snap.index_of_positional(0), Some(1));
    }

    /// A utility with one option of each spelling that takes a value.
    const VALUE_OPTIONS: ValueOptions = ValueOptions {
        shorts: &['d'],
        longs: &["suffix", "delimiter"],
    };

    #[test]
    fn an_operand_is_found_past_a_detached_option_value() {
        // Counting positionals would stop at the `q` given to `--suffix`.
        let snap = Snapshot::with_program(&["numfmt", "--suffix", "q", "1000"]);
        assert_eq!(snap.index_of_operand(0, &VALUE_OPTIONS), Some(3));
        assert_eq!(snap.index_of_operand(1, &VALUE_OPTIONS), None);
        assert_eq!(snap.index_of_positional(0), Some(2));
    }

    #[test]
    fn an_attached_value_does_not_swallow_the_operand() {
        for args in [
            &["numfmt", "--suffix=q", "1000"][..],
            &["numfmt", "-dq", "1000"][..],
        ] {
            let snap = Snapshot::with_program(args);
            assert_eq!(
                snap.index_of_operand(0, &VALUE_OPTIONS),
                Some(2),
                "in {args:?}"
            );
        }
    }

    #[test]
    fn an_abbreviated_long_option_still_takes_its_value() {
        let snap = Snapshot::with_program(&["numfmt", "--suf", "q", "1000"]);
        assert_eq!(snap.index_of_operand(0, &VALUE_OPTIONS), Some(3));
    }

    #[test]
    fn only_the_last_of_a_short_cluster_takes_the_next_argument() {
        // `-zd` ends in the value option, `-dz` does not.
        let snap = Snapshot::with_program(&["numfmt", "-zd", ",", "1000"]);
        assert_eq!(snap.index_of_operand(0, &VALUE_OPTIONS), Some(3));
        let snap = Snapshot::with_program(&["numfmt", "-dz", "1000"]);
        assert_eq!(snap.index_of_operand(0, &VALUE_OPTIONS), Some(2));
    }

    #[test]
    fn an_option_value_past_a_double_dash_is_an_operand() {
        let snap = Snapshot::with_program(&["numfmt", "--", "--suffix", "q"]);
        assert_eq!(snap.index_of_operand(0, &VALUE_OPTIONS), Some(2));
        assert_eq!(snap.index_of_operand(1, &VALUE_OPTIONS), Some(3));
    }

    #[test]
    fn an_option_the_table_does_not_name_takes_nothing() {
        let snap = Snapshot::with_program(&["numfmt", "--round", "up", "1000"]);
        assert_eq!(snap.index_of_operand(0, &VALUE_OPTIONS), Some(2));
    }

    #[test]
    fn locate_at_points_inside_the_named_argument() {
        for (args, index) in [(&["-k2.3x", "f"][..], 0), (&["-k", "2.3x", "f"][..], 1)] {
            let snap = snapshot(args);
            let span = snap.locate_at(index, "2.3x", 3..4).unwrap();
            assert_eq!(&snap.text[span], "x", "in {args:?}");
        }
    }

    #[test]
    fn locate_at_ignores_an_earlier_argument_with_the_same_tail() {
        let snap = snapshot(&["data.2.3q", "-k2.3q"]);
        let span = snap.locate_at(1, "2.3q", 3..4).unwrap();
        assert_eq!(span, snap.spans[1].start + 5..snap.spans[1].start + 6);
    }

    #[test]
    fn locate_at_underlines_the_whole_argument_when_the_operand_is_not_its_tail() {
        let snap = snapshot(&["ab", "cd"]);
        assert_eq!(snap.locate_at(1, "zz", 0..1), Some(3..5));
    }

    #[test]
    fn locate_at_past_the_end_cannot_be_located() {
        let snap = snapshot(&["ab"]);
        assert_eq!(snap.locate_at(5, "ab", 0..1), None);
    }
}
