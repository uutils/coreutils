// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command};
use std::path::Path;
use uucore::display::print_verbatim;
use uucore::error::{UResult, UUsageError};
use uucore::format_usage;
use uucore::line_ending::LineEnding;

use uucore::locale::get_message;

mod options {
    pub const ZERO: &str = "zero";
    pub const DIR: &str = "dir";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app()
        .after_help(get_message("dirname-after-help"))
        .try_get_matches_from(args)?;

    let line_ending = LineEnding::from_zero_flag(matches.get_flag(options::ZERO));

    let dirnames: Vec<String> = matches
        .get_many::<String>(options::DIR)
        .unwrap_or_default()
        .cloned()
        .collect();

    if dirnames.is_empty() {
        return Err(UUsageError::new(1, get_message("dirname-missing-operand")));
    }

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
        print!("{line_ending}");
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .about(get_message("dirname-about"))
        .version(uucore::crate_version!())
        .override_usage(format_usage(&get_message("dirname-usage")))
        .args_override_self(true)
        .infer_long_args(true)
        .arg(
            Arg::new(options::ZERO)
                .long(options::ZERO)
                .short('z')
                .help(get_message("dirname-zero-help"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DIR)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath),
        )
}
