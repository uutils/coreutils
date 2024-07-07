// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("comm.md");
const USAGE: &str = help_usage!("comm.md");

pub mod options {
    pub const COLUMN_1: &str = "1";
    pub const COLUMN_2: &str = "2";
    pub const COLUMN_3: &str = "3";
    pub const DELIMITER: &str = "output-delimiter";
    pub const DELIMITER_DEFAULT: &str = "\t";
    pub const FILE_1: &str = "FILE1";
    pub const FILE_2: &str = "FILE2";
    pub const TOTAL: &str = "total";
    pub const ZERO_TERMINATED: &str = "zero-terminated";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::COLUMN_1)
                .short('1')
                .help("suppress column 1 (lines unique to FILE1)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::COLUMN_2)
                .short('2')
                .help("suppress column 2 (lines unique to FILE2)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::COLUMN_3)
                .short('3')
                .help("suppress column 3 (lines that appear in both files)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::DELIMITER)
                .long(options::DELIMITER)
                .help("separate columns with STR")
                .value_name("STR")
                .default_value(options::DELIMITER_DEFAULT)
                .allow_hyphen_values(true)
                .action(ArgAction::Append)
                .hide_default_value(true),
        )
        .arg(
            Arg::new(options::ZERO_TERMINATED)
                .long(options::ZERO_TERMINATED)
                .short('z')
                .overrides_with(options::ZERO_TERMINATED)
                .help("line delimiter is NUL, not newline")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE_1)
                .required(true)
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::FILE_2)
                .required(true)
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::TOTAL)
                .long(options::TOTAL)
                .help("output a summary")
                .action(ArgAction::SetTrue),
        )
}
