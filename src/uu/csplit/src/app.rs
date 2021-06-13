use clap::{crate_version, App, Arg};

const SUMMARY: &str = "split a file into sections determined by context lines";
const LONG_HELP: &str = "Output pieces of FILE separated by PATTERN(s) to files 'xx00', 'xx01', ..., and output byte counts of each piece to standard output.";

pub mod options {
    pub const SUFFIX_FORMAT: &str = "suffix-format";
    pub const SUPPRESS_MATCHED: &str = "suppress-matched";
    pub const DIGITS: &str = "digits";
    pub const PREFIX: &str = "prefix";
    pub const KEEP_FILES: &str = "keep-files";
    pub const QUIET: &str = "quiet";
    pub const ELIDE_EMPTY_FILES: &str = "elide-empty-files";
    pub const FILE: &str = "file";
    pub const PATTERN: &str = "pattern";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(SUMMARY)
        .arg(
            Arg::with_name(options::SUFFIX_FORMAT)
                .short("b")
                .long(options::SUFFIX_FORMAT)
                .value_name("FORMAT")
                .help("use sprintf FORMAT instead of %02d"),
        )
        .arg(
            Arg::with_name(options::PREFIX)
                .short("f")
                .long(options::PREFIX)
                .value_name("PREFIX")
                .help("use PREFIX instead of 'xx'"),
        )
        .arg(
            Arg::with_name(options::KEEP_FILES)
                .short("k")
                .long(options::KEEP_FILES)
                .help("do not remove output files on errors"),
        )
        .arg(
            Arg::with_name(options::SUPPRESS_MATCHED)
                .long(options::SUPPRESS_MATCHED)
                .help("suppress the lines matching PATTERN"),
        )
        .arg(
            Arg::with_name(options::DIGITS)
                .short("n")
                .long(options::DIGITS)
                .value_name("DIGITS")
                .help("use specified number of digits instead of 2"),
        )
        .arg(
            Arg::with_name(options::QUIET)
                .short("s")
                .long(options::QUIET)
                .visible_alias("silent")
                .help("do not print counts of output file sizes"),
        )
        .arg(
            Arg::with_name(options::ELIDE_EMPTY_FILES)
                .short("z")
                .long(options::ELIDE_EMPTY_FILES)
                .help("remove empty output files"),
        )
        .arg(Arg::with_name(options::FILE).hidden(true).required(true))
        .arg(
            Arg::with_name(options::PATTERN)
                .hidden(true)
                .multiple(true)
                .required(true),
        )
        .after_help(LONG_HELP)
}
