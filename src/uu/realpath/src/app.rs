use clap::{crate_version, App, Arg};

const ABOUT: &str = "print the resolved path";

pub const OPT_QUIET: &str = "quiet";
pub const OPT_STRIP: &str = "strip";
pub const OPT_ZERO: &str = "zero";

pub const ARG_FILES: &str = "files";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(OPT_QUIET)
                .short("q")
                .long(OPT_QUIET)
                .help("Do not print warnings for invalid paths"),
        )
        .arg(
            Arg::with_name(OPT_STRIP)
                .short("s")
                .long(OPT_STRIP)
                .help("Only strip '.' and '..' components, but don't resolve symbolic links"),
        )
        .arg(
            Arg::with_name(OPT_ZERO)
                .short("z")
                .long(OPT_ZERO)
                .help("Separate output filenames with \\0 rather than newline"),
        )
        .arg(
            Arg::with_name(ARG_FILES)
                .multiple(true)
                .takes_value(true)
                .required(true)
                .min_values(1),
        )
}
