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

static NAME: &str = "dirname";
static SYNTAX: &str = "[OPTION] NAME...";
static SUMMARY: &str = "strip last component from file name";
static VERSION: &str = env!("CARGO_PKG_VERSION");
static LONG_HELP: &str = "
 Output each NAME with its last non-slash component and trailing slashes
 removed; if NAME contains no /'s, output '.' (meaning the current
 directory).
";

mod options {
    pub const ZERO: &str = "zero";
    pub const DIR: &str = "dir";
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let matches = App::new(executable!())
        .name(NAME)
        .usage(SYNTAX)
        .about(SUMMARY)
        .after_help(LONG_HELP)
        .version(VERSION)
        .arg(
            Arg::with_name(options::ZERO)
                .short(options::ZERO)
                .short("z")
                .takes_value(false)
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
