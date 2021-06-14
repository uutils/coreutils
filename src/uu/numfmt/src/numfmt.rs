//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Yury Krivopalov <ykrivopalov@yandex.ru>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

#[macro_use]
extern crate uucore;

use crate::app::*;
use crate::format::format_and_print;
use crate::options::TransformOptions;
use crate::units::{Result, Transform, Unit};
use clap::ArgMatches;
use options::NumfmtOptions;
use std::io::{BufRead, Write};
use uucore::ranges::Range;

pub mod app;
pub mod format;
mod options;
mod units;

fn get_usage() -> String {
    format!("{0} [OPTION]... [NUMBER]...", executable!())
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
    let from = parse_unit(args.value_of(FROM).unwrap())?;
    let to = parse_unit(args.value_of(TO).unwrap())?;

    let transform = TransformOptions {
        from: Transform { unit: from },
        to: Transform { unit: to },
    };

    let padding = match args.value_of(PADDING) {
        Some(s) => s.parse::<isize>().map_err(|err| err.to_string()),
        None => Ok(0),
    }?;

    let header = match args.occurrences_of(HEADER) {
        0 => Ok(0),
        _ => {
            let value = args.value_of(HEADER).unwrap();

            value
                .parse::<usize>()
                .map_err(|_| value)
                .and_then(|n| match n {
                    0 => Err(value),
                    _ => Ok(n),
                })
                .map_err(|value| format!("invalid header value ‘{}’", value))
        }
    }?;

    let fields = match args.value_of(FIELD) {
        Some("-") => vec![Range {
            low: 1,
            high: std::usize::MAX,
        }],
        Some(v) => Range::from_list(v)?,
        None => unreachable!(),
    };

    let delimiter = args.value_of(DELIMITER).map_or(Ok(None), |arg| {
        if arg.len() == 1 {
            Ok(Some(arg.to_string()))
        } else {
            Err("the delimiter must be a single character".to_string())
        }
    })?;

    Ok(NumfmtOptions {
        transform,
        padding,
        header,
        fields,
        delimiter,
    })
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = get_app(executable!())
        .usage(&usage[..])
        .get_matches_from(args);

    let result = parse_options(&matches).and_then(|options| match matches.values_of(NUMBER) {
        Some(values) => handle_args(values, options),
        None => handle_stdin(options),
    });

    match result {
        Err(e) => {
            std::io::stdout().flush().expect("error flushing stdout");
            show_error!("{}", e);
            1
        }
        _ => 0,
    }
}
