use clap::{crate_version, App, Arg};

const ABOUT: &str = "Remove the DIRECTORY(ies), if they are empty.";

pub const OPT_IGNORE_FAIL_NON_EMPTY: &str = "ignore-fail-on-non-empty";
pub const OPT_PARENTS: &str = "parents";
pub const OPT_VERBOSE: &str = "verbose";

pub const ARG_DIRS: &str = "dirs";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(OPT_IGNORE_FAIL_NON_EMPTY)
                .long(OPT_IGNORE_FAIL_NON_EMPTY)
                .help("ignore each failure that is solely because a directory is non-empty"),
        )
        .arg(
            Arg::with_name(OPT_PARENTS)
                .short("p")
                .long(OPT_PARENTS)
                .help(
                    "remove DIRECTORY and its ancestors; e.g.,
              'rmdir -p a/b/c' is similar to rmdir a/b/c a/b a",
                ),
        )
        .arg(
            Arg::with_name(OPT_VERBOSE)
                .short("v")
                .long(OPT_VERBOSE)
                .help("output a diagnostic for every directory processed"),
        )
        .arg(
            Arg::with_name(ARG_DIRS)
                .multiple(true)
                .takes_value(true)
                .min_values(1)
                .required(true),
        )
}
