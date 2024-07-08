// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};

use uucore::{format_usage, help_about, help_usage};

const USAGE: &str = help_usage!("sum.md");
const ABOUT: &str = help_about!("sum.md");

pub mod options {
    pub static FILE: &str = "file";
    pub static BSD_COMPATIBLE: &str = "r";
    pub static SYSTEM_V_COMPATIBLE: &str = "sysv";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .about(ABOUT)
        .infer_long_args(true)
        .arg(
            Arg::new(options::FILE)
                .action(ArgAction::Append)
                .hide(true)
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::BSD_COMPATIBLE)
                .short('r')
                .help("use the BSD sum algorithm, use 1K blocks (default)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SYSTEM_V_COMPATIBLE)
                .short('s')
                .long(options::SYSTEM_V_COMPATIBLE)
                .help("use System V sum algorithm, use 512 bytes blocks")
                .action(ArgAction::SetTrue),
        )
}
