// spell-checker:ignore (ToDO) ugoa RFILE RFILE's

use clap::{crate_version, App, Arg};

const ABOUT: &str = "Change the mode of each FILE to MODE.
 With --reference, change the mode of each FILE to that of RFILE.";

const LONG_USAGE: &str =
    "Each MODE is of the form '[ugoa]*([-+=]([rwxXst]*|[ugo]))+|[-+=]?[0-7]+'.";

pub mod options {
    pub const CHANGES: &str = "changes";
    pub const QUIET: &str = "quiet"; // visible_alias("silent")
    pub const VERBOSE: &str = "verbose";
    pub const NO_PRESERVE_ROOT: &str = "no-preserve-root";
    pub const PRESERVE_ROOT: &str = "preserve-root";
    pub const REFERENCE: &str = "RFILE";
    pub const RECURSIVE: &str = "recursive";
    pub const MODE: &str = "MODE";
    pub const FILE: &str = "FILE";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .after_help(LONG_USAGE)
        .arg(
            Arg::with_name(options::CHANGES)
                .long(options::CHANGES)
                .short("c")
                .help("like verbose but report only when a change is made"),
        )
        .arg(
            Arg::with_name(options::QUIET)
                .long(options::QUIET)
                .visible_alias("silent")
                .short("f")
                .help("suppress most error messages"),
        )
        .arg(
            Arg::with_name(options::VERBOSE)
                .long(options::VERBOSE)
                .short("v")
                .help("output a diagnostic for every file processed"),
        )
        .arg(
            Arg::with_name(options::NO_PRESERVE_ROOT)
                .long(options::NO_PRESERVE_ROOT)
                .help("do not treat '/' specially (the default)"),
        )
        .arg(
            Arg::with_name(options::PRESERVE_ROOT)
                .long(options::PRESERVE_ROOT)
                .help("fail to operate recursively on '/'"),
        )
        .arg(
            Arg::with_name(options::RECURSIVE)
                .long(options::RECURSIVE)
                .short("R")
                .help("change files and directories recursively"),
        )
        .arg(
            Arg::with_name(options::REFERENCE)
                .long("reference")
                .takes_value(true)
                .help("use RFILE's mode instead of MODE values"),
        )
        .arg(
            Arg::with_name(options::MODE)
                .required_unless(options::REFERENCE)
                .takes_value(true),
            // It would be nice if clap could parse with delimiter, e.g. "g-x,u+x",
            // however .multiple(true) cannot be used here because FILE already needs that.
            // Only one positional argument with .multiple(true) set is allowed per command
        )
        .arg(
            Arg::with_name(options::FILE)
                .required_unless(options::MODE)
                .multiple(true),
        )
}
