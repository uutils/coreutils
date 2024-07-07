// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_section, help_usage};

const USAGE: &str = help_usage!("dircolors.md");
const ABOUT: &str = help_about!("dircolors.md");
const AFTER_HELP: &str = help_section!("after help", "dircolors.md");

pub mod options {
    pub const BOURNE_SHELL: &str = "bourne-shell";
    pub const C_SHELL: &str = "c-shell";
    pub const PRINT_DATABASE: &str = "print-database";
    pub const PRINT_LS_COLORS: &str = "print-ls-colors";
    pub const FILE: &str = "FILE";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .override_usage(format_usage(USAGE))
        .args_override_self(true)
        .infer_long_args(true)
        .arg(
            Arg::new(options::BOURNE_SHELL)
                .long("sh")
                .short('b')
                .visible_alias("bourne-shell")
                .overrides_with(options::C_SHELL)
                .help("output Bourne shell code to set LS_COLORS")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::C_SHELL)
                .long("csh")
                .short('c')
                .visible_alias("c-shell")
                .overrides_with(options::BOURNE_SHELL)
                .help("output C shell code to set LS_COLORS")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PRINT_DATABASE)
                .long("print-database")
                .short('p')
                .help("print the byte counts")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::PRINT_LS_COLORS)
                .long("print-ls-colors")
                .help("output fully escaped colors for display")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE)
                .hide(true)
                .value_hint(clap::ValueHint::FilePath)
                .action(ArgAction::Append),
        )
}
