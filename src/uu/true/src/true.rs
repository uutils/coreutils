// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use clap::{Arg, ArgAction, Command};
use uucore::{crate_version, translate};

// uucore::main does not support no-result
pub fn uumain(args: impl uucore::Args) -> i32 {
    uu_false::true_false(args, 0, "true")
}

pub fn uu_app() -> Command {
    Command::new("true")
        .version(crate_version!())
        .help_template(uucore::localized_help_template("true"))
        .about(translate!("true-about"))
        // We provide our own help and version options, to ensure maximum compatibility with GNU.
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new("help")
                .long("help")
                .help(translate!("true-help-text"))
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new("version")
                .long("version")
                .help(translate!("true-version-text"))
                .action(ArgAction::Version),
        )
}
