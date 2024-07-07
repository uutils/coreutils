// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::builder::ValueParser;
use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("stat.md");
const USAGE: &str = help_usage!("stat.md");
const LONG_USAGE: &str = help_section!("long usage", "stat.md");

pub mod options {
    pub const DEREFERENCE: &str = "dereference";
    pub const FILE_SYSTEM: &str = "file-system";
    pub const FORMAT: &str = "format";
    pub const PRINTF: &str = "printf";
    pub const TERSE: &str = "terse";
    pub const FILES: &str = "files";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(LONG_USAGE)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::DEREFERENCE)
                .short('L')
                .long(options::DEREFERENCE)
                .help("follow links")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE_SYSTEM)
                .short('f')
                .long(options::FILE_SYSTEM)
                .help("display file system status instead of file status")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::TERSE)
                .short('t')
                .long(options::TERSE)
                .help("print the information in terse form")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FORMAT)
                .short('c')
                .long(options::FORMAT)
                .help(
                    "use the specified FORMAT instead of the default;
 output a newline after each use of FORMAT",
                )
                .value_name("FORMAT"),
        )
        .arg(
            Arg::new(options::PRINTF)
                .long(options::PRINTF)
                .value_name("FORMAT")
                .help(
                    "like --format, but interpret backslash escapes,
            and do not output a mandatory trailing newline;
            if you want a newline, include \n in FORMAT",
                ),
        )
        .arg(
            Arg::new(options::FILES)
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::FilePath),
        )
}
