// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{builder::ValueParser, crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_usage};

static ABOUT: &str = help_about!("rmdir.md");
const USAGE: &str = help_usage!("rmdir.md");

pub mod options {
    pub static OPT_IGNORE_FAIL_NON_EMPTY: &str = "ignore-fail-on-non-empty";
    pub static OPT_PARENTS: &str = "parents";
    pub static OPT_VERBOSE: &str = "verbose";

    pub static ARG_DIRS: &str = "dirs";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::OPT_IGNORE_FAIL_NON_EMPTY)
                .long(options::OPT_IGNORE_FAIL_NON_EMPTY)
                .help("ignore each failure that is solely because a directory is non-empty")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_PARENTS)
                .short('p')
                .long(options::OPT_PARENTS)
                .help(
                    "remove DIRECTORY and its ancestors; e.g.,
                  'rmdir -p a/b/c' is similar to rmdir a/b/c a/b a",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_VERBOSE)
                .short('v')
                .long(options::OPT_VERBOSE)
                .help("output a diagnostic for every directory processed")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ARG_DIRS)
                .action(ArgAction::Append)
                .num_args(1..)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::DirPath),
        )
}
