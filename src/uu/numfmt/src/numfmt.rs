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
use uucore::display::Quotable;
use uucore::error::UResult;
use uucore::format_usage;
use uucore::ranges::Range;

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

fn parse_options(args: &ArgMatches) -> Result<NumfmtOptions> {
    let from = parse_unit(args.value_of(options::FROM).unwrap())?;
    let to = parse_unit(args.value_of(options::TO).unwrap())?;

    let transform = TransformOptions { from, to };

    let padding = match args.value_of(options::PADDING) {
        Some(s) => s.parse::<isize>().map_err(|err| err.to_string()),
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
    })
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

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
                .help("replace the numbers in these input fields (default=1) see FIELDS below")
                .value_name("FIELDS")
                .default_value(options::FIELD_DEFAULT),
        )
        .arg(
            Arg::new(options::FROM)
                .long(options::FROM)
                .help("auto-scale input numbers to UNITs; see UNIT below")
                .value_name("UNIT")
                .default_value(options::FROM_DEFAULT),
        )
        .arg(
            Arg::new(options::TO)
                .long(options::TO)
                .help("auto-scale output numbers to UNITs; see UNIT below")
                .value_name("UNIT")
                .default_value(options::TO_DEFAULT),
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
                    down, from-zero (default), towards-zero, nearest",
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
    use super::{handle_buffer, NumfmtOptions, Range, RoundMethod, TransformOptions, Unit};
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
                to: Unit::None,
            },
            padding: 10,
            header: 1,
            fields: vec![Range { low: 0, high: 1 }],
            delimiter: None,
            round: RoundMethod::Nearest,
            suffix: None,
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
}
