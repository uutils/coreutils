// This file is part of the uutils coreutils package.
//
// (c) Derek Chiang <derekchiang93@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use std::path::Path;
use uucore::InvalidEncodingHandling;

static ABOUT: &str = "strip last component from file name";
static VERSION: &str = env!("CARGO_PKG_VERSION");

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

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let usage = get_usage();
    let after_help = get_long_usage();

    let matches = App::new(executable!())
        .about(ABOUT)
        .usage(&usage[..])
        .after_help(&after_help[..])
        .version(VERSION)
        .arg(
            Arg::with_name(options::ZERO)
                .long(options::ZERO)
                .short("z")
                .help("separate output with NUL rather than newline"),
        )
        .arg(Arg::with_name(options::DIR).hidden(true).multiple(true))
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
        show_usage_error!("missing operand");
        return 1;
    }

    0
}
