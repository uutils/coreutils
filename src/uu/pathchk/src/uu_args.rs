// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("pathchk.md");
const USAGE: &str = help_usage!("pathchk.md");

pub mod options {
    pub const POSIX: &str = "posix";
    pub const POSIX_SPECIAL: &str = "posix-special";
    pub const PORTABILITY: &str = "portability";
    pub const PATH: &str = "path";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::POSIX)
                .short('p')
                .help("check for most POSIX systems")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::POSIX_SPECIAL)
                .short('P')
                .help(r#"check for empty names and leading "-""#)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PORTABILITY)
                .long(options::PORTABILITY)
                .help("check for all POSIX systems (equivalent to -p -P)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PATH)
                .hide(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath),
        )
}
