// This file is part of the uutils coreutils package.
//
// (c) Derek Chiang <derekchiang93@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use std::path::Path;
use uucore::display::print_verbatim;
use uucore::error::{UResult, UUsageError};
use uucore::format_usage;

static ABOUT: &str = "strip last component from file name";
const USAGE: &str = "{} [OPTION] NAME...";

mod options {
    pub const ZERO: &str = "zero";
    pub const DIR: &str = "dir";
}

fn get_long_usage() -> &'static str {
    "Output each NAME with its last non-slash component and trailing slashes \n\
        removed; if NAME contains no /'s, output '.' (meaning the current directory)."
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args.collect_lossy();

    let matches = uu_app()
        .after_help(get_long_usage())
        .try_get_matches_from(args)?;

    let separator = if matches.get_flag(options::ZERO) {
        "\0"
    } else {
        "\n"
    };

    let dirnames: Vec<String> = matches
        .get_many::<String>(options::DIR)
        .unwrap_or_default()
        .map(|s| s.to_owned())
        .collect();

    if !dirnames.is_empty() {
        for path in &dirnames {
            let p = Path::new(path);
            match p.parent() {
                Some(d) => {
                    if d.components().next().is_none() {
                        print!(".");
                    } else {
                        print_verbatim(d).unwrap();
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
        return Err(UUsageError::new(1, "missing operand"));
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .about(ABOUT)
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::ZERO)
                .long(options::ZERO)
                .short('z')
                .help("separate output with NUL rather than newline")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DIR)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath),
        )
}
