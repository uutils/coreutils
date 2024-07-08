// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::builder::ValueParser;
use clap::{crate_name, crate_version, Arg, ArgAction, Command};

use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("env.md");
const USAGE: &str = help_usage!("env.md");
const AFTER_HELP: &str = help_section!("after help", "env.md");

pub fn uu_app() -> Command {
    Command::new(crate_name!())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .after_help(AFTER_HELP)
        .infer_long_args(true)
        .trailing_var_arg(true)
        .arg(
            Arg::new("ignore-environment")
                .short('i')
                .long("ignore-environment")
                .help("start with an empty environment")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("chdir")
                .short('C') // GNU env compatibility
                .long("chdir")
                .number_of_values(1)
                .value_name("DIR")
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::DirPath)
                .help("change working directory to DIR"),
        )
        .arg(
            Arg::new("null")
                .short('0')
                .long("null")
                .help(
                    "end each output line with a 0 byte rather than a newline (only \
                valid when printing the environment)",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("file")
                .short('f')
                .long("file")
                .value_name("PATH")
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(ValueParser::os_string())
                .action(ArgAction::Append)
                .help(
                    "read and set variables from a \".env\"-style configuration file \
                (prior to any unset and/or set)",
                ),
        )
        .arg(
            Arg::new("unset")
                .short('u')
                .long("unset")
                .value_name("NAME")
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .help("remove variable from the environment"),
        )
        .arg(
            Arg::new("debug")
                .short('v')
                .long("debug")
                .action(ArgAction::Count)
                .help("print verbose information for each processing step"),
        )
        .arg(
            Arg::new("split-string") // split string handling is implemented directly, not using CLAP. But this entry here is needed for the help information output.
                .short('S')
                .long("split-string")
                .value_name("S")
                .action(ArgAction::Set)
                .value_parser(ValueParser::os_string())
                .help("process and split S into separate arguments; used to pass multiple arguments on shebang lines")
        ).arg(
            Arg::new("argv0")
                .overrides_with("argv0")
                .short('a')
                .long("argv0")
                .value_name("a")
                .action(ArgAction::Set)
                .value_parser(ValueParser::os_string())
                .help("Override the zeroth argument passed to the command being executed. \
                       Without this option a default value of `command` is used.")
        )
        .arg(
            Arg::new("vars")
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
        )
        .arg(
            Arg::new("ignore-signal")
                .long("ignore-signal")
                .value_name("SIG")
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .help("set handling of SIG signal(s) to do nothing")
        )
}
