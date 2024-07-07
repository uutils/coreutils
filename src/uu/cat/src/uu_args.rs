// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore NONBLANK nonblank

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_usage};

pub const USAGE: &str = help_usage!("cat.md");
pub const ABOUT: &str = help_about!("cat.md");

pub mod options {
    pub static FILE: &str = "file";
    pub static SHOW_ALL: &str = "show-all";
    pub static NUMBER_NONBLANK: &str = "number-nonblank";
    pub static SHOW_NONPRINTING_ENDS: &str = "e";
    pub static SHOW_ENDS: &str = "show-ends";
    pub static NUMBER: &str = "number";
    pub static SQUEEZE_BLANK: &str = "squeeze-blank";
    pub static SHOW_NONPRINTING_TABS: &str = "t";
    pub static SHOW_TABS: &str = "show-tabs";
    pub static SHOW_NONPRINTING: &str = "show-nonprinting";
    pub static IGNORED_U: &str = "ignored-u";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .override_usage(format_usage(USAGE))
        .about(ABOUT)
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(clap::ArgAction::Append)
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::SHOW_ALL)
                .short('A')
                .long(options::SHOW_ALL)
                .help("equivalent to -vET")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NUMBER_NONBLANK)
                .short('b')
                .long(options::NUMBER_NONBLANK)
                .help("number nonempty output lines, overrides -n")
                // Note: This MUST NOT .overrides_with(options::NUMBER)!
                // In clap, overriding is symmetric, so "-b -n" counts as "-n", which is not what we want.
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHOW_NONPRINTING_ENDS)
                .short('e')
                .help("equivalent to -vE")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHOW_ENDS)
                .short('E')
                .long(options::SHOW_ENDS)
                .help("display $ at end of each line")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NUMBER)
                .short('n')
                .long(options::NUMBER)
                .help("number all output lines")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SQUEEZE_BLANK)
                .short('s')
                .long(options::SQUEEZE_BLANK)
                .help("suppress repeated empty output lines")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHOW_NONPRINTING_TABS)
                .short('t')
                .help("equivalent to -vT")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHOW_TABS)
                .short('T')
                .long(options::SHOW_TABS)
                .help("display TAB characters at ^I")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHOW_NONPRINTING)
                .short('v')
                .long(options::SHOW_NONPRINTING)
                .help("use ^ and M- notation, except for LF (\\n) and TAB (\\t)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::IGNORED_U)
                .short('u')
                .help("(ignored)")
                .action(ArgAction::SetTrue),
        )
}
