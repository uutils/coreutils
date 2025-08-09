// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use std::path::Path;
use uucore::display::print_verbatim;
use uucore::error::{UResult, UUsageError};
use uucore::format_usage;
use uucore::line_ending::LineEnding;

use uucore::translate;

mod options {
    pub const ZERO: &str = "zero";
    pub const DIR: &str = "dir";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app()
        .after_help(translate!("dirname-after-help"))
        .try_get_matches_from(args)?;

    let line_ending = LineEnding::from_zero_flag(matches.get_flag(options::ZERO));

    let dirnames: Vec<OsString> = matches
        .get_many::<OsString>(options::DIR)
        .unwrap_or_default()
        .cloned()
        .collect();

    if dirnames.is_empty() {
        return Err(UUsageError::new(1, translate!("dirname-missing-operand")));
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
                if p.is_absolute() || path.as_os_str() == "/" {
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
        .about(translate!("dirname-about"))
        .version(uucore::crate_version!())
        .override_usage(format_usage(&translate!("dirname-usage")))
        .args_override_self(true)
        .infer_long_args(true)
        .arg(
            Arg::new(options::ZERO)
                .long(options::ZERO)
                .short('z')
                .help(translate!("dirname-zero-help"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DIR)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath)
                .value_parser(clap::value_parser!(OsString)),
        )
}
