// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use uucore::format_usage;
use uucore::translate;

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
    Command::new("cat")
        .version(uucore::crate_version!())
        .override_usage(format_usage(&translate!("cat-usage")))
        .about(translate!("cat-about"))
        .help_template(uucore::localized_help_template("cat"))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .action(ArgAction::Append)
                .value_parser(clap::value_parser!(OsString))
                .default_value("-")
                .value_hint(clap::ValueHint::FilePath),
        )
        .arg(
            Arg::new(options::SHOW_ALL)
                .short('A')
                .long(options::SHOW_ALL)
                .help(translate!("cat-help-show-all"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NUMBER_NONBLANK)
                .short('b')
                .long(options::NUMBER_NONBLANK)
                .help(translate!("cat-help-number-nonblank"))
                // Note: This MUST NOT .overrides_with(options::NUMBER)!
                // In clap, overriding is symmetric, so "-b -n" counts as "-n", which is not what we want.
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHOW_NONPRINTING_ENDS)
                .short('e')
                .help(translate!("cat-help-show-nonprinting-ends"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHOW_ENDS)
                .short('E')
                .long(options::SHOW_ENDS)
                .help(translate!("cat-help-show-ends"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NUMBER)
                .short('n')
                .long(options::NUMBER)
                .help(translate!("cat-help-number"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SQUEEZE_BLANK)
                .short('s')
                .long(options::SQUEEZE_BLANK)
                .help(translate!("cat-help-squeeze-blank"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHOW_NONPRINTING_TABS)
                .short('t')
                .help(translate!("cat-help-show-nonprinting-tabs"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHOW_TABS)
                .short('T')
                .long(options::SHOW_TABS)
                .help(translate!("cat-help-show-tabs"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SHOW_NONPRINTING)
                .short('v')
                .long(options::SHOW_NONPRINTING)
                .help(translate!("cat-help-show-nonprinting"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::IGNORED_U)
                .short('u')
                .help(translate!("cat-help-ignored-u"))
                .action(ArgAction::SetTrue),
        )
}
