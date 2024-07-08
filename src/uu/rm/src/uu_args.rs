// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{builder::ValueParser, crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("rm.md");
const USAGE: &str = help_usage!("rm.md");
const AFTER_HELP: &str = help_section!("after help", "rm.md");

pub mod options {
    pub static OPT_DIR: &str = "dir";
    pub static OPT_INTERACTIVE: &str = "interactive";
    pub static OPT_FORCE: &str = "force";
    pub static OPT_NO_PRESERVE_ROOT: &str = "no-preserve-root";
    pub static OPT_ONE_FILE_SYSTEM: &str = "one-file-system";
    pub static OPT_PRESERVE_ROOT: &str = "preserve-root";
    pub static OPT_PROMPT: &str = "prompt";
    pub static OPT_PROMPT_MORE: &str = "prompt-more";
    pub static OPT_RECURSIVE: &str = "recursive";
    pub static OPT_VERBOSE: &str = "verbose";
    pub static PRESUME_INPUT_TTY: &str = "-presume-input-tty";

    pub static ARG_FILES: &str = "files";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .args_override_self(true)
        .arg(
            Arg::new(options::OPT_FORCE)
                .short('f')
                .long(options::OPT_FORCE)
                .help("ignore nonexistent files and arguments, never prompt")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_PROMPT)
                .short('i')
                .help("prompt before every removal")
                .overrides_with_all([options::OPT_PROMPT_MORE,options::OPT_INTERACTIVE])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_PROMPT_MORE)
                .short('I')
                .help("prompt once before removing more than three files, or when removing recursively. \
                Less intrusive than -i, while still giving some protection against most mistakes")
                .overrides_with_all([options::OPT_PROMPT, options::OPT_INTERACTIVE])
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_INTERACTIVE)
                .long(options::OPT_INTERACTIVE)
                .help(
                    "prompt according to WHEN: never, once (-I), or always (-i). Without WHEN, \
                    prompts always",
                )
                .value_name("WHEN")
                .num_args(0..=1)
                .require_equals(true)
                .default_missing_value("always")
                .overrides_with_all([options::OPT_PROMPT, options::OPT_PROMPT_MORE]),
        )
        .arg(
            Arg::new(options::OPT_ONE_FILE_SYSTEM)
                .long(options::OPT_ONE_FILE_SYSTEM)
                .help(
                    "when removing a hierarchy recursively, skip any directory that is on a file \
                    system different from that of the corresponding command line argument (NOT \
                    IMPLEMENTED)",
                ).action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_NO_PRESERVE_ROOT)
                .long(options::OPT_NO_PRESERVE_ROOT)
                .help("do not treat '/' specially")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_PRESERVE_ROOT)
                .long(options::OPT_PRESERVE_ROOT)
                .help("do not remove '/' (default)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_RECURSIVE)
                .short('r')
                .visible_short_alias('R')
                .long(options::OPT_RECURSIVE)
                .help("remove directories and their contents recursively")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_DIR)
                .short('d')
                .long(options::OPT_DIR)
                .help("remove empty directories")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_VERBOSE)
                .short('v')
                .long(options::OPT_VERBOSE)
                .help("explain what is being done")
                .action(ArgAction::SetTrue),
        )
        // From the GNU source code:
        // This is solely for testing.
        // Do not document.
        // It is relatively difficult to ensure that there is a tty on stdin.
        // Since rm acts differently depending on that, without this option,
        // it'd be harder to test the parts of rm that depend on that setting.
        // In contrast with Arg::long, Arg::alias does not strip leading
        // hyphens. Therefore it supports 3 leading hyphens.
        .arg(
            Arg::new(options::PRESUME_INPUT_TTY)
                .long("presume-input-tty")
                .alias(options::PRESUME_INPUT_TTY)
                .hide(true)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::ARG_FILES)
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .num_args(1..)
                .value_hint(clap::ValueHint::AnyPath),
        )
}
