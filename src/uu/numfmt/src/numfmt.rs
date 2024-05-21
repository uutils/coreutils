// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use crate::errors::*;
use crate::format::format_and_print;
use crate::options::*;
use crate::units::{Result, Unit};
use clap::{crate_version, parser::ValueSource, Arg, ArgAction, ArgMatches, Command};
use std::io::{BufRead, Write};
use std::str::FromStr;

use units::{IEC_BASES, SI_BASES};
use uucore::display::Quotable;
use uucore::error::UResult;
use uucore::ranges::Range;
use uucore::shortcut_value_parser::ShortcutValueParser;
use uucore::{format_usage, help_about, help_section, help_usage, show, show_error};

pub mod errors;
pub mod format;
pub mod options;
mod units;

const ABOUT: &str = help_about!("numfmt.md");
const AFTER_HELP: &str = help_section!("after help", "numfmt.md");
const USAGE: &str = help_usage!("numfmt.md");

fn handle_args<'a>(args: impl Iterator<Item = &'a str>, options: &NumfmtOptions) -> UResult<()> {
    for l in args {
        format_and_handle_validation(l, options)?;
    }
    Ok(())
}

fn handle_buffer<R>(input: R, options: &NumfmtOptions) -> UResult<()>
where
    R: BufRead,
{
    for (idx, line_result) in input.lines().by_ref().enumerate() {
        match line_result {
            Ok(line) if idx < options.header => {
                println!("{line}");
                Ok(())
            }
            Ok(line) => format_and_handle_validation(line.as_ref(), options),
            Err(err) => return Err(Box::new(NumfmtError::IoError(err.to_string()))),
        }?;
    }
    Ok(())
}

fn format_and_handle_validation(input_line: &str, options: &NumfmtOptions) -> UResult<()> {
    let handled_line = format_and_print(input_line, options);

    if let Err(error_message) = handled_line {
        match options.invalid {
            InvalidModes::Abort => {
                return Err(Box::new(NumfmtError::FormattingError(error_message)));
            }
            InvalidModes::Fail => {
                show!(NumfmtError::FormattingError(error_message));
            }
            InvalidModes::Warn => {
                show_error!("{}", error_message);
            }
            InvalidModes::Ignore => {}
        };
        println!("{}", input_line);
    }

    Ok(())
}

fn parse_unit(s: &str) -> Result<Unit> {
    match s {
        "auto" => Ok(Unit::Auto),
        "si" => Ok(Unit::Si),
        "iec" => Ok(Unit::Iec(false)),
        "iec-i" => Ok(Unit::Iec(true)),
        "none" => Ok(Unit::None),
        _ => Err("Unsupported unit is specified".to_owned()),
    }
}

// Parses a unit size. Suffixes are turned into their integer representations. For example, 'K'
// will return `Ok(1000)`, and '2K' will return `Ok(2000)`.
fn parse_unit_size(s: &str) -> Result<usize> {
    let number: String = s.chars().take_while(char::is_ascii_digit).collect();
    let suffix = &s[number.len()..];

    if number.is_empty() || "0".repeat(number.len()) != number {
        if let Some(multiplier) = parse_unit_size_suffix(suffix) {
            if number.is_empty() {
                return Ok(multiplier);
            }

            if let Ok(n) = number.parse::<usize>() {
                return Ok(n * multiplier);
            }
        }
    }

    Err(format!("invalid unit size: {}", s.quote()))
}

// Parses a suffix of a unit size and returns the corresponding multiplier. For example,
// the suffix 'K' will return `Some(1000)`, and 'Ki' will return `Some(1024)`.
//
// If the suffix is empty, `Some(1)` is returned.
//
// If the suffix is unknown, `None` is returned.
fn parse_unit_size_suffix(s: &str) -> Option<usize> {
    if s.is_empty() {
        return Some(1);
    }

    let suffix = s.chars().next().unwrap();

    if let Some(i) = ['K', 'M', 'G', 'T', 'P', 'E']
        .iter()
        .position(|&ch| ch == suffix)
    {
        return match s.len() {
            1 => Some(SI_BASES[i + 1] as usize),
            2 if s.ends_with('i') => Some(IEC_BASES[i + 1] as usize),
            _ => None,
        };
    }

    None
}

fn parse_options(args: &ArgMatches) -> Result<NumfmtOptions> {
    let from = parse_unit(args.get_one::<String>(options::FROM).unwrap())?;
    let to = parse_unit(args.get_one::<String>(options::TO).unwrap())?;
    let from_unit = parse_unit_size(args.get_one::<String>(options::FROM_UNIT).unwrap())?;
    let to_unit = parse_unit_size(args.get_one::<String>(options::TO_UNIT).unwrap())?;

    let transform = TransformOptions {
        from,
        from_unit,
        to,
        to_unit,
    };

    let padding = match args.get_one::<String>(options::PADDING) {
        Some(s) => s
            .parse::<isize>()
            .map_err(|_| s)
            .and_then(|n| match n {
                0 => Err(s),
                _ => Ok(n),
            })
            .map_err(|s| format!("invalid padding value {}", s.quote())),
        None => Ok(0),
    }?;

    let header = if args.value_source(options::HEADER) == Some(ValueSource::CommandLine) {
        let value = args.get_one::<String>(options::HEADER).unwrap();

        value
            .parse::<usize>()
            .map_err(|_| value)
            .and_then(|n| match n {
                0 => Err(value),
                _ => Ok(n),
            })
            .map_err(|value| format!("invalid header value {}", value.quote()))
    } else {
        Ok(0)
    }?;

    let fields = args.get_one::<String>(options::FIELD).unwrap().as_str();
    // a lone "-" means "all fields", even as part of a list of fields
    let fields = if fields.split(&[',', ' ']).any(|x| x == "-") {
        vec![Range {
            low: 1,
            high: std::usize::MAX,
        }]
    } else {
        Range::from_list(fields)?
    };

    let format = match args.get_one::<String>(options::FORMAT) {
        Some(s) => s.parse()?,
        None => FormatOptions::default(),
    };

    if format.grouping && to != Unit::None {
        return Err("grouping cannot be combined with --to".to_string());
    }

    let delimiter = args
        .get_one::<String>(options::DELIMITER)
        .map_or(Ok(None), |arg| {
            if arg.len() == 1 {
                Ok(Some(arg.to_string()))
            } else {
                Err("the delimiter must be a single character".to_string())
            }
        })?;

    // unwrap is fine because the argument has a default value
    let round = match args.get_one::<String>(options::ROUND).unwrap().as_str() {
        "up" => RoundMethod::Up,
        "down" => RoundMethod::Down,
        "from-zero" => RoundMethod::FromZero,
        "towards-zero" => RoundMethod::TowardsZero,
        "nearest" => RoundMethod::Nearest,
        _ => unreachable!("Should be restricted by clap"),
    };

    let suffix = args.get_one::<String>(options::SUFFIX).cloned();

    let invalid =
        InvalidModes::from_str(args.get_one::<String>(options::INVALID).unwrap()).unwrap();

    Ok(NumfmtOptions {
        transform,
        padding,
        header,
        fields,
        delimiter,
        round,
        suffix,
        format,
        invalid,
    })
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let options = parse_options(&matches).map_err(NumfmtError::IllegalArgument)?;

    let result = match matches.get_many::<String>(options::NUMBER) {
        Some(values) => handle_args(values.map(|s| s.as_str()), &options),
        None => {
            let stdin = std::io::stdin();
            let mut locked_stdin = stdin.lock();
            handle_buffer(&mut locked_stdin, &options)
        }
    };

    match result {
        Err(e) => {
            std::io::stdout().flush().expect("error flushing stdout");
            Err(e)
        }
        _ => Ok(()),
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .override_usage(format_usage(USAGE))
        .allow_negative_numbers(true)
        .infer_long_args(true)
        .arg(
            Arg::new(options::DELIMITER)
                .short('d')
                .long(options::DELIMITER)
                .value_name("X")
                .help("use X instead of whitespace for field delimiter"),
        )
        .arg(
            Arg::new(options::FIELD)
                .long(options::FIELD)
                .help("replace the numbers in these input fields; see FIELDS below")
                .value_name("FIELDS")
                .allow_hyphen_values(true)
                .default_value(options::FIELD_DEFAULT),
        )
        .arg(
            Arg::new(options::FORMAT)
                .long(options::FORMAT)
                .help("use printf style floating-point FORMAT; see FORMAT below for details")
                .value_name("FORMAT")
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new(options::FROM)
                .long(options::FROM)
                .help("auto-scale input numbers to UNITs; see UNIT below")
                .value_name("UNIT")
                .default_value(options::FROM_DEFAULT),
        )
        .arg(
            Arg::new(options::FROM_UNIT)
                .long(options::FROM_UNIT)
                .help("specify the input unit size")
                .value_name("N")
                .default_value(options::FROM_UNIT_DEFAULT),
        )
        .arg(
            Arg::new(options::TO)
                .long(options::TO)
                .help("auto-scale output numbers to UNITs; see UNIT below")
                .value_name("UNIT")
                .default_value(options::TO_DEFAULT),
        )
        .arg(
            Arg::new(options::TO_UNIT)
                .long(options::TO_UNIT)
                .help("the output unit size")
                .value_name("N")
                .default_value(options::TO_UNIT_DEFAULT),
        )
        .arg(
            Arg::new(options::PADDING)
                .long(options::PADDING)
                .help(
                    "pad the output to N characters; positive N will \
                     right-align; negative N will left-align; padding is \
                     ignored if the output is wider than N; the default is \
                     to automatically pad if a whitespace is found",
                )
                .value_name("N"),
        )
        .arg(
            Arg::new(options::HEADER)
                .long(options::HEADER)
                .help(
                    "print (without converting) the first N header lines; \
                     N defaults to 1 if not specified",
                )
                .num_args(..=1)
                .value_name("N")
                .default_missing_value(options::HEADER_DEFAULT)
                .hide_default_value(true),
        )
        .arg(
            Arg::new(options::ROUND)
                .long(options::ROUND)
                .help("use METHOD for rounding when scaling")
                .value_name("METHOD")
                .default_value("from-zero")
                .value_parser(ShortcutValueParser::new([
                    "up",
                    "down",
                    "from-zero",
                    "towards-zero",
                    "nearest",
                ])),
        )
        .arg(
            Arg::new(options::SUFFIX)
                .long(options::SUFFIX)
                .help(
                    "print SUFFIX after each formatted number, and accept \
                    inputs optionally ending with SUFFIX",
                )
                .value_name("SUFFIX"),
        )
        .arg(
            Arg::new(options::INVALID)
                .long(options::INVALID)
                .help("set the failure mode for invalid input")
                .default_value("abort")
                .value_parser(["abort", "fail", "warn", "ignore"])
                .value_name("INVALID"),
        )
        .arg(
            Arg::new(options::NUMBER)
                .hide(true)
                .action(ArgAction::Append),
        )
}

#[cfg(test)]
mod tests {
    use uucore::error::get_exit_code;

    use super::{
        handle_args, handle_buffer, parse_unit_size, parse_unit_size_suffix, FormatOptions,
        InvalidModes, NumfmtOptions, Range, RoundMethod, TransformOptions, Unit,
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
            format: FormatOptions::default(),
            invalid: InvalidModes::Abort,
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
        let result_debug = format!("{:?}", result);
        let result_display = format!("{}", result);
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
            "FormattingError(\"invalid suffix in input: 'hello'\")"
        );
        assert_eq!(result_display, "invalid suffix in input: 'hello'");
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
        assert!(
            get_exit_code() == 2,
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
        let input_value = ["5", "4Q"].into_iter();
        let mut options = get_valid_options();
        options.invalid = InvalidModes::Fail;
        handle_args(input_value, &options).unwrap();
        assert!(
            get_exit_code() == 2,
            "should set exit code 2 for formatting errors"
        );
    }

    #[test]
    fn args_warn_returns_status_0_for_invalid_input() {
        let input_value = ["5", "4Q"].into_iter();
        let mut options = get_valid_options();
        options.invalid = InvalidModes::Warn;
        let result = handle_args(input_value, &options);
        assert!(result.is_ok(), "did not return ok for invalid input");
    }

    #[test]
    fn test_parse_unit_size() {
        assert_eq!(1, parse_unit_size("1").unwrap());
        assert_eq!(1, parse_unit_size("01").unwrap());
        assert!(parse_unit_size("1.1").is_err());
        assert!(parse_unit_size("0").is_err());
        assert!(parse_unit_size("-1").is_err());
        assert!(parse_unit_size("A").is_err());
        assert!(parse_unit_size("18446744073709551616").is_err());
    }

    #[test]
    fn test_parse_unit_size_with_suffix() {
        assert_eq!(1000, parse_unit_size("K").unwrap());
        assert_eq!(1024, parse_unit_size("Ki").unwrap());
        assert_eq!(2000, parse_unit_size("2K").unwrap());
        assert_eq!(2048, parse_unit_size("2Ki").unwrap());
        assert!(parse_unit_size("0K").is_err());
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
