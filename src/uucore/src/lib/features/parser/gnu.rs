// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore uerror colour osample aosample aovalue pattached replacen

//! GNU-style option scanning, independent of command presentation and settings.
//!
//! A utility owns a static option schema, applies the ordered events to its typed
//! settings, and stops consuming events when it handles help or version. Only
//! tooling and those display requests need to project the schema into Clap.

use std::ffi::{OsStr, OsString};
use std::fmt::Write as _;

use crate::clap_localization::{Color, ColorManager};
use crate::display::Quotable;
use crate::error::{UError, UUsageError};
use crate::translate;

/// Whether an option accepts a value. Optional values must be attached.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueArity {
    None,
    Required,
    Optional {
        default_missing: Option<&'static str>,
    },
}

/// The one source of option identity, spellings, arity, and lazy help metadata.
#[derive(Clone, Copy, Debug)]
pub struct OptionSpec {
    pub id: &'static str,
    pub short: &'static [char],
    pub long: &'static [&'static str],
    pub arity: ValueArity,
    pub help_key: &'static str,
    pub value_name: Option<&'static str>,
    hidden: bool,
    abbreviate: bool,
    visible_aliases: bool,
    short_optional_equals: bool,
    short_required_equals: bool,
    help_literal: Option<&'static str>,
    help_renderer: Option<fn() -> String>,
}

impl OptionSpec {
    /// A flag. The first spelling in each list is the displayed name.
    pub const fn flag(
        id: &'static str,
        short: &'static [char],
        long: &'static [&'static str],
        help_key: &'static str,
    ) -> Self {
        Self {
            id,
            short,
            long,
            arity: ValueArity::None,
            help_key,
            value_name: None,
            hidden: false,
            abbreviate: true,
            visible_aliases: false,
            short_optional_equals: false,
            short_required_equals: false,
            help_literal: None,
            help_renderer: None,
        }
    }

    /// An option whose value may be attached or supplied as the next token.
    pub const fn required(
        id: &'static str,
        short: &'static [char],
        long: &'static [&'static str],
        help_key: &'static str,
        value_name: &'static str,
    ) -> Self {
        let mut spec = Self::flag(id, short, long, help_key);
        spec.arity = ValueArity::Required;
        spec.value_name = Some(value_name);
        spec
    }

    /// An option whose optional value is accepted only when attached.
    pub const fn optional(
        id: &'static str,
        short: &'static [char],
        long: &'static [&'static str],
        help_key: &'static str,
        default_missing: &'static str,
        value_name: &'static str,
    ) -> Self {
        let mut spec = Self::required(id, short, long, help_key, value_name);
        spec.arity = ValueArity::Optional {
            default_missing: Some(default_missing),
        };
        spec
    }

    /// Let tooling derive the displayed value name from the option identity.
    pub const fn default_value_name(mut self) -> Self {
        self.value_name = None;
        self
    }

    /// Distinguish an omitted optional value from an explicitly empty value.
    pub const fn no_default_missing(mut self) -> Self {
        self.arity = ValueArity::Optional {
            default_missing: None,
        };
        self
    }

    pub const fn hidden(mut self) -> Self {
        self.hidden = true;
        self
    }

    /// Require an exact long spelling for this option.
    pub const fn abbreviate(mut self, enabled: bool) -> Self {
        self.abbreviate = enabled;
        self
    }

    /// Display aliases as well as accepting them in tooling interfaces.
    pub const fn visible_aliases(mut self) -> Self {
        self.visible_aliases = true;
        self
    }

    /// Treat a short optional option as a flag unless followed by `=`.
    ///
    /// This permits `-Fl` to remain a cluster while accepting `-F=always`.
    pub const fn short_optional_equals(mut self) -> Self {
        self.short_optional_equals = true;
        self
    }

    /// Accept an optional `=` before a required short option's attached value.
    ///
    /// GNU short options normally retain this character as part of the value.
    /// A utility can explicitly retain an existing uutils extension instead.
    pub const fn short_required_equals(mut self) -> Self {
        self.short_required_equals = true;
        self
    }

    /// Retain an existing literal label without translating it eagerly.
    pub const fn help_literal(mut self, text: &'static str) -> Self {
        self.help_literal = Some(text);
        self
    }

    /// Defer composed or dynamically translated help until projection.
    pub const fn help_with(mut self, render: fn() -> String) -> Self {
        self.help_renderer = Some(render);
        self
    }

    /// Construct presentation metadata only when help or tooling requests it.
    ///
    /// Callers can add value parsers, display ordering, and legacy actions; they
    /// should not repeat spellings or value arity in those customizations.
    pub fn arg(&self) -> clap::Arg {
        let mut arg = clap::Arg::new(self.id).hide(self.hidden);
        if let Some(&short) = self.short.first() {
            arg = arg.short(short);
            arg = if self.visible_aliases {
                arg.visible_short_aliases(self.short.iter().skip(1).copied())
            } else {
                arg.short_aliases(self.short.iter().skip(1).copied())
            };
        }
        if let Some(&long) = self.long.first() {
            arg = arg.long(long);
            arg = if self.visible_aliases {
                arg.visible_aliases(self.long.iter().skip(1).copied())
            } else {
                arg.aliases(self.long.iter().skip(1).copied())
            };
        }
        arg = match self.arity {
            ValueArity::None => arg.action(clap::ArgAction::SetTrue),
            ValueArity::Required => arg.action(clap::ArgAction::Set).num_args(1),
            ValueArity::Optional { default_missing } => {
                let mut arg = arg
                    .action(clap::ArgAction::Set)
                    .num_args(0..=1)
                    .require_equals(true);
                if let Some(value) = default_missing {
                    arg = arg.default_missing_value(value);
                }
                arg
            }
        };
        if let Some(value_name) = self.value_name {
            arg = arg.value_name(value_name);
        }
        if let Some(render) = self.help_renderer {
            arg = arg.help(render());
        } else if let Some(text) = self.help_literal {
            arg = arg.help(text);
        } else if !self.help_key.is_empty() {
            arg = arg.help(translate!(self.help_key));
        }
        arg
    }
}

/// How a command treats options following the first operand.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ordering {
    Intermixed,
    StopAtFirstOperand,
}

/// Explicit invocation policy: the scanner does not inspect the environment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Policy {
    pub ordering: Ordering,
}

impl Policy {
    pub const fn gnu() -> Self {
        Self {
            ordering: Ordering::Intermixed,
        }
    }

    pub const fn posix() -> Self {
        Self {
            ordering: Ordering::StopAtFirstOperand,
        }
    }
}

/// The location of an event in the original command line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Position {
    /// Argument index, with the program name at index zero.
    pub argument: usize,
    /// Zero-based position within a short option cluster, if applicable.
    pub short: Option<usize>,
    /// Increasing event index, including operands and clustered flags.
    pub sequence: usize,
}

/// Location of an explicitly supplied value inside the original argument.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValuePosition {
    pub argument: usize,
    /// The ASCII prefix preceding an attached value; zero for a separate value.
    pub offset: usize,
}

/// One option, before utility-specific precedence and validation are applied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Occurrence {
    pub id: &'static str,
    pub value: Option<OsString>,
    pub position: Position,
    /// The spelling used, without any attached value (including abbreviations).
    pub spelling: OsString,
    pub value_position: Option<ValuePosition>,
}

/// Events retain input order, including the order within short clusters.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event {
    Option(Occurrence),
    Operand { value: OsString, position: Position },
}

/// Structured failures; rendering and the exit status belong to the caller.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    UnknownOption,
    AmbiguousOption { candidates: Vec<&'static str> },
    MissingValue,
    UnexpectedValue,
    InvalidValue { value: OsString, valid: Vec<String> },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    pub kind: Box<ErrorKind>,
    pub position: Position,
    /// The original offending token for scanner errors, or the option spelling
    /// for a utility-created value error. Raw values remain in `kind`, and
    /// `position` / `value_position` locate their complete original arguments.
    /// A boxed slice keeps this cold-path error small on Windows too, where an
    /// OsString has a larger inline representation.
    pub argument: Box<OsStr>,
    pub spelling: OsString,
    pub value_position: Option<ValuePosition>,
}

impl ParseError {
    pub fn is_unknown_option(&self) -> bool {
        matches!(*self.kind, ErrorKind::UnknownOption)
    }

    pub fn argument(&self) -> &OsStr {
        &self.argument
    }

    /// Make a utility-specific value validation failure after scanning a flag.
    #[cold]
    pub fn invalid_value(occurrence: &Occurrence, valid: &[&str]) -> Self {
        Self {
            kind: Box::new(ErrorKind::InvalidValue {
                value: occurrence.value.clone().unwrap_or_default(),
                valid: valid.iter().map(|value| (*value).to_owned()).collect(),
            }),
            position: occurrence.position,
            argument: occurrence.spelling.clone().into_boxed_os_str(),
            spelling: occurrence.spelling.clone(),
            value_position: occurrence.value_position,
        }
    }

    /// Render a diagnostic without constructing a command or parsing again.
    #[cold]
    pub fn to_uerror(&self, code: i32) -> Box<dyn UError> {
        UUsageError::new(code, self.message())
    }

    /// Render against original argv when rich diagnostics were requested.
    #[cold]
    pub fn to_uerror_with_args(&self, code: i32, args: Option<&[OsString]>) -> Box<dyn UError> {
        let error = self.to_uerror(code);
        crate::diagnostics::error_after_report(args, error, |args, error| {
            let index = self
                .value_position
                .map_or(self.position.argument, |value| value.argument);
            let snapshot = crate::diagnostics::Snapshot::with_program(args);
            let message = error.to_string();
            if let Some(argument) = args.get(index).and_then(|argument| argument.to_str()) {
                let range = if let Some(value) = self.value_position {
                    value.offset..argument.len()
                } else if let Some(short) = self.position.short {
                    crate::diagnostics::char_span(argument, short + 1)
                } else {
                    0..argument.len()
                };
                snapshot.render_inside_at(index, argument, range, &message, None, None)
            } else {
                snapshot.render(index, &message, None, None)
            }
        })
    }

    #[cold]
    fn message(&self) -> String {
        let colors = ColorManager::from_env();
        let error_word = colors.colorize(&translate!("common-error"), Color::Red);
        let mut option = self.spelling.quote().to_string();
        if let Some(spelling) = self
            .spelling
            .to_str()
            .filter(|spelling| !spelling.is_empty())
        {
            // Style inside the quotes, retaining the existing escaping of raw
            // OS strings (including strings which cannot be displayed as UTF-8).
            option = option.replacen(spelling, &colors.colorize(spelling, Color::Yellow), 1);
        }
        let message = match self.kind.as_ref() {
            ErrorKind::UnknownOption => {
                translate!("parser-error-unknown-option", "option" => option)
            }
            ErrorKind::AmbiguousOption { candidates } => {
                let candidates = candidates
                    .iter()
                    .map(|name| format!("'--{name}'"))
                    .collect::<Vec<_>>()
                    .join(", ");
                translate!("parser-error-ambiguous-option", "option" => option, "candidates" => candidates)
            }
            ErrorKind::MissingValue => {
                translate!("parser-error-missing-value", "option" => option)
            }
            ErrorKind::UnexpectedValue => {
                translate!("parser-error-unexpected-value", "option" => option)
            }
            ErrorKind::InvalidValue { value, valid } => {
                let mut message = translate!("parser-error-invalid-value", "error_word" => error_word, "value" => value.quote(), "option" => option);
                if !valid.is_empty() {
                    let _ = write!(
                        message,
                        "\n  [{}: {}]",
                        translate!("clap-error-possible-values"),
                        valid.join(", ")
                    );
                }
                message
            }
        };
        let mut message = if matches!(self.kind.as_ref(), ErrorKind::InvalidValue { .. }) {
            message
        } else {
            format!("{error_word}: {message}")
        };
        if self.is_unknown_option() {
            // Keep the public diagnostic's usage line. This only renders the
            // utility's existing usage key; no command or help page is built.
            let usage = translate!(&format!("{}-usage", crate::util_name()));
            let _ = write!(
                message,
                "\n\n{}: {}\n",
                translate!("common-usage"),
                crate::format_usage(&usage)
            );
        }
        message
    }
}

struct Cluster {
    argument: OsString,
    index: usize,
    offset: usize,
    ordinal: usize,
}

/// An incremental scanner. Input excludes `argv[0]`; indices still include it.
///
/// No command metadata is constructed while scanning. Required values consume
/// the next token even when it is `--` or starts with `-`. An omitted optional
/// value never consumes the next token.
pub struct Parser<'a, I> {
    args: I,
    specs: &'a [OptionSpec],
    policy: Policy,
    argument: usize,
    sequence: usize,
    operands_only: bool,
    failed: bool,
    cluster: Option<Cluster>,
}

impl<'a, I: Iterator<Item = OsString>> Parser<'a, I> {
    pub fn new(args: I, specs: &'a [OptionSpec], policy: Policy) -> Self {
        Self {
            args,
            specs,
            policy,
            argument: 0,
            sequence: 0,
            operands_only: false,
            failed: false,
            cluster: None,
        }
    }

    fn position(&mut self, argument: usize, short: Option<usize>) -> Position {
        self.sequence += 1;
        Position {
            argument,
            short,
            sequence: self.sequence,
        }
    }

    fn error(
        kind: ErrorKind,
        position: Position,
        argument: OsString,
        spelling: OsString,
    ) -> ParseError {
        ParseError {
            kind: Box::new(kind),
            position,
            argument: argument.into_boxed_os_str(),
            spelling,
            value_position: None,
        }
    }

    fn long(&mut self, argument: OsString, index: usize) -> Result<Event, ParseError> {
        let position = self.position(index, None);
        // Splitting is platform-specific: a Unix byte sequence is not a Windows
        // OS string, and invalid value encodings must survive unchanged.
        let (name, attached) = split_long(&argument);
        let spelling = {
            let mut spelling = OsString::from("--");
            spelling.push(&name);
            spelling
        };
        let name_text = name.to_str();
        let exact =
            name_text.and_then(|name| self.specs.iter().find(|spec| spec.long.contains(&name)));
        let spec = if let Some(spec) = exact {
            Some(spec)
        } else {
            let mut matches = self.specs.iter().filter(|spec| {
                spec.abbreviate
                    && name_text
                        .is_some_and(|name| spec.long.iter().any(|long| long.starts_with(name)))
            });
            let first = matches.next();
            if matches.any(|candidate| {
                first
                    .is_some_and(|first| candidate.id != first.id || candidate.arity != first.arity)
            }) {
                let candidates = self
                    .specs
                    .iter()
                    .filter(|spec| spec.abbreviate)
                    .flat_map(|spec| spec.long.iter().copied())
                    .filter(|long| name_text.is_some_and(|name| long.starts_with(name)))
                    .collect();
                return Err(Self::error(
                    ErrorKind::AmbiguousOption { candidates },
                    position,
                    argument,
                    spelling,
                ));
            }
            first
        };
        let Some(spec) = spec else {
            return Err(Self::error(
                ErrorKind::UnknownOption,
                position,
                argument,
                spelling,
            ));
        };
        let attached_offset = 3 + name.as_encoded_bytes().len();
        let (value, value_position) = match spec.arity {
            ValueArity::None => {
                if attached.is_some() {
                    return Err(Self::error(
                        ErrorKind::UnexpectedValue,
                        position,
                        argument,
                        spelling,
                    ));
                }
                (None, None)
            }
            ValueArity::Required => {
                if let Some(value) = attached {
                    (
                        Some(value),
                        Some(ValuePosition {
                            argument: index,
                            offset: attached_offset,
                        }),
                    )
                } else if let Some(value) = self.args.next() {
                    self.argument += 1;
                    (
                        Some(value),
                        Some(ValuePosition {
                            argument: self.argument,
                            offset: 0,
                        }),
                    )
                } else {
                    return Err(Self::error(
                        ErrorKind::MissingValue,
                        position,
                        argument,
                        spelling,
                    ));
                }
            }
            ValueArity::Optional { default_missing } => {
                if let Some(value) = attached {
                    (
                        Some(value),
                        Some(ValuePosition {
                            argument: index,
                            offset: attached_offset,
                        }),
                    )
                } else {
                    (default_missing.map(OsString::from), None)
                }
            }
        };
        Ok(Event::Option(Occurrence {
            id: spec.id,
            value,
            position,
            spelling,
            value_position,
        }))
    }

    fn short(&mut self, mut cluster: Cluster) -> Result<Event, ParseError> {
        let position = self.position(cluster.index, Some(cluster.ordinal));
        let byte = cluster
            .argument
            .as_encoded_bytes()
            .get(cluster.offset)
            .copied();
        let spec = byte.filter(u8::is_ascii).and_then(|byte| {
            self.specs
                .iter()
                .find(|spec| spec.short.contains(&char::from(byte)))
        });
        let Some(spec) = spec else {
            let spelling = cluster.argument.clone();
            return Err(Self::error(
                ErrorKind::UnknownOption,
                position,
                cluster.argument,
                spelling,
            ));
        };
        // A match proves that this byte was ASCII, even in an invalid OS string.
        let short = char::from(byte.unwrap_or_default());
        let spelling = OsString::from(format!("-{short}"));
        cluster.offset += 1;
        cluster.ordinal += 1;
        let has_tail = cluster.offset < cluster.argument.as_encoded_bytes().len();
        let (value, value_position) = match spec.arity {
            ValueArity::None => {
                if has_tail {
                    self.cluster = Some(cluster);
                }
                (None, None)
            }
            ValueArity::Required => {
                if has_tail {
                    let equals =
                        cluster.argument.as_encoded_bytes().get(cluster.offset) == Some(&b'=');
                    let offset = cluster.offset + usize::from(spec.short_required_equals && equals);
                    let value = suffix(&cluster.argument, offset);
                    (
                        Some(value),
                        Some(ValuePosition {
                            argument: cluster.index,
                            offset,
                        }),
                    )
                } else if let Some(value) = self.args.next() {
                    self.argument += 1;
                    (
                        Some(value),
                        Some(ValuePosition {
                            argument: self.argument,
                            offset: 0,
                        }),
                    )
                } else {
                    return Err(Self::error(
                        ErrorKind::MissingValue,
                        position,
                        cluster.argument,
                        spelling,
                    ));
                }
            }
            ValueArity::Optional { default_missing } => {
                let equals = cluster.argument.as_encoded_bytes().get(cluster.offset) == Some(&b'=');
                if has_tail && (!spec.short_optional_equals || equals) {
                    let offset = cluster.offset + usize::from(spec.short_optional_equals);
                    (
                        Some(suffix(&cluster.argument, offset)),
                        Some(ValuePosition {
                            argument: cluster.index,
                            offset,
                        }),
                    )
                } else {
                    if has_tail {
                        self.cluster = Some(cluster);
                    }
                    (default_missing.map(OsString::from), None)
                }
            }
        };
        Ok(Event::Option(Occurrence {
            id: spec.id,
            value,
            position,
            spelling,
            value_position,
        }))
    }
}

impl<I: Iterator<Item = OsString>> Iterator for Parser<'_, I> {
    type Item = Result<Event, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed {
            return None;
        }
        loop {
            let result = if let Some(cluster) = self.cluster.take() {
                self.short(cluster)
            } else {
                let argument = self.args.next()?;
                self.argument += 1;
                let index = self.argument;
                if self.operands_only
                    || argument == "-"
                    || !argument.as_encoded_bytes().starts_with(b"-")
                {
                    if self.policy.ordering == Ordering::StopAtFirstOperand {
                        self.operands_only = true;
                    }
                    let position = self.position(index, None);
                    Ok(Event::Operand {
                        value: argument,
                        position,
                    })
                } else if argument == "--" {
                    self.operands_only = true;
                    continue;
                } else if argument.as_encoded_bytes().starts_with(b"--") {
                    self.long(argument, index)
                } else {
                    self.cluster = Some(Cluster {
                        argument,
                        index,
                        offset: 1,
                        ordinal: 0,
                    });
                    continue;
                }
            };
            self.failed = result.is_err();
            return Some(result);
        }
    }
}

#[cfg(any(unix, all(target_os = "wasi", target_env = "p1")))]
fn suffix(value: &OsStr, offset: usize) -> OsString {
    #[cfg(unix)]
    use std::os::unix::ffi::OsStrExt;
    #[cfg(all(target_os = "wasi", target_env = "p1", not(unix)))]
    use std::os::wasi::ffi::OsStrExt;
    OsStr::from_bytes(&value.as_encoded_bytes()[offset..]).to_owned()
}

#[cfg(windows)]
fn suffix(value: &OsStr, offset: usize) -> OsString {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    // The skipped prefix consists only of ASCII option spelling characters.
    OsString::from_wide(&value.encode_wide().skip(offset).collect::<Vec<_>>())
}

#[cfg(not(any(unix, windows, all(target_os = "wasi", target_env = "p1"))))]
fn suffix(value: &OsStr, offset: usize) -> OsString {
    // No byte or wide-string extension API is available on these targets.
    // Split valid UTF-8 safely, preserving an unsupported encoding whole rather
    // than replacing its bytes and deciding parsing behavior from that text.
    value
        .to_str()
        .and_then(|value| value.get(offset..))
        .map_or_else(|| value.to_owned(), OsString::from)
}

fn split_long(value: &OsStr) -> (OsString, Option<OsString>) {
    let bytes = value.as_encoded_bytes();
    if let Some(equal) = bytes.iter().position(|byte| *byte == b'=') {
        // All bytes before '=' name an option. Valid names are ASCII, so using
        // their length to split the native string never splits a code point.
        let name = &bytes[2..equal];
        if !name.is_ascii() {
            // Unknown non-ASCII names cannot use encoded byte offsets to split
            // a Windows wide string. Preserve the whole unknown spelling.
            return (suffix(value, 2), None);
        }
        let tail = suffix(value, equal + 1);
        let name = std::str::from_utf8(name).map_or_else(|_| suffix(value, 2), OsString::from);
        (name, Some(tail))
    } else {
        (suffix(value, 2), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPTIONS: &[OptionSpec] = &[
        OptionSpec::flag("all", &['a'], &["all", "everything"], ""),
        OptionSpec::flag("brief", &['b'], &["brief"], ""),
        OptionSpec::required("output", &['o'], &["output"], "", "FILE"),
        OptionSpec::optional("color", &[], &["color", "colour"], "", "always", "WHEN"),
        OptionSpec::optional("classify", &['F'], &["classify"], "", "always", "WHEN")
            .short_optional_equals(),
        OptionSpec::optional("optional", &['p'], &["optional"], "", "default", "VALUE"),
        OptionSpec::optional("root", &[], &["preserve-root"], "", "", "MODE").no_default_missing(),
        OptionSpec::flag("help", &['h'], &["help"], ""),
        OptionSpec::flag("version", &[], &["version"], ""),
    ];

    fn parse(args: &[&str], policy: Policy) -> Result<Vec<Event>, ParseError> {
        Parser::new(args.iter().map(OsString::from), OPTIONS, policy).collect()
    }

    fn options(events: &[Event]) -> Vec<(&str, Option<&OsStr>)> {
        events
            .iter()
            .filter_map(|event| match event {
                Event::Option(option) => Some((option.id, option.value.as_deref())),
                Event::Operand { .. } => None,
            })
            .collect()
    }

    fn operands(events: &[Event]) -> Vec<&OsStr> {
        events
            .iter()
            .filter_map(|event| match event {
                Event::Operand { value, .. } => Some(value.as_os_str()),
                Event::Option(_) => None,
            })
            .collect()
    }

    #[test]
    fn intermixes_without_reordering_occurrences_or_operands() {
        let events = parse(
            &["first", "-aba", "second", "--brief", "third"],
            Policy::gnu(),
        )
        .unwrap();
        assert_eq!(
            options(&events),
            [
                ("all", None),
                ("brief", None),
                ("all", None),
                ("brief", None)
            ]
        );
        assert_eq!(operands(&events), ["first", "second", "third"]);
        assert!(matches!(
            &events[0],
            Event::Operand {
                position: Position {
                    argument: 1,
                    sequence: 1,
                    ..
                },
                ..
            }
        ));
        for (event, ordinal) in events[1..4].iter().zip(0..3) {
            let Event::Option(option) = event else {
                panic!("expected option");
            };
            assert_eq!(
                option.position,
                Position {
                    argument: 2,
                    short: Some(ordinal),
                    sequence: ordinal + 2
                }
            );
        }
    }

    #[test]
    fn posix_stops_after_first_real_operand_not_a_required_value() {
        let events = parse(
            &["-o", "output", "-a", "file", "-b", "--help"],
            Policy::posix(),
        )
        .unwrap();
        assert_eq!(
            options(&events),
            [("output", Some(OsStr::new("output"))), ("all", None)]
        );
        assert_eq!(operands(&events), ["file", "-b", "--help"]);
    }

    #[test]
    fn lone_dash_and_empty_string_are_operands() {
        for first in ["-", ""] {
            let events = parse(&[first, "-a"], Policy::posix()).unwrap();
            assert!(options(&events).is_empty());
            assert_eq!(operands(&events), [first, "-a"]);
        }
    }

    #[test]
    fn explicit_terminator_disables_all_option_recognition() {
        let events = parse(&["-a", "--", "--help", "-b", "--", "-"], Policy::gnu()).unwrap();
        assert_eq!(options(&events), [("all", None)]);
        assert_eq!(operands(&events), ["--help", "-b", "--", "-"]);
    }

    #[test]
    fn accepts_all_required_value_spellings() {
        for args in [
            vec!["-o", "sample"],
            vec!["-osample"],
            vec!["--output", "sample"],
            vec!["--output=sample"],
            vec!["-aosample"],
        ] {
            let events = parse(&args, Policy::gnu()).unwrap();
            assert_eq!(
                options(&events).last(),
                Some(&("output", Some(OsStr::new("sample"))))
            );
        }
    }

    #[test]
    fn required_value_consumes_option_looking_tokens_and_terminator() {
        for value in ["-a", "--help", "--", "-", ""] {
            for option in ["-o", "--output"] {
                let events = parse(&[option, value, "-b"], Policy::posix()).unwrap();
                assert_eq!(
                    options(&events),
                    [("output", Some(OsStr::new(value))), ("brief", None)]
                );
                assert!(operands(&events).is_empty());
            }
        }
    }

    #[test]
    fn required_value_positions_distinguish_attached_and_separate_values() {
        for (args, argument, offset) in [
            (vec!["-ao", "value"], 2, 0),
            (vec!["-aovalue"], 1, 3),
            (vec!["--output", "value"], 2, 0),
            (vec!["--out=value"], 1, 6),
        ] {
            let events = parse(&args, Policy::gnu()).unwrap();
            let Event::Option(option) = events.last().unwrap() else {
                panic!("expected option");
            };
            assert_eq!(
                option.value_position,
                Some(ValuePosition { argument, offset })
            );
        }
    }

    #[test]
    fn required_short_equals_extension_is_explicit_and_retains_positions() {
        const SPECS: &[OptionSpec] = &[
            OptionSpec::required("gnu", &['o'], &[], "", "VALUE"),
            OptionSpec::required("extension", &['w'], &[], "", "VALUE").short_required_equals(),
        ];
        for (argument, expected, offset) in [
            ("-o=37", "=37", 2),
            ("-w=37", "37", 3),
            ("-w==37", "=37", 3),
            ("-w37", "37", 2),
            ("-w=", "", 3),
        ] {
            let event = Parser::new([OsString::from(argument)].into_iter(), SPECS, Policy::gnu())
                .next()
                .unwrap()
                .unwrap();
            let Event::Option(option) = event else {
                panic!("expected option");
            };
            assert_eq!(option.value.as_deref(), Some(OsStr::new(expected)));
            assert_eq!(
                option.value_position,
                Some(ValuePosition {
                    argument: 1,
                    offset
                })
            );
        }
        // A separate value keeps '=': the extension only concerns attachment.
        let events = Parser::new(
            ["-w", "=37"].map(OsString::from).into_iter(),
            SPECS,
            Policy::gnu(),
        )
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
        assert_eq!(options(&events), [("extension", Some(OsStr::new("=37")))]);
    }

    #[cfg(unix)]
    #[test]
    fn required_short_equals_extension_preserves_raw_value() {
        use std::os::unix::ffi::OsStringExt;
        const SPECS: &[OptionSpec] =
            &[OptionSpec::required("extension", &['w'], &[], "", "VALUE").short_required_equals()];
        let argument = OsString::from_vec(b"-w=raw_\xff".to_vec());
        let event = Parser::new([argument].into_iter(), SPECS, Policy::gnu())
            .next()
            .unwrap()
            .unwrap();
        let Event::Option(option) = event else {
            panic!("expected option");
        };
        assert_eq!(option.value, Some(OsString::from_vec(b"raw_\xff".to_vec())));
        assert_eq!(
            option.value_position,
            Some(ValuePosition {
                argument: 1,
                offset: 3
            })
        );
    }

    #[test]
    fn optional_long_values_never_consume_an_operand() {
        let events = parse(
            &["--color", "never", "--colour=auto", "tail"],
            Policy::gnu(),
        )
        .unwrap();
        assert_eq!(
            options(&events),
            [
                ("color", Some(OsStr::new("always"))),
                ("color", Some(OsStr::new("auto")))
            ]
        );
        assert_eq!(operands(&events), ["never", "tail"]);
    }

    #[test]
    fn optional_short_values_are_attached_only() {
        let events = parse(&["-p", "separate", "-pattached", "-p=equal"], Policy::gnu()).unwrap();
        assert_eq!(
            options(&events),
            [
                ("optional", Some(OsStr::new("default"))),
                ("optional", Some(OsStr::new("attached"))),
                ("optional", Some(OsStr::new("=equal")))
            ]
        );
        assert_eq!(operands(&events), ["separate"]);
    }

    #[test]
    fn short_optional_equals_keeps_clusters_and_accepts_empty_values() {
        let events = parse(&["-Fab", "-F=never", "-F=", "next"], Policy::gnu()).unwrap();
        assert_eq!(
            options(&events),
            [
                ("classify", Some(OsStr::new("always"))),
                ("all", None),
                ("brief", None),
                ("classify", Some(OsStr::new("never"))),
                ("classify", Some(OsStr::new("")))
            ]
        );
        assert_eq!(operands(&events), ["next"]);
    }

    #[test]
    fn omitted_optional_value_differs_from_explicit_empty_value() {
        let events = parse(&["--preserve-root", "--preserve-root="], Policy::gnu()).unwrap();
        assert_eq!(
            options(&events),
            [("root", None), ("root", Some(OsStr::new("")))]
        );
    }

    #[test]
    fn exact_names_override_an_ambiguous_prefix() {
        const SPECS: &[OptionSpec] = &[
            OptionSpec::flag("sort", &[], &["sort"], ""),
            OptionSpec::flag("sortable", &[], &["sortable"], ""),
        ];
        let events = Parser::new([OsString::from("--sort")].into_iter(), SPECS, Policy::gnu())
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(options(&events), [("sort", None)]);
    }

    #[test]
    fn aliases_of_one_option_do_not_make_its_prefix_ambiguous() {
        let events = parse(&["--colo", "--every"], Policy::gnu()).unwrap();
        assert_eq!(
            options(&events),
            [("color", Some(OsStr::new("always"))), ("all", None)]
        );
    }

    #[test]
    fn ambiguous_prefix_retains_candidates_and_original_position() {
        let error = parse(&["operand", "--o"], Policy::gnu()).unwrap_err();
        assert_eq!(
            *error.kind,
            ErrorKind::AmbiguousOption {
                candidates: vec!["output", "optional"]
            }
        );
        assert_eq!(error.argument(), "--o");
        assert_eq!(error.position.argument, 2);
    }

    #[test]
    fn empty_long_prefix_with_equals_is_ambiguous() {
        // The bare terminator is handled before long matching; adding '='
        // instead supplies a value to an empty abbreviation, as GNU 9.12 does.
        let error = parse(&["--=sample"], Policy::gnu()).unwrap_err();
        let ErrorKind::AmbiguousOption { candidates } = error.kind.as_ref() else {
            panic!("expected ambiguity");
        };
        assert!(candidates.contains(&"all"));
        assert!(candidates.contains(&"output"));
        assert_eq!(error.argument(), "--=sample");
        assert_eq!(error.spelling, "--");
    }

    #[test]
    fn exact_only_option_is_not_an_abbreviation_candidate() {
        const SPECS: &[OptionSpec] =
            &[OptionSpec::flag("protected", &[], &["protected"], "").abbreviate(false)];
        assert!(
            Parser::new([OsString::from("--prot")].into_iter(), SPECS, Policy::gnu())
                .next()
                .unwrap()
                .unwrap_err()
                .is_unknown_option()
        );
        assert!(
            Parser::new(
                [OsString::from("--protected")].into_iter(),
                SPECS,
                Policy::gnu()
            )
            .next()
            .unwrap()
            .is_ok()
        );
    }

    #[test]
    fn missing_values_and_values_on_flags_are_distinct_errors() {
        for option in ["-o", "--output"] {
            assert_eq!(
                *parse(&[option], Policy::gnu()).unwrap_err().kind,
                ErrorKind::MissingValue
            );
        }
        assert_eq!(
            *parse(&["--all=yes"], Policy::gnu()).unwrap_err().kind,
            ErrorKind::UnexpectedValue
        );
    }

    #[test]
    fn unknown_option_fuses_the_iterator() {
        let mut parser = Parser::new(
            ["-ab?", "--help"].map(OsString::from).into_iter(),
            OPTIONS,
            Policy::gnu(),
        );
        assert!(parser.next().unwrap().is_ok());
        assert!(parser.next().unwrap().is_ok());
        let error = parser.next().unwrap().unwrap_err();
        assert_eq!(error.position.short, Some(2));
        assert_eq!(error.argument(), "-ab?");
        assert!(error.is_unknown_option());
        assert!(parser.next().is_none());
    }

    #[test]
    fn incremental_help_does_not_inspect_later_arguments() {
        let mut input = ["--help", "--unknown"].map(OsString::from).into_iter();
        let mut parser = Parser::new(input.by_ref(), OPTIONS, Policy::gnu());
        assert!(matches!(
            parser.next(),
            Some(Ok(Event::Option(Occurrence { id: "help", .. })))
        ));
        drop(parser);
        assert_eq!(input.next(), Some(OsString::from("--unknown")));
        assert!(parse(&["--unknown", "--help"], Policy::gnu()).is_err());
    }

    #[test]
    fn successful_scanning_does_not_render_help_metadata() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static RENDERS: AtomicUsize = AtomicUsize::new(0);
        fn render() -> String {
            RENDERS.fetch_add(1, Ordering::Relaxed);
            "Deferred help".to_owned()
        }
        const SPECS: &[OptionSpec] = &[OptionSpec::flag(
            "all",
            &['a'],
            &["all"],
            "deliberately-absent-translation-key",
        )
        .help_with(render)];
        let mut parser = Parser::new([OsString::from("-a")].into_iter(), SPECS, Policy::gnu());
        assert!(matches!(
            parser.next(),
            Some(Ok(Event::Option(Occurrence { id: "all", .. })))
        ));
        assert_eq!(RENDERS.load(Ordering::Relaxed), 0);
        assert_eq!(
            SPECS[0].arg().get_help().unwrap().to_string(),
            "Deferred help"
        );
        assert_eq!(RENDERS.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn clap_projection_preserves_schema_spellings_and_arity() {
        let app = clap::Command::new("scanner")
            .disable_help_flag(true)
            .disable_version_flag(true)
            .args(OPTIONS.iter().map(OptionSpec::arg));
        let matches = app
            .try_get_matches_from(["scanner", "--everything", "-o", "output", "--colour=never"])
            .unwrap();
        assert!(matches.get_flag("all"));
        assert_eq!(
            matches.get_one::<String>("output").map(String::as_str),
            Some("output")
        );
        assert_eq!(
            matches.get_one::<String>("color").map(String::as_str),
            Some("never")
        );
    }

    #[cfg(unix)]
    #[test]
    fn unix_raw_values_and_operands_survive_every_value_spelling() {
        use std::os::unix::ffi::OsStringExt;
        let raw = OsString::from_vec(b"raw_\xff".to_vec());
        for prefix in ["-o", "--output="] {
            let mut attached = OsString::from(prefix);
            attached.push(&raw);
            let events = Parser::new([attached, raw.clone()].into_iter(), OPTIONS, Policy::gnu())
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(options(&events), [("output", Some(raw.as_os_str()))]);
            assert_eq!(operands(&events), [raw.as_os_str()]);
        }
        let events = Parser::new(
            [OsString::from("--output"), raw.clone(), raw.clone()].into_iter(),
            OPTIONS,
            Policy::posix(),
        )
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
        assert_eq!(options(&events), [("output", Some(raw.as_os_str()))]);
        assert_eq!(operands(&events), [raw.as_os_str()]);
    }

    #[cfg(windows)]
    #[test]
    fn windows_unpaired_surrogates_survive_values_and_operands() {
        use std::os::windows::ffi::OsStringExt;
        let raw = OsString::from_wide(&[0x64, 0xd834, 0x78]);
        for prefix in ["-o", "--output="] {
            let mut attached = OsString::from(prefix);
            attached.push(&raw);
            let events = Parser::new([attached, raw.clone()].into_iter(), OPTIONS, Policy::gnu())
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(options(&events), [("output", Some(raw.as_os_str()))]);
            assert_eq!(operands(&events), [raw.as_os_str()]);
        }
    }

    #[test]
    fn reproducible_clusters_preserve_every_occurrence() {
        let mut seed = 0x91c4_712b_u32;
        for _ in 0..128 {
            let mut cluster = String::from("-");
            let mut expected = Vec::new();
            for _ in 0..12 {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let (short, id) = if seed & 0x1000 == 0 {
                    ('a', "all")
                } else {
                    ('b', "brief")
                };
                cluster.push(short);
                expected.push((id, None));
            }
            let events = parse(&[&cluster], Policy::gnu()).unwrap();
            assert_eq!(options(&events), expected);
        }
    }
}
