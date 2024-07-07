// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::builder::ValueParser;
use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("runcon.md");
const USAGE: &str = help_usage!("runcon.md");
const DESCRIPTION: &str = help_section!("after help", "runcon.md");

pub mod options {
    pub const COMPUTE: &str = "compute";

    pub const USER: &str = "user";
    pub const ROLE: &str = "role";
    pub const TYPE: &str = "type";
    pub const RANGE: &str = "range";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(DESCRIPTION)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::COMPUTE)
                .short('c')
                .long(options::COMPUTE)
                .help("Compute process transition context before modifying.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::USER)
                .short('u')
                .long(options::USER)
                .value_name("USER")
                .help("Set user USER in the target security context.")
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            Arg::new(options::ROLE)
                .short('r')
                .long(options::ROLE)
                .value_name("ROLE")
                .help("Set role ROLE in the target security context.")
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            Arg::new(options::TYPE)
                .short('t')
                .long(options::TYPE)
                .value_name("TYPE")
                .help("Set type TYPE in the target security context.")
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            Arg::new(options::RANGE)
                .short('l')
                .long(options::RANGE)
                .value_name("RANGE")
                .help("Set range RANGE in the target security context.")
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            Arg::new("ARG")
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::CommandName),
        )
        // Once "ARG" is parsed, everything after that belongs to it.
        //
        // This is not how POSIX does things, but this is how the GNU implementation
        // parses its command line.
        .trailing_var_arg(true)
}
