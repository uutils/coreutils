//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Yury Krivopalov <ykrivopalov@yandex.ru>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore N'th M'th

use crate::errors::*;
use crate::format::format_and_print;
use crate::options::*;
use crate::units::{Result, Unit};
use clap::{crate_version, Arg, ArgMatches, Command};
use std::io::{BufRead, Write};
use units::{IEC_BASES, SI_BASES};
use uucore::display::Quotable;
use uucore::error::UResult;
use uucore::ranges::Range;
use uucore::{format_usage, InvalidEncodingHandling};

pub mod errors;
pub mod format;
pub mod options;
mod units;

static ABOUT: &str = "Convert numbers from/to human-readable strings";
static LONG_HELP: &str = "UNIT options:
   none   no auto-scaling is done; suffixes will trigger an error

   auto   accept optional single/two letter suffix:

          1K = 1000, 1Ki = 1024, 1M = 1000000, 1Mi = 1048576,

   si     accept optional single letter suffix:

          1K = 1000, 1M = 1000000, ...

   iec    accept optional single letter suffix:

          1K = 1024, 1M = 1048576, ...

   iec-i  accept optional two-letter suffix:

          1Ki = 1024, 1Mi = 1048576, ...

FIELDS supports cut(1) style field ranges:
  N    N'th field, counted from 1
  N-   from N'th field, to end of line
  N-M  from N'th to M'th field (inclusive)
  -M   from first to M'th field (inclusive)
  -    all fields
Multiple fields/ranges can be separated with commas

FORMAT must be suitable for printing one floating-point argument '%f'.
Optional quote (%'f) will enable --grouping (if supported by current locale).
Optional width value (%10f) will pad output. Optional zero (%010f) width
will zero pad the number. Optional negative values (%-10f) will left align.
Optional precision (%.1f) will override the input determined precision.
";
const USAGE: &str = "{} [OPTION]... [NUMBER]...";

fn handle_args<'a>(args: impl Iterator<Item = &'a str>, options: &NumfmtOptions) -> UResult<()> {
    for l in args {
        match format_and_print(l, options) {
            Ok(_) => Ok(()),
            Err(e) => Err(NumfmtError::FormattingError(e.to_string())),
        }?;
    }

    Ok(())
}

fn handle_buffer<R>(input: R, options: &NumfmtOptions) -> UResult<()>
where
    R: BufRead,
{
    let mut lines = input.lines();
    for (idx, line) in lines.by_ref().enumerate() {
        match line {
            Ok(l) if idx < options.header => {
                println!("{}", l);
                Ok(())
            }
            Ok(l) => match format_and_print(&l, options) {
                Ok(_) => Ok(()),
                Err(e) => Err(NumfmtError::FormattingError(e.to_string())),
            },
            Err(e) => Err(NumfmtError::IoError(e.to_string())),
        }?;
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
    let from = parse_unit(args.value_of(options::FROM).unwrap())?;
    let to = parse_unit(args.value_of(options::TO).unwrap())?;
    let from_unit = parse_unit_size(args.value_of(options::FROM_UNIT).unwrap())?;
    let to_unit = parse_unit_size(args.value_of(options::TO_UNIT).unwrap())?;

    let transform = TransformOptions {
        from,
        from_unit,
        to,
        to_unit,
    };

    let padding = match args.value_of(options::PADDING) {
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

    let header = match args.occurrences_of(options::HEADER) {
        0 => Ok(0),
        _ => {
            let value = args.value_of(options::HEADER).unwrap();

            value
                .parse::<usize>()
                .map_err(|_| value)
                .and_then(|n| match n {
                    0 => Err(value),
                    _ => Ok(n),
                })
                .map_err(|value| format!("invalid header value {}", value.quote()))
        }
    }?;

    let fields = match args.value_of(options::FIELD).unwrap() {
        "-" => vec![Range {
            low: 1,
            high: std::usize::MAX,
        }],
        v => Range::from_list(v)?,
    };

    let format = match args.value_of(options::FORMAT) {
        Some(s) => s.parse()?,
        None => FormatOptions::default(),
    };

    if format.grouping && to != Unit::None {
        return Err("grouping cannot be combined with --to".to_string());
    }

    let delimiter = args.value_of(options::DELIMITER).map_or(Ok(None), |arg| {
        if arg.len() == 1 {
            Ok(Some(arg.to_string()))
        } else {
            Err("the delimiter must be a single character".to_string())
        }
    })?;

    // unwrap is fine because the argument has a default value
    let round = match args.value_of(options::ROUND).unwrap() {
        "up" => RoundMethod::Up,
        "down" => RoundMethod::Down,
        "from-zero" => RoundMethod::FromZero,
        "towards-zero" => RoundMethod::TowardsZero,
        "nearest" => RoundMethod::Nearest,
        _ => unreachable!("Should be restricted by clap"),
    };

    let suffix = args.value_of(options::SUFFIX).map(|s| s.to_owned());

    Ok(NumfmtOptions {
        transform,
        padding,
        header,
        fields,
        delimiter,
        round,
        suffix,
        format,
    })
}

// If the --format argument and its value are provided separately, they are concatenated to avoid a
// potential clap error. For example: "--format --%f--" is changed to "--format=--%f--".
fn concat_format_arg_and_value(args: &[String]) -> Vec<String> {
    let mut processed_args: Vec<String> = Vec::with_capacity(args.len());
    let mut iter = args.iter().peekable();

    while let Some(arg) = iter.next() {
        if arg == "--format" && iter.peek().is_some() {
            processed_args.push(format!("--format={}", iter.peek().unwrap()));
            iter.next();
        } else {
            processed_args.push(arg.to_string());
        }
    }

    processed_args
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();

    let matches = uu_app().get_matches_from(concat_format_arg_and_value(&args));

    let options = parse_options(&matches).map_err(NumfmtError::IllegalArgument)?;

    let result = match matches.values_of(options::NUMBER) {
        Some(values) => handle_args(values, &options),
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

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(LONG_HELP)
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
                .default_value(options::FIELD_DEFAULT),
        )
        .arg(
            Arg::new(options::FORMAT)
                .long(options::FORMAT)
                .help("use printf style floating-point FORMAT; see FORMAT below for details")
                .takes_value(true)
                .value_name("FORMAT"),
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
                .value_name("N")
                .default_missing_value(options::HEADER_DEFAULT)
                .hide_default_value(true),
        )
        .arg(
            Arg::new(options::ROUND)
                .long(options::ROUND)
                .help(
                    "use METHOD for rounding when scaling; METHOD can be: up,\
                    down, from-zero, towards-zero, nearest",
                )
                .value_name("METHOD")
                .default_value("from-zero")
                .possible_values(&["up", "down", "from-zero", "towards-zero", "nearest"]),
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
            Arg::new(options::NUMBER)
                .hide(true)
                .multiple_occurrences(true),
        )
}

#[cfg(test)]
mod tests {
    use super::{
        handle_buffer, parse_unit_size, parse_unit_size_suffix, FormatOptions, NumfmtOptions,
        Range, RoundMethod, TransformOptions, Unit,
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
        }
    }

    #[test]
    fn broken_buffer_returns_io_error() {
        let mock_buffer = MockBuffer {};
        let result = handle_buffer(BufReader::new(mock_buffer), &get_valid_options())
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
        let result_debug = format!("{:?}", result);
        let result_display = format!("{}", result);
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
