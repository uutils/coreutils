// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{builder::ValueParser, crate_version, Arg, ArgAction, Command};

use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("mkdir.md");
const USAGE: &str = help_usage!("mkdir.md");
const AFTER_HELP: &str = help_section!("after help", "mkdir.md");

pub mod options {
    pub const MODE: &str = "mode";
    pub const PARENTS: &str = "parents";
    pub const VERBOSE: &str = "verbose";
    pub const DIRS: &str = "dirs";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::MODE)
                .short('m')
                .long(options::MODE)
                .help("set file mode (not implemented on windows)"),
        )
        .arg(
            Arg::new(options::PARENTS)
                .short('p')
                .long(options::PARENTS)
                .help("make parent directories as needed")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .short('v')
                .long(options::VERBOSE)
                .help("print a message for each printed directory")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DIRS)
                .action(ArgAction::Append)
                .num_args(1..)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::DirPath),
        )
}
