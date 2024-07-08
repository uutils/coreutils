// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, value_parser, Arg, ArgAction, Command};

use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("more.md");
const USAGE: &str = help_usage!("more.md");

pub mod options {
    pub const SILENT: &str = "silent";
    #[allow(dead_code)] //TODO: Implement this option
    pub const LOGICAL: &str = "logical";
    #[allow(dead_code)] //TODO: Implement this option
    pub const NO_PAUSE: &str = "no-pause";
    pub const PRINT_OVER: &str = "print-over";
    pub const CLEAN_PRINT: &str = "clean-print";
    pub const SQUEEZE: &str = "squeeze";
    pub const PLAIN: &str = "plain";
    pub const LINES: &str = "lines";
    pub const NUMBER: &str = "number";
    pub const PATTERN: &str = "pattern";
    pub const FROM_LINE: &str = "from-line";
    pub const FILES: &str = "files";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .version(crate_version!())
        .infer_long_args(true)
        .arg(
            Arg::new(options::PRINT_OVER)
                .short('c')
                .long(options::PRINT_OVER)
                .help("Do not scroll, display text and clean line ends")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SILENT)
                .short('d')
                .long(options::SILENT)
                .help("Display help instead of ringing bell")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CLEAN_PRINT)
                .short('p')
                .long(options::CLEAN_PRINT)
                .help("Do not scroll, clean screen and display text")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SQUEEZE)
                .short('s')
                .long(options::SQUEEZE)
                .help("Squeeze multiple blank lines into one")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PLAIN)
                .short('u')
                .long(options::PLAIN)
                .action(ArgAction::SetTrue)
                .hide(true),
        )
        .arg(
            Arg::new(options::PATTERN)
                .short('P')
                .long(options::PATTERN)
                .allow_hyphen_values(true)
                .required(false)
                .value_name("pattern")
                .help("Display file beginning from pattern match"),
        )
        .arg(
            Arg::new(options::FROM_LINE)
                .short('F')
                .long(options::FROM_LINE)
                .num_args(1)
                .value_name("number")
                .value_parser(value_parser!(usize))
                .help("Display file beginning from line number"),
        )
        .arg(
            Arg::new(options::LINES)
                .short('n')
                .long(options::LINES)
                .value_name("number")
                .num_args(1)
                .value_parser(value_parser!(u16).range(0..))
                .help("The number of lines per screen full"),
        )
        .arg(
            Arg::new(options::NUMBER)
                .long(options::NUMBER)
                .num_args(1)
                .value_parser(value_parser!(u16).range(0..))
                .help("Same as --lines"),
        )
        // The commented arguments below are unimplemented:
        /*
        .arg(
            Arg::new(options::LOGICAL)
                .short('f')
                .long(options::LOGICAL)
                .help("Count logical rather than screen lines"),
        )
        .arg(
            Arg::new(options::NO_PAUSE)
                .short('l')
                .long(options::NO_PAUSE)
                .help("Suppress pause after form feed"),
        )
        */
        .arg(
            Arg::new(options::FILES)
                .required(false)
                .action(ArgAction::Append)
                .help("Path to the files to be read")
                .value_hint(clap::ValueHint::FilePath),
        )
}
