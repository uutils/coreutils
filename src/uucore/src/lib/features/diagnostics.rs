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
//! A utility is expected to map its own error type to an argument index, a
//! label, and optionally a line of advice; everything user-facing is passed in
//! already localized, so this module holds no messages of its own.
//!
//! Rendering only happens when stderr is a terminal, so anything reading our
//! output — a script, a pipe, a test suite — still sees the plain one-line
//! message it always did.
//!
//! ```text
//! test: invalid integer 'zap'
//!    ╭─[ test:1:7 ]
//!    │
//!  1 │ 7 -eq zap
//!    │       ─┬─
//!    │        ╰─── expected an integer here
//! ───╯
//! ```

// spell-checker:ignore étage

use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt::Write as _;
use std::io::IsTerminal;
use std::ops::Range;

use ariadne::{CharSet, Color, Config, IndexType, Label, Report, ReportKind, Source};

use crate::display::Quotable;

/// Whether errors should be rendered against their argument list.
///
/// Callers should check this before doing any work that only a diagnostic needs,
/// so that the default path costs nothing.
///
/// # Returns
///
/// `true` when stderr is a terminal — a person is watching, and gets the rich
/// form. `false` in a script or a pipe, where whatever reads stderr gets the
/// plain message it can grep for.
pub fn enabled() -> bool {
    std::io::stderr().is_terminal()
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

/// An argument list rendered as a single line, with the position of every
/// argument inside it.
pub struct Snapshot {
    /// The arguments joined by spaces, quoted where needed.
    text: String,
    /// Byte range of each argument inside `text`.
    spans: Vec<Range<usize>>,
    /// Whether each argument was written out as-is, so that an offset inside it
    /// also holds inside `text`.
    verbatim: Vec<bool>,
    /// The arguments themselves, for [`Snapshot::index_of`].
    args: Vec<OsString>,
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
            verbatim: Vec::with_capacity(len),
            args: Vec::with_capacity(len),
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
        let verbatim = match arg.to_str() {
            // Operators such as `=` or `!=` are shell-special, but quoting them
            // here would only obscure the expression.
            Some(s) if !s.is_empty() && !s.chars().any(char::is_whitespace) => {
                self.text.push_str(s);
                true
            }
            _ => {
                let _ = write!(self.text, "{}", arg.maybe_quote());
                false
            }
        };
        self.spans.push(start..self.text.len());
        self.verbatim.push(verbatim);
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
    /// The index of the first argument equal to `arg`, or `None` if there is
    /// none. A repeated value resolves to its first occurrence.
    pub fn index_of(&self, arg: &OsStr) -> Option<usize> {
        self.args.iter().position(|candidate| candidate == arg)
    }

    /// Index of the first argument equal to `arg`, held as raw bytes.
    ///
    /// # Arguments
    ///
    /// * `arg` - The argument to look for, as raw bytes.
    ///
    /// # Returns
    ///
    /// The index of the first argument equal to `arg`, or `None` if there is
    /// none or the bytes cannot be represented as an [`OsStr`] on this
    /// platform.
    pub fn index_of_bytes(&self, arg: &[u8]) -> Option<usize> {
        self.index_of(&crate::os_str_from_bytes(arg).ok()?)
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
    /// * `label` - Text placed under the caret, already localized.
    /// * `help` - An optional line of advice, already localized.
    ///
    /// # Returns
    ///
    /// `false` if nothing could be rendered, in which case the caller should
    /// fall back to a plain one-line message.
    pub fn render(&self, index: usize, message: &str, label: &str, help: Option<&str>) -> bool {
        let Some(span) = self.span_at(index) else {
            return false;
        };
        self.report(span, message, label, help)
    }

    /// Write a report pointing at `range`, a byte range *inside* `operand`.
    ///
    /// For utilities whose operands are small languages of their own — a `sort`
    /// key, a `chmod` mode — the argument as a whole is rarely the answer; the
    /// caret belongs under the one character that broke the parse. `operand` is
    /// looked up among the arguments, so a key passed as `-k2.3x` and one passed
    /// as `-k 2.3x` both point at the same place.
    ///
    /// Falls back to underlining the whole argument when quoting means an offset
    /// inside `operand` no longer lines up with what is printed.
    ///
    /// # Arguments
    ///
    /// * `operand` - The operand at fault, as it was parsed.
    /// * `range` - Byte range inside `operand` to point at. An empty range
    ///   marks the character it starts at.
    /// * `message` - The error message, already localized.
    /// * `label` - Text placed under the caret, already localized.
    /// * `help` - An optional line of advice, already localized.
    ///
    /// # Returns
    ///
    /// `false` if nothing could be rendered, in which case the caller should
    /// fall back to a plain one-line message.
    pub fn render_inside(
        &self,
        operand: &str,
        range: Range<usize>,
        message: &str,
        label: &str,
        help: Option<&str>,
    ) -> bool {
        let Some(span) = self.locate(operand, range) else {
            return false;
        };
        self.report(span, message, label, help)
    }

    /// Byte range covered by `range` — an offset inside `operand` — once
    /// `operand` has been found among the arguments.
    fn locate(&self, operand: &str, range: Range<usize>) -> Option<Range<usize>> {
        let index = self
            .args
            .iter()
            .position(|arg| arg.as_encoded_bytes().ends_with(operand.as_bytes()))?;
        let whole = self.spans[index].clone();
        if !self.verbatim[index] {
            return Some(whole);
        }

        let base = whole.end - operand.len();
        // Offsets come from someone else's parser, so they are only trusted as
        // far as the text agrees with them.
        let start = self.floor_boundary(base + range.start.min(operand.len()));
        let end = self.floor_boundary(base + range.end.clamp(range.start, operand.len()));
        if start < end {
            return Some(start..end);
        }
        // An empty range means something is missing rather than wrong; give the
        // caret the character it stopped at, or the whole argument if the
        // operand ran out.
        match self.text[start..].chars().next() {
            Some(c) if start < whole.end => Some(start..start + c.len_utf8()),
            _ => Some(whole),
        }
    }

    /// `offset`, moved back to the nearest character boundary.
    fn floor_boundary(&self, offset: usize) -> usize {
        (0..=offset)
            .rev()
            .find(|&i| self.text.is_char_boundary(i))
            .unwrap_or(0)
    }

    fn report(&self, span: Range<usize>, message: &str, label: &str, help: Option<&str>) -> bool {
        let id = crate::util_name();
        let color = env::var_os("NO_COLOR").is_none() && std::io::stderr().is_terminal();
        let config = Config::default()
            // ariadne counts characters unless told otherwise, which would drift
            // on multi-byte arguments.
            .with_index_type(IndexType::Byte)
            .with_color(color)
            .with_char_set(CharSet::Unicode);

        let mut report = Report::build(ReportKind::Error, (id, span.clone()))
            .with_config(config)
            .with_label(
                Label::new((id, span))
                    .with_message(label)
                    .with_color(Color::Red),
            );

        if let Some(help) = help {
            report = report.with_help(help);
        }

        let mut rendered = Vec::new();
        if report
            .finish()
            .write((id, Source::from(self.text.as_str())), &mut rendered)
            .is_err()
        {
            return false;
        }

        // ariadne heads every report with its own line — a hardcoded, untranslated
        // "Error:". Drop it and write the utility name instead, so the first line
        // reads exactly like the plain one-line form.
        let rendered = String::from_utf8_lossy(&rendered);
        let body = rendered.split_once('\n').map_or("", |(_, rest)| rest);
        eprint!("{id}: {message}\n{body}");
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(args: &[&str]) -> Snapshot {
        Snapshot::new(args)
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
        assert_eq!(snap.index_of_bytes(b"4"), Some(2));
    }

    #[cfg(unix)]
    #[test]
    fn invalid_utf8_arguments_can_still_be_found() {
        let snap = Snapshot::from_bytes(&[&b"ba\x80d"[..], b"+", b"1"]);
        // A lossy conversion would not compare equal to the raw argument.
        assert_eq!(snap.index_of_bytes(b"ba\x80d"), Some(0));
    }

    #[test]
    fn index_past_the_end_clamps_to_the_last_argument() {
        let snap = snapshot(&["7", "-lt"]);
        assert_eq!(snap.span_at(99), Some(2..5));
    }

    #[test]
    fn a_range_inside_an_operand_is_found_whether_it_is_glued_to_the_option() {
        for args in [&["-k2.3x", "f"][..], &["-k", "2.3x", "f"][..]] {
            let snap = snapshot(args);
            let span = snap.locate("2.3x", 3..4).unwrap();
            assert_eq!(&snap.text[span], "x");
        }
    }

    #[test]
    fn an_empty_range_still_gets_one_character() {
        let snap = snapshot(&["u+rwx,g"]);
        let span = snap.locate("u+rwx,g", 6..6).unwrap();
        assert_eq!(&snap.text[span], "g");
    }

    #[test]
    fn a_range_at_the_end_of_an_operand_falls_back_to_the_argument() {
        let snap = snapshot(&["-k1,"]);
        assert_eq!(snap.locate("1,", 2..2), Some(0..4));
    }

    #[test]
    fn a_quoted_operand_falls_back_to_the_whole_argument() {
        let snap = snapshot(&["a b", "-k1"]);
        // Offsets inside `a b` would land on the quotes that were added.
        assert_eq!(snap.locate("a b", 1..2), Some(0..5));
        assert_eq!(&snap.text[snap.locate("a b", 1..2).unwrap()], "'a b'");
    }

    #[test]
    fn an_operand_that_is_not_an_argument_cannot_be_located() {
        let snap = snapshot(&["-k1"]);
        assert_eq!(snap.locate("2.3", 0..1), None);
    }

    #[test]
    fn offsets_inside_a_multibyte_operand_stay_on_character_boundaries() {
        let snap = snapshot(&["--from=éx"]);
        // Mid-character offset, walked back rather than slicing a code point.
        let span = snap.locate("éx", 1..3).unwrap();
        assert_eq!(&snap.text[span], "éx");
    }

    #[test]
    fn index_of_finds_the_first_match() {
        let snap = snapshot(&["dup", "-ef", "dup"]);
        assert_eq!(snap.index_of(OsStr::new("dup")), Some(0));
        assert_eq!(snap.index_of(OsStr::new("absent")), None);
    }
}
