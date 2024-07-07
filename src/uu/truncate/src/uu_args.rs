// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("truncate.md");
const AFTER_HELP: &str = help_section!("after help", "truncate.md");
const USAGE: &str = help_usage!("truncate.md");

pub mod options {
    pub static IO_BLOCKS: &str = "io-blocks";
    pub static NO_CREATE: &str = "no-create";
    pub static REFERENCE: &str = "reference";
    pub static SIZE: &str = "size";
    pub static ARG_FILES: &str = "files";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .after_help(AFTER_HELP)
        .arg(
            Arg::new(options::IO_BLOCKS)
                .short('o')
                .long(options::IO_BLOCKS)
                .help(
                    "treat SIZE as the number of I/O blocks of the file rather than bytes \
            (NOT IMPLEMENTED)",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NO_CREATE)
                .short('c')
                .long(options::NO_CREATE)
                .help("do not create files that do not exist")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::REFERENCE)
                .short('r')
                .long(options::REFERENCE)
                .required_unless_present(options::SIZE)
                .help("base the size of each file on the size of RFILE")
                .value_name("RFILE")
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::SIZE)
                .short('s')
                .long(options::SIZE)
                .required_unless_present(options::REFERENCE)
                .help(
                    "set or adjust the size of each file according to SIZE, which is in \
            bytes unless --io-blocks is specified",
                )
                .value_name("SIZE"),
        )
        .arg(
            Arg::new(options::ARG_FILES)
                .value_name("FILE")
                .action(ArgAction::Append)
                .required(true)
                .value_hint(clap::ValueHint::FilePath),
        )
}
