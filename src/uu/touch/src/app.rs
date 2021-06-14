// spell-checker:ignore (ToDO) MMDDhhmm

use clap::{crate_version, App, Arg, ArgGroup};

const ABOUT: &str = "Update the access and modification times of each FILE to the current time.";
pub mod options {
    // Both SOURCES and sources are needed as we need to be able to refer to the ArgGroup.
    pub const SOURCES: &str = "sources";
    pub mod sources {
        pub const DATE: &str = "date";
        pub const REFERENCE: &str = "reference";
        pub const CURRENT: &str = "current";
    }
    pub const ACCESS: &str = "access";
    pub const MODIFICATION: &str = "modification";
    pub const NO_CREATE: &str = "no-create";
    pub const NO_DEREF: &str = "no-dereference";
    pub const TIME: &str = "time";
}

pub const ARG_FILES: &str = "files";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::ACCESS)
                .short("a")
                .help("change only the access time"),
        )
        .arg(
            Arg::with_name(options::sources::CURRENT)
                .short("t")
                .help("use [[CC]YY]MMDDhhmm[.ss] instead of the current time")
                .value_name("STAMP")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(options::sources::DATE)
                .short("d")
                .long(options::sources::DATE)
                .help("parse argument and use it instead of current time")
                .value_name("STRING"),
        )
        .arg(
            Arg::with_name(options::MODIFICATION)
                .short("m")
                .help("change only the modification time"),
        )
        .arg(
            Arg::with_name(options::NO_CREATE)
                .short("c")
                .long(options::NO_CREATE)
                .help("do not create any files"),
        )
        .arg(
            Arg::with_name(options::NO_DEREF)
                .short("h")
                .long(options::NO_DEREF)
                .help(
                    "affect each symbolic link instead of any referenced file \
                     (only for systems that can change the timestamps of a symlink)",
                ),
        )
        .arg(
            Arg::with_name(options::sources::REFERENCE)
                .short("r")
                .long(options::sources::REFERENCE)
                .help("use this file's times instead of the current time")
                .value_name("FILE"),
        )
        .arg(
            Arg::with_name(options::TIME)
                .long(options::TIME)
                .help(
                    "change only the specified time: \"access\", \"atime\", or \
                     \"use\" are equivalent to -a; \"modify\" or \"mtime\" are \
                     equivalent to -m",
                )
                .value_name("WORD")
                .possible_values(&["access", "atime", "use"])
                .takes_value(true),
        )
        .arg(
            Arg::with_name(ARG_FILES)
                .multiple(true)
                .takes_value(true)
                .min_values(1),
        )
        .group(ArgGroup::with_name(options::SOURCES).args(&[
            options::sources::CURRENT,
            options::sources::DATE,
            options::sources::REFERENCE,
        ]))
}
