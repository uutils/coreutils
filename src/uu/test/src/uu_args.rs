// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Command};
use uucore::{format_usage, help_about, help_section};

const ABOUT: &str = help_about!("test.md");

// The help_usage method replaces util name (the first word) with {}.
// And, The format_usage method replaces {} with execution_phrase ( e.g. test or [ ).
// However, This test command has two util names.
// So, we use test or [ instead of {} so that the usage string is correct.
const USAGE: &str = "\
test EXPRESSION
[
[ EXPRESSION ]
[ ]
[ OPTION
]";

// We use after_help so that this comes after the usage string (it would come before if we used about)
const AFTER_HELP: &str = help_section!("after help", "test.md");

pub fn uu_app() -> Command {
    // Disable printing of -h and -v as valid alternatives for --help and --version,
    // since we don't recognize -h and -v as help/version flags.
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .after_help(AFTER_HELP)
}
