use clap::{crate_version, App, Arg};

const ABOUT: &str = "Print value of a symbolic link or canonical file name.";
pub const OPT_CANONICALIZE: &str = "canonicalize";
pub const OPT_CANONICALIZE_MISSING: &str = "canonicalize-missing";
pub const OPT_CANONICALIZE_EXISTING: &str = "canonicalize-existing";
pub const OPT_NO_NEWLINE: &str = "no-newline";
pub const OPT_QUIET: &str = "quiet";
pub const OPT_SILENT: &str = "silent";
pub const OPT_VERBOSE: &str = "verbose";
pub const OPT_ZERO: &str = "zero";

pub const ARG_FILES: &str = "files";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(OPT_CANONICALIZE)
                .short("f")
                .long(OPT_CANONICALIZE)
                .help(
                    "canonicalize by following every symlink in every component of the \
                 given name recursively; all but the last component must exist",
                ),
        )
        .arg(
            Arg::with_name(OPT_CANONICALIZE_EXISTING)
                .short("e")
                .long("canonicalize-existing")
                .help(
                    "canonicalize by following every symlink in every component of the \
                 given name recursively, all components must exist",
                ),
        )
        .arg(
            Arg::with_name(OPT_CANONICALIZE_MISSING)
                .short("m")
                .long(OPT_CANONICALIZE_MISSING)
                .help(
                    "canonicalize by following every symlink in every component of the \
                 given name recursively, without requirements on components existence",
                ),
        )
        .arg(
            Arg::with_name(OPT_NO_NEWLINE)
                .short("n")
                .long(OPT_NO_NEWLINE)
                .help("do not output the trailing delimiter"),
        )
        .arg(
            Arg::with_name(OPT_QUIET)
                .short("q")
                .long(OPT_QUIET)
                .help("suppress most error messages"),
        )
        .arg(
            Arg::with_name(OPT_SILENT)
                .short("s")
                .long(OPT_SILENT)
                .help("suppress most error messages"),
        )
        .arg(
            Arg::with_name(OPT_VERBOSE)
                .short("v")
                .long(OPT_VERBOSE)
                .help("report error message"),
        )
        .arg(
            Arg::with_name(OPT_ZERO)
                .short("z")
                .long(OPT_ZERO)
                .help("separate output with NUL rather than newline"),
        )
        .arg(Arg::with_name(ARG_FILES).multiple(true).takes_value(true))
}
