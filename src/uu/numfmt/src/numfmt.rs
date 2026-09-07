// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore behavior

use crate::errors::NumfmtError;
use crate::format::{escape_line, write_formatted_with_delimiter, write_formatted_with_whitespace};
use crate::options::{
    DEBUG, DELIMITER, FIELD, FIELD_DEFAULT, FORMAT, FROM, FROM_DEFAULT, FROM_UNIT,
    FROM_UNIT_DEFAULT, FormatOptions, GROUPING, HEADER, HEADER_DEFAULT, INVALID, InvalidModes,
    NUMBER, NumfmtOptions, OptionValueError, PADDING, ParseError, ROUND, RoundMethod, SUFFIX, TO,
    TO_DEFAULT, TO_UNIT, TO_UNIT_DEFAULT, TransformOptions, UNIT_SEPARATOR, ZERO_TERMINATED,
};
use crate::units::{Result, Unit};
use clap::{Arg, ArgAction, ArgMatches, Command, builder::ValueParser, parser::ValueSource};
use std::ffi::OsString;
use std::io::{BufRead, BufWriter, IsTerminal, Write, stderr};
use std::str::FromStr;

use uucore::display::Quotable;
use uucore::error::UResult;
use uucore::i18n::decimal::locale_grouping_separator;
use uucore::parser::parse_size::{IEC_BASES, SI_BASES};
use uucore::parser::shortcut_value_parser::ShortcutValueParser;
use uucore::ranges::Range;
use uucore::{format_usage, os_str_as_bytes, show, translate};

pub mod errors;
pub mod format;
pub mod options;

mod diagnostics;
mod numeric;
mod units;

// Returns `true` if the input is in scientific notation
fn is_scientific(input: &[u8]) -> bool {
    input
        .iter()
        .position(|b| b"Ee".contains(b))
        .is_some_and(|pos| input.get(pos + 1).is_some_and(u8::is_ascii_digit))
}

/// Format a single line and write it, handling `--invalid` error modes.
///
/// Returns `true` if the line contained invalid input (only possible in
/// non-abort modes).
fn format_and_write(
    writer: &mut dyn Write,
    input_line: &[u8],
    options: &NumfmtOptions,
    eol: Option<u8>,
) -> UResult<bool> {
    // GNU truncates at the first embedded null byte.
    let line = match memchr::memchr(b'\0', input_line) {
        Some(i) => &input_line[..i],
        None => input_line,
    };

    // In non-abort modes we buffer the formatted output so that on error we
    // can emit the original line instead.
    let buffer_output = !matches!(options.invalid, InvalidModes::Abort);
    let mut buf = Vec::new();
    let dest: &mut dyn Write = if buffer_output { &mut buf } else { writer };

    let result = if options.delimiter.is_some() {
        write_formatted_with_delimiter(dest, line, options, eol)
    } else {
        match std::str::from_utf8(line) {
            Ok(s) => {
                if is_scientific(s.as_bytes()) {
                    Err(format!(
                        "invalid suffix in input: '{}'",
                        String::from_utf8_lossy(line)
                    ))
                } else {
                    write_formatted_with_whitespace(dest, s, options, eol)
                }
            }
            Err(_) => Err(translate!(
                "numfmt-error-invalid-number",
                "input" => escape_line(line).quote()
            )),
        }
    };

    if let Err(msg) = result {
        match options.invalid {
            InvalidModes::Abort => {
                return Err(Box::new(NumfmtError::FormattingError(msg)));
            }
            InvalidModes::Fail => {
                show!(NumfmtError::FormattingError(msg));
            }
            InvalidModes::Warn => {
                let _ = writeln!(stderr(), "numfmt: {msg}");
            }
            InvalidModes::Ignore => {}
        }
        // On error, echo the original line unchanged.
        writer.write_all(input_line)?;
        if let Some(eol) = eol {
            writer.write_all(&[eol])?;
        }
        return Ok(true);
    }

    if buffer_output {
        writer.write_all(&buf)?;
    }
    Ok(false)
}

/// Run `body` with stdout buffered the way stdio buffers it: a block at a time
/// into a file or a pipe, where only the total number of writes matters, and a
/// line at a time onto a terminal, where output is read as it is produced.
///
/// Whatever `body` wrote is flushed before this returns, so an error still
/// leaves the lines that came before it on stdout.
fn with_stdout<T>(body: impl FnOnce(&mut dyn Write) -> UResult<T>) -> UResult<T> {
    let stdout = std::io::stdout();
    if stdout.is_terminal() {
        let mut writer = stdout.lock();
        finish(body(&mut writer), &mut writer)
    } else {
        let mut writer = BufWriter::new(stdout.lock());
        finish(body(&mut writer), &mut writer)
    }
}

/// Flush `writer`, keeping the failure `result` already carries, if any.
fn finish<T>(result: UResult<T>, writer: &mut impl Write) -> UResult<T> {
    let flushed = writer
        .flush()
        .map_err(|e| NumfmtError::IoError(e.to_string()));
    let value = result?;
    flushed?;
    Ok(value)
}

/// Process command-line number arguments.
///
/// `snapshot` is the command line as typed, for the caret under a number that
/// does not convert, and `None` when diagnostics are off.
///
/// Returns `true` if any line contained invalid input.
fn handle_args<'a>(
    args: impl Iterator<Item = &'a [u8]>,
    options: &NumfmtOptions,
    snapshot: Option<&[OsString]>,
) -> UResult<bool> {
    with_stdout(|stdout| {
        let terminator = if options.zero_terminated { 0u8 } else { b'\n' };
        let mut saw_invalid = false;
        for (n, l) in args.enumerate() {
            match format_and_write(stdout, l, options, Some(terminator)) {
                Ok(invalid) => saw_invalid |= invalid,
                // Only this mode stops on the first bad number; the others carry
                // on, where a report per line would bury the output.
                Err(error) => {
                    return Err(uucore::diagnostics::error_after_report(
                        snapshot,
                        error,
                        |args, error| {
                            diagnostics::render_input(args, l, n, &error.to_string(), options)
                        },
                    ));
                }
            }
        }
        Ok(saw_invalid)
    })
}

/// Process lines read from stdin.
///
/// Returns `true` if any line contained invalid input.
fn handle_buffer<R: BufRead>(input: R, options: &NumfmtOptions) -> UResult<bool> {
    with_stdout(|stdout| handle_buffer_to(input, options, stdout))
}

fn handle_buffer_to<R: BufRead>(
    mut input: R,
    options: &NumfmtOptions,
    stdout: &mut dyn Write,
) -> UResult<bool> {
    let terminator = if options.zero_terminated { 0u8 } else { b'\n' };
    let mut buf = Vec::new();
    let mut line_idx = 0;
    let mut saw_invalid = false;

    loop {
        buf.clear();
        let n = input
            .read_until(terminator, &mut buf)
            .map_err(|e| NumfmtError::IoError(e.to_string()))?;
        if n == 0 {
            break;
        }

        let has_terminator = buf.last() == Some(&terminator);
        let line = if has_terminator {
            &buf[..buf.len() - 1]
        } else {
            &buf[..]
        };
        // Emit the terminator only when the input line had one (preserve
        // missing final newline).
        let eol = has_terminator.then_some(terminator);

        if line_idx < options.header {
            // Pass header lines through unchanged.
            stdout.write_all(line)?;
            if let Some(t) = eol {
                stdout.write_all(&[t])?;
            }
        } else {
            saw_invalid |= format_and_write(stdout, line, options, eol)?;
        }

        line_idx += 1;
    }

    Ok(saw_invalid)
}

fn parse_unit(s: &str, opt: &'static str) -> std::result::Result<Unit, ParseError> {
    match s {
        "auto" if opt != TO => Ok(Unit::Auto),
        "si" => Ok(Unit::Si),
        "iec" => Ok(Unit::Iec(false)),
        "iec-i" => Ok(Unit::Iec(true)),
        "none" => Ok(Unit::None),
        value => Err(OptionValueError {
            message: translate!("numfmt-error-invalid-unit-argument", "arg" => value, "opt" => format!("--{opt}")),
            option: opt,
            value: value.to_string(),
            // `auto` is a real unit, just not one --to can scale to.
            label: (value == "auto").then_some("numfmt-diag-label-auto-from-only"),
            help: "numfmt-diag-help-unit",
        }
        .into()),
    }
}

/// Parses a unit size. Suffixes are turned into their integer representations. For example, 'K'
/// will return `Ok(1000)`, and '2K' will return `Ok(2000)`.
fn parse_unit_size(s: &str, opt: &'static str) -> std::result::Result<usize, ParseError> {
    let split = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    let (number, suffix) = s.split_at(split);

    // Reject all-zero numeric parts like "0" or "00K".
    let all_zero = !number.is_empty() && number.bytes().all(|b| b == b'0');
    if !all_zero && let Some(multiplier) = parse_unit_size_suffix(suffix) {
        if number.is_empty() {
            return Ok(multiplier);
        }
        if let Some(size) = number
            .parse::<usize>()
            .ok()
            .and_then(|n| n.checked_mul(multiplier))
        {
            return Ok(size);
        }
    }

    Err(OptionValueError {
        message: translate!("numfmt-error-invalid-unit-size", "size" => s.quote()),
        option: opt,
        value: s.to_string(),
        // A zero size is spelled like a valid one; the rest the help covers.
        // An unknown suffix is what is wrong about "0x", not the zero.
        label: (all_zero && parse_unit_size_suffix(suffix).is_some())
            .then_some("numfmt-diag-label-zero-unit-size"),
        help: "numfmt-diag-help-unit-size",
    }
    .into())
}

/// Parses a suffix of a unit size and returns the corresponding multiplier. For example,
/// the suffix 'K' will return `Some(1000)`, and 'Ki' will return `Some(1024)`.
///
/// If the suffix is empty, `Some(1)` is returned.
///
/// If the suffix is unknown, `None` is returned.
fn parse_unit_size_suffix(s: &str) -> Option<usize> {
    if s.is_empty() {
        return Some(1);
    }

    let i = ['K', 'M', 'G', 'T', 'P', 'E']
        .iter()
        .position(|&ch| s.starts_with(ch))?;

    match s.len() {
        1 => Some(SI_BASES[i + 1] as usize),
        2 if s.ends_with('i') => Some(IEC_BASES[i + 1] as usize),
        _ => None,
    }
}

/// Parse delimiter argument, ensuring it's a single character.
/// For non-UTF8 locales, we allow up to 4 bytes (max UTF-8 char length).
fn parse_delimiter(arg: &OsString) -> Result<Vec<u8>> {
    let bytes = os_str_as_bytes(arg).map_err(|e| e.to_string())?;
    // TODO: Cut, NL and here need to find a better way to do locale specific character count
    if arg.to_str().is_some_and(|s| s.chars().count() > 1)
        || (arg.to_str().is_none() && bytes.len() > 4)
    {
        Err(translate!(
            "numfmt-error-delimiter-must-be-single-character"
        ))
    } else {
        Ok(bytes.to_vec())
    }
}

fn parse_options(args: &ArgMatches) -> std::result::Result<NumfmtOptions, ParseError> {
    let from = parse_unit(args.get_one::<String>(FROM).unwrap(), FROM)?;
    let to = parse_unit(args.get_one::<String>(TO).unwrap(), TO)?;
    let from_unit = parse_unit_size(args.get_one::<String>(FROM_UNIT).unwrap(), FROM_UNIT)?;
    let to_unit = parse_unit_size(args.get_one::<String>(TO_UNIT).unwrap(), TO_UNIT)?;

    let transform = TransformOptions {
        from,
        from_unit,
        to,
        to_unit,
    };

    let padding = match args.get_one::<String>(PADDING) {
        Some(s) => s
            .parse::<isize>()
            .map_err(|_| s)
            .and_then(|n| match n {
                0 => Err(s),
                _ => Ok(n),
            })
            .map_err(|s| OptionValueError {
                message: translate!("numfmt-error-invalid-padding", "value" => s.quote()),
                option: PADDING,
                value: s.clone(),
                // A zero parses fine and only fails on meaning.
                label: (s == "0").then_some("numfmt-diag-label-zero-padding"),
                help: "numfmt-diag-help-padding",
            }),
        None => Ok(0),
    }?;

    let header = if args.value_source(HEADER) == Some(ValueSource::CommandLine) {
        let value = args.get_one::<String>(HEADER).unwrap();

        value
            .parse::<usize>()
            .map_err(|_| value)
            .and_then(|n| match n {
                0 => Err(value),
                _ => Ok(n),
            })
            .map_err(|value| OptionValueError {
                message: translate!("numfmt-error-invalid-header", "value" => value.quote()),
                option: HEADER,
                value: value.clone(),
                label: (value == "0").then_some("numfmt-diag-label-zero-header"),
                help: "numfmt-diag-help-header",
            })
    } else {
        Ok(0)
    }?;

    let fields = args.get_one::<String>(FIELD).unwrap().as_str();
    // a lone "-" means "all fields", even as part of a list of fields
    let fields = if fields.split(&[',', ' ']).any(|x| x == "-") {
        vec![Range {
            low: 1,
            high: usize::MAX,
        }]
    } else {
        Range::from_list(fields)?
    };

    let grouping = args.get_flag(GROUPING);
    let format = match args.get_one::<String>(FORMAT) {
        Some(s) => s.parse()?,
        None => FormatOptions::default(),
    };

    if grouping && args.contains_id(FORMAT) {
        return Err(translate!("numfmt-error-grouping-cannot-be-combined-with-format").into());
    }

    let grouping = grouping || format.grouping;

    if grouping && to != Unit::None {
        return Err(translate!("numfmt-error-grouping-cannot-be-combined-with-to").into());
    }

    let delimiter = args
        .get_one::<OsString>(DELIMITER)
        .map(parse_delimiter)
        .transpose()?;

    // unwrap is fine because the argument has a default value
    let round = match args.get_one::<String>(ROUND).unwrap().as_str() {
        "up" => RoundMethod::Up,
        "down" => RoundMethod::Down,
        "from-zero" => RoundMethod::FromZero,
        "towards-zero" => RoundMethod::TowardsZero,
        "nearest" => RoundMethod::Nearest,
        _ => unreachable!("Should be restricted by clap"),
    };

    let suffix = args.get_one::<String>(SUFFIX).cloned();

    let unit_separator = args
        .get_one::<String>(UNIT_SEPARATOR)
        .cloned()
        .unwrap_or_default();

    let explicit_unit_separator = args.contains_id(UNIT_SEPARATOR)
        && args.value_source(UNIT_SEPARATOR) == Some(ValueSource::CommandLine);

    let invalid = InvalidModes::from_str(args.get_one::<String>(INVALID).unwrap()).unwrap();

    let zero_terminated = args.get_flag(ZERO_TERMINATED);

    let debug = args.get_flag(DEBUG);

    Ok(NumfmtOptions {
        transform,
        padding,
        header,
        fields,
        delimiter,
        round,
        suffix,
        unit_separator,
        grouping,
        explicit_unit_separator,
        format,
        invalid,
        zero_terminated,
        debug,
    })
}

fn print_debug_warnings(options: &NumfmtOptions, matches: &ArgMatches) {
    fn print_warning(msg_key: &str) {
        let _ = writeln!(stderr(), "numfmt: {}", translate!(msg_key));
    }

    // Warn if no conversion option is specified
    // 2>/dev/full does not abort
    if options.transform.from == Unit::None
        && options.transform.to == Unit::None
        && options.padding == 0
        && !options.grouping
    {
        print_warning("numfmt-debug-no-conversion");
    }

    if options.grouping && locale_grouping_separator().is_empty() {
        print_warning("numfmt-debug-grouping-no-effect");
    }

    // Warn if --header is used with command-line input
    if options.header > 0 && matches.get_many::<OsString>(NUMBER).is_some() {
        print_warning("numfmt-debug-header-ignored");
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args: Vec<OsString> = args.collect();
    // Kept for the caret in --format diagnostics.
    let format_args = uucore::diagnostics::capture(&args);
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;
    let options = match parse_options(&matches) {
        Ok(options) => options,
        // A format error still knows where in the format string it happened,
        // so it is the one error worth a caret.
        Err(ParseError::Format(error)) => {
            return Err(uucore::diagnostics::error_after_report(
                format_args.as_deref(),
                NumfmtError::IllegalArgument(error.message.clone()),
                |args, _| {
                    matches
                        .get_one::<String>(FORMAT)
                        .is_some_and(|format| diagnostics::render(args, format, &error))
                },
            ));
        }
        // As for a format, a field list knows which of its ranges is at fault.
        Err(ParseError::Field(error)) => {
            return Err(uucore::diagnostics::error_after_report(
                format_args.as_deref(),
                NumfmtError::IllegalArgument(error.message.clone()),
                |args, _| {
                    matches
                        .get_one::<String>(FIELD)
                        .is_some_and(|fields| diagnostics::render_field(args, fields, &error))
                },
            ));
        }
        // An option value that is wrong as a whole: underline it where typed.
        Err(ParseError::Value(error)) => {
            return Err(uucore::diagnostics::error_after_report(
                format_args.as_deref(),
                NumfmtError::IllegalArgument(error.message.clone()),
                |args, _| diagnostics::render_value(args, &error),
            ));
        }
        Err(ParseError::Other(message)) => {
            return Err(NumfmtError::IllegalArgument(message).into());
        }
    };

    if options.debug {
        print_debug_warnings(&options, &matches);
    }

    let result = if let Some(values) = matches.get_many::<OsString>(NUMBER) {
        let byte_args: Vec<&[u8]> = values
            .map(|s| os_str_as_bytes(s).map_err(|e| e.to_string()))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(NumfmtError::IllegalArgument)?;
        handle_args(byte_args.into_iter(), &options, format_args.as_deref())
    } else {
        let stdin = std::io::stdin();
        handle_buffer(stdin.lock(), &options)
    };

    match result {
        Err(e) => {
            // Flush stdout before returning the error so any partial output is
            // visible (matches GNU behavior).
            let _ = std::io::stdout().flush();
            Err(e)
        }
        Ok(saw_invalid) => {
            if options.debug && saw_invalid {
                let _ = writeln!(
                    stderr(),
                    "numfmt: {}",
                    translate!("numfmt-debug-failed-to-convert")
                );
            }
            Ok(())
        }
    }
}

pub fn uu_app() -> Command {
    Command::new("numfmt")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("numfmt"))
        .about(translate!("numfmt-about"))
        .after_help(translate!("numfmt-after-help"))
        .override_usage(format_usage(&translate!("numfmt-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(DEBUG)
                .long(DEBUG)
                .help(translate!("numfmt-help-debug"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(GROUPING)
                .long(GROUPING)
                .help(translate!("numfmt-help-grouping"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(DELIMITER)
                .short('d')
                .long(DELIMITER)
                .value_name("X")
                .value_parser(ValueParser::os_string())
                .allow_hyphen_values(true)
                .help(translate!("numfmt-help-delimiter")),
        )
        .arg(
            Arg::new(FIELD)
                .long(FIELD)
                .help(translate!("numfmt-help-field"))
                .value_name("FIELDS")
                .allow_hyphen_values(true)
                .default_value(FIELD_DEFAULT),
        )
        .arg(
            Arg::new(FORMAT)
                .long(FORMAT)
                .help(translate!("numfmt-help-format"))
                .value_name("FORMAT")
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(FROM)
                .long(FROM)
                .help(translate!("numfmt-help-from"))
                .value_name("UNIT")
                .default_value(FROM_DEFAULT)
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(FROM_UNIT)
                .long(FROM_UNIT)
                .help(translate!("numfmt-help-from-unit"))
                .value_name("N")
                .default_value(FROM_UNIT_DEFAULT)
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(TO)
                .long(TO)
                .help(translate!("numfmt-help-to"))
                .value_name("UNIT")
                .default_value(TO_DEFAULT)
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(TO_UNIT)
                .long(TO_UNIT)
                .help(translate!("numfmt-help-to-unit"))
                .value_name("N")
                .default_value(TO_UNIT_DEFAULT)
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(PADDING)
                .long(PADDING)
                .help(translate!("numfmt-help-padding"))
                .value_name("N")
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(HEADER)
                .long(HEADER)
                .help(translate!("numfmt-help-header"))
                .num_args(..=1)
                .value_name("N")
                .default_missing_value(HEADER_DEFAULT)
                .hide_default_value(true)
                .require_equals(true),
        )
        .arg(
            Arg::new(ROUND)
                .long(ROUND)
                .help(translate!("numfmt-help-round"))
                .value_name("METHOD")
                .default_value("from-zero")
                .value_parser(ShortcutValueParser::new([
                    "up",
                    "down",
                    "from-zero",
                    "towards-zero",
                    "nearest",
                ]))
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(SUFFIX)
                .long(SUFFIX)
                .help(translate!("numfmt-help-suffix"))
                .value_name("SUFFIX")
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(UNIT_SEPARATOR)
                .long(UNIT_SEPARATOR)
                .help(translate!("numfmt-help-unit-separator"))
                .value_name("STRING")
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(INVALID)
                .long(INVALID)
                .help(translate!("numfmt-help-invalid"))
                .default_value("abort")
                .value_parser(["abort", "fail", "warn", "ignore"])
                .value_name("INVALID")
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(ZERO_TERMINATED)
                .long(ZERO_TERMINATED)
                .short('z')
                .help(translate!("numfmt-help-zero-terminated"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(NUMBER)
                .hide(true)
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string()),
        )
}

#[cfg(test)]
mod tests {
    use uucore::error::get_exit_code;

    use super::{
        FROM_UNIT, FormatOptions, InvalidModes, NumfmtOptions, Range, RoundMethod,
        TransformOptions, Unit, handle_args, handle_buffer, parse_unit_size,
        parse_unit_size_suffix,
    };
    use std::io::{BufReader, Error, ErrorKind, Read};
    struct MockBuffer {}

    impl Read for MockBuffer {
        fn read(&mut self, _: &mut [u8]) -> Result<usize, Error> {
            Err(Error::new(ErrorKind::BrokenPipe, "broken pipe"))
        }
    }

    fn get_valid_options() -> NumfmtOptions {
        NumfmtOptions {
            transform: TransformOptions {
                from: Unit::None,
                from_unit: 1,
                to: Unit::None,
                to_unit: 1,
            },
            padding: 10,
            header: 1,
            fields: vec![Range { low: 0, high: 1 }],
            delimiter: None,
            round: RoundMethod::Nearest,
            suffix: None,
            unit_separator: String::new(),
            grouping: false,
            explicit_unit_separator: false,
            format: FormatOptions::default(),
            invalid: InvalidModes::Abort,
            zero_terminated: false,
            debug: false,
        }
    }

    #[test]
    fn broken_buffer_returns_io_error() {
        let mock_buffer = MockBuffer {};
        let result = handle_buffer(BufReader::new(mock_buffer), &get_valid_options())
            .expect_err("returned Ok after receiving IO error");
        let result_debug = format!("{result:?}");
        let result_display = format!("{result}");
        assert_eq!(result_debug, "IoError(\"broken pipe\")");
        assert_eq!(result_display, "broken pipe");
        assert_eq!(result.code(), 1);
    }

    #[test]
    fn broken_buffer_returns_io_error_after_header() {
        let mock_buffer = MockBuffer {};
        let mut options = get_valid_options();
        options.header = 0;
        let result = handle_buffer(BufReader::new(mock_buffer), &options)
            .expect_err("returned Ok after receiving IO error");
        let result_debug = format!("{result:?}");
        let result_display = format!("{result}");
        assert_eq!(result_debug, "IoError(\"broken pipe\")");
        assert_eq!(result_display, "broken pipe");
        assert_eq!(result.code(), 1);
    }

    #[test]
    fn non_numeric_returns_formatting_error() {
        let input_value = b"135\nhello";
        let result = handle_buffer(BufReader::new(&input_value[..]), &get_valid_options())
            .expect_err("returned Ok after receiving improperly formatted input");
        let result_debug = format!("{result:?}");
        let result_display = format!("{result}");
        assert_eq!(
            result_debug,
            "FormattingError(\"numfmt-error-invalid-number\")"
        );
        assert_eq!(result_display, "numfmt-error-invalid-number");
        assert_eq!(result.code(), 2);
    }

    #[test]
    fn valid_input_returns_ok() {
        let input_value = b"165\n100\n300\n500";
        let result = handle_buffer(BufReader::new(&input_value[..]), &get_valid_options());
        assert!(result.is_ok(), "did not return Ok for valid input");
    }

    #[test]
    fn warn_returns_ok_for_invalid_input() {
        let input_value = b"5\n4Q\n";
        let mut options = get_valid_options();
        options.invalid = InvalidModes::Warn;
        let result = handle_buffer(BufReader::new(&input_value[..]), &options);
        assert!(result.is_ok(), "did not return Ok for invalid input");
    }

    #[test]
    fn ignore_returns_ok_for_invalid_input() {
        let input_value = b"5\n4Q\n";
        let mut options = get_valid_options();
        options.invalid = InvalidModes::Ignore;
        let result = handle_buffer(BufReader::new(&input_value[..]), &options);
        assert!(result.is_ok(), "did not return Ok for invalid input");
    }

    #[test]
    fn buffer_fail_returns_status_2_for_invalid_input() {
        let input_value = b"5\n4Q\n";
        let mut options = get_valid_options();
        options.invalid = InvalidModes::Fail;
        handle_buffer(BufReader::new(&input_value[..]), &options).unwrap();
        assert_eq!(
            get_exit_code(),
            2,
            "should set exit code 2 for formatting errors"
        );
    }

    #[test]
    fn abort_returns_status_2_for_invalid_input() {
        let input_value = b"5\n4Q\n";
        let mut options = get_valid_options();
        options.invalid = InvalidModes::Abort;
        let result = handle_buffer(BufReader::new(&input_value[..]), &options);
        assert!(result.is_err(), "did not return err for invalid input");
    }

    #[test]
    fn args_fail_returns_status_2_for_invalid_input() {
        let input_value = [b"5".as_slice(), b"4Q"].into_iter();
        let mut options = get_valid_options();
        options.invalid = InvalidModes::Fail;
        handle_args(input_value, &options, None).unwrap();
        assert_eq!(
            get_exit_code(),
            2,
            "should set exit code 2 for formatting errors"
        );
    }

    #[test]
    fn args_warn_returns_status_0_for_invalid_input() {
        let input_value = [b"5".as_slice(), b"4Q"].into_iter();
        let mut options = get_valid_options();
        options.invalid = InvalidModes::Warn;
        let result = handle_args(input_value, &options, None);
        assert!(result.is_ok(), "did not return ok for invalid input");
    }

    #[test]
    fn test_parse_unit_size() {
        assert_eq!(1, parse_unit_size("1", FROM_UNIT).unwrap());
        assert_eq!(1, parse_unit_size("01", FROM_UNIT).unwrap());
        assert!(parse_unit_size("1.1", FROM_UNIT).is_err());
        assert!(parse_unit_size("0", FROM_UNIT).is_err());
        assert!(parse_unit_size("-1", FROM_UNIT).is_err());
        assert!(parse_unit_size("A", FROM_UNIT).is_err());
        assert!(parse_unit_size("18446744073709551616", FROM_UNIT).is_err());
    }

    #[test]
    fn test_parse_unit_size_with_suffix() {
        assert_eq!(1000, parse_unit_size("K", FROM_UNIT).unwrap());
        assert_eq!(1024, parse_unit_size("Ki", FROM_UNIT).unwrap());
        assert_eq!(2000, parse_unit_size("2K", FROM_UNIT).unwrap());
        assert_eq!(2048, parse_unit_size("2Ki", FROM_UNIT).unwrap());
        assert!(parse_unit_size("0K", FROM_UNIT).is_err());
    }

    #[test]
    fn test_parse_unit_size_suffix() {
        assert_eq!(1, parse_unit_size_suffix("").unwrap());
        assert_eq!(1000, parse_unit_size_suffix("K").unwrap());
        assert_eq!(1024, parse_unit_size_suffix("Ki").unwrap());
        assert_eq!(1000 * 1000, parse_unit_size_suffix("M").unwrap());
        assert_eq!(1024 * 1024, parse_unit_size_suffix("Mi").unwrap());
        assert!(parse_unit_size_suffix("Kii").is_none());
        assert!(parse_unit_size_suffix("A").is_none());
    }
}
