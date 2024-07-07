// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{builder::ValueParser, crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("mktemp.md");
const USAGE: &str = help_usage!("mktemp.md");

pub mod options {
    pub static OPT_DIRECTORY: &str = "directory";
    pub static OPT_DRY_RUN: &str = "dry-run";
    pub static OPT_QUIET: &str = "quiet";
    pub static OPT_SUFFIX: &str = "suffix";
    pub static OPT_TMPDIR: &str = "tmpdir";
    pub static OPT_P: &str = "p";
    pub static OPT_T: &str = "t";
    pub static ARG_TEMPLATE: &str = "template";
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::OPT_DIRECTORY)
                .short('d')
                .long(options::OPT_DIRECTORY)
                .help("Make a directory instead of a file")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_DRY_RUN)
                .short('u')
                .long(options::OPT_DRY_RUN)
                .help("do not create anything; merely print a name (unsafe)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_QUIET)
                .short('q')
                .long("quiet")
                .help("Fail silently if an error occurs.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::OPT_SUFFIX)
                .long(options::OPT_SUFFIX)
                .help(
                    "append SUFFIX to TEMPLATE; SUFFIX must not contain a path separator. \
                     This option is implied if TEMPLATE does not end with X.",
                )
                .value_name("SUFFIX"),
        )
        .arg(
            Arg::new(options::OPT_P)
                .short('p')
                .help("short form of --tmpdir")
                .value_name("DIR")
                .num_args(1)
                .value_parser(ValueParser::path_buf())
                .value_hint(clap::ValueHint::DirPath),
        )
        .arg(
            Arg::new(options::OPT_TMPDIR)
                .long(options::OPT_TMPDIR)
                .help(
                    "interpret TEMPLATE relative to DIR; if DIR is not specified, use \
                     $TMPDIR ($TMP on windows) if set, else /tmp. With this option, \
                     TEMPLATE must not be an absolute name; unlike with -t, TEMPLATE \
                     may contain slashes, but mktemp creates only the final component",
                )
                .value_name("DIR")
                // Allows use of default argument just by setting --tmpdir. Else,
                // use provided input to generate tmpdir
                .num_args(0..=1)
                // Require an equals to avoid ambiguity if no tmpdir is supplied
                .require_equals(true)
                .overrides_with(options::OPT_P)
                .value_parser(ValueParser::path_buf())
                .value_hint(clap::ValueHint::DirPath),
        )
        .arg(
            Arg::new(options::OPT_T)
                .short('t')
                .help(
                    "Generate a template (using the supplied prefix and TMPDIR \
                (TMP on windows) if set) to create a filename template [deprecated]",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(Arg::new(options::ARG_TEMPLATE).num_args(..=1))
}
