// This file is part of the uutils coreutils package.
//
// (c) Derek Chiang <derekchiang93@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[macro_use]
extern crate uucore;

use clap::{crate_version, App, Arg};
use std::path::Path;
use uucore::error::{UResult, USimpleError};
use uucore::InvalidEncodingHandling;

static ABOUT: &str = "strip last component from file name";

mod options {
    pub const ZERO: &str = "zero";
    pub const DIR: &str = "dir";
}

fn get_usage() -> String {
    format!("{0} [OPTION] NAME...", executable!())
}

fn get_long_usage() -> String {
    String::from(
        "Output each NAME with its last non-slash component and trailing slashes
        removed; if NAME contains no /'s, output '.' (meaning the current directory).",
    )
}

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let usage = get_usage();
    let after_help = get_long_usage();

    let matches = uu_app()
        .usage(&usage[..])
        .after_help(&after_help[..])
        .get_matches_from(args);

    let separator = if matches.is_present(options::ZERO) {
        "\0"
    } else {
        "\n"
    };

    let dirnames: Vec<String> = matches
        .values_of(options::DIR)
        .unwrap_or_default()
        .map(str::to_owned)
        .collect();

    if !dirnames.is_empty() {
        for path in dirnames.iter() {
            let p = Path::new(path);
            match p.parent() {
                Some(d) => {
                    if d.components().next() == None {
                        print!(".")
                    } else {
                        print!("{}", d.to_string_lossy());
                    }
                }
                None => {
                    if p.is_absolute() || path == "/" {
                        print!("/");
                    } else {
                        print!(".");
                    }
                }
            }
            print!("{}", separator);
        }
    } else {
        return Err(UUsageError::new(1, "missing operand".to_string()));
    }

    Ok(())
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(executable!())
        .about(ABOUT)
        .version(crate_version!())
        .arg(
            Arg::with_name(options::ZERO)
                .long(options::ZERO)
                .short("z")
                .help("separate output with NUL rather than newline"),
        )
        .arg(Arg::with_name(options::DIR).hidden(true).multiple(true))
}
