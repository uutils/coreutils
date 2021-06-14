use clap::{crate_version, App, Arg};

const ABOUT: &str = "create a temporary file or directory.";

pub const DEFAULT_TEMPLATE: &str = "tmp.XXXXXXXXXX";

pub const OPT_DIRECTORY: &str = "directory";
pub const OPT_DRY_RUN: &str = "dry-run";
pub const OPT_QUIET: &str = "quiet";
pub const OPT_SUFFIX: &str = "suffix";
pub const OPT_TMPDIR: &str = "tmpdir";
pub const OPT_T: &str = "t";

pub const ARG_TEMPLATE: &str = "template";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(OPT_DIRECTORY)
                .short("d")
                .long(OPT_DIRECTORY)
                .help("Make a directory instead of a file"),
        )
        .arg(
            Arg::with_name(OPT_DRY_RUN)
                .short("u")
                .long(OPT_DRY_RUN)
                .help("do not create anything; merely print a name (unsafe)"),
        )
        .arg(
            Arg::with_name(OPT_QUIET)
                .short("q")
                .long("quiet")
                .help("Fail silently if an error occurs."),
        )
        .arg(
            Arg::with_name(OPT_SUFFIX)
                .long(OPT_SUFFIX)
                .help(
                    "append SUFFIX to TEMPLATE; SUFFIX must not contain a path separator. \
                     This option is implied if TEMPLATE does not end with X.",
                )
                .value_name("SUFFIX"),
        )
        .arg(
            Arg::with_name(OPT_TMPDIR)
                .short("p")
                .long(OPT_TMPDIR)
                .help(
                    "interpret TEMPLATE relative to DIR; if DIR is not specified, use \
                     $TMPDIR if set, else /tmp. With this option, TEMPLATE must not \
                     be an absolute name; unlike with -t, TEMPLATE may contain \
                     slashes, but mktemp creates only the final component",
                )
                .value_name("DIR"),
        )
        .arg(Arg::with_name(OPT_T).short(OPT_T).help(
            "Generate a template (using the supplied prefix and TMPDIR if set) \
             to create a filename template [deprecated]",
        ))
        .arg(
            Arg::with_name(ARG_TEMPLATE)
                .multiple(false)
                .takes_value(true)
                .max_values(1)
                .default_value(DEFAULT_TEMPLATE),
        )
}
