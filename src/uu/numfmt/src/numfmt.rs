//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Yury Krivopalov <ykrivopalov@yandex.ru>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore N'th M'th

use crate::format::format_and_print;
use crate::options::*;
use crate::units::{Result, Unit};
use clap::{crate_version, App, AppSettings, Arg, ArgMatches};
use std::io::{BufRead, Write};
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};
use uucore::ranges::Range;

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

fn usage() -> String {
    format!("{0} [OPTION]... [NUMBER]...", uucore::execution_phrase())
}

fn handle_args<'a>(args: impl Iterator<Item = &'a str>, options: NumfmtOptions) -> Result<()> {
    for l in args {
        format_and_print(l, &options)?;
    }

    Ok(())
}

fn handle_stdin(options: NumfmtOptions) -> Result<()> {
    let stdin = std::io::stdin();
    let locked_stdin = stdin.lock();

    let mut lines = locked_stdin.lines();
    for l in lines.by_ref().take(options.header) {
        l.map(|s| println!("{}", s)).map_err(|e| e.to_string())?;
    }

    for l in lines {
        l.map_err(|e| e.to_string())
            .and_then(|l| format_and_print(&l, &options))?;
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

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let usage = usage();

    let matches = uu_app().usage(&usage[..]).get_matches_from(args);

    let result =
        parse_options(&matches).and_then(|options| match matches.values_of(options::NUMBER) {
            Some(values) => handle_args(values, options),
            None => handle_stdin(options),
        });

    match result {
        Err(e) => {
            std::io::stdout().flush().expect("error flushing stdout");
            // TODO Change `handle_args()` and `handle_stdin()` so that
            // they return `UResult`.
            return Err(USimpleError::new(1, e));
        }
        _ => Ok(()),
    }
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(LONG_HELP)
        .setting(AppSettings::AllowNegativeNumbers)
        .arg(
            Arg::with_name(options::DELIMITER)
                .short("d")
                .long(options::DELIMITER)
                .value_name("X")
                .help("use X instead of whitespace for field delimiter"),
        )
        .arg(
            Arg::with_name(options::FIELD)
                .long(options::FIELD)
                .help("replace the numbers in these input fields (default=1) see FIELDS below")
                .value_name("FIELDS")
                .default_value(options::FIELD_DEFAULT),
        )
        .arg(
            Arg::with_name(options::FROM)
                .long(options::FROM)
                .help("auto-scale input numbers to UNITs; see UNIT below")
                .value_name("UNIT")
                .default_value(options::FROM_DEFAULT),
        )
        .arg(
            Arg::with_name(options::TO)
                .long(options::TO)
                .help("auto-scale output numbers to UNITs; see UNIT below")
                .value_name("UNIT")
                .default_value(options::TO_DEFAULT),
        )
        .arg(
            Arg::with_name(options::PADDING)
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
            Arg::with_name(options::HEADER)
                .long(options::HEADER)
                .help(
                    "print (without converting) the first N header lines; \
                     N defaults to 1 if not specified",
                )
                .value_name("N")
                .default_value(options::HEADER_DEFAULT)
                .hide_default_value(true),
        )
        .arg(
            Arg::with_name(options::ROUND)
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
            Arg::with_name(options::SUFFIX)
                .long(options::SUFFIX)
                .help(
                    "print SUFFIX after each formatted number, and accept \
                    inputs optionally ending with SUFFIX",
                )
                .value_name("SUFFIX"),
        )
        .arg(Arg::with_name(options::NUMBER).hidden(true).multiple(true))
}
