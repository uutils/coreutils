// spell-checker:ignore (ToDO) RFILE RFILE's

use clap::{crate_version, App, Arg};

const ABOUT: &str = "change file owner and group";

pub mod options {
    pub mod verbosity {
        pub const CHANGES: &str = "changes";
        pub const QUIET: &str = "quiet";
        pub const SILENT: &str = "silent";
        pub const VERBOSE: &str = "verbose";
    }
    pub mod preserve_root {
        pub const PRESERVE: &str = "preserve-root";
        pub const NO_PRESERVE: &str = "no-preserve-root";
    }
    pub mod dereference {
        pub const DEREFERENCE: &str = "dereference";
        pub const NO_DEREFERENCE: &str = "no-dereference";
    }
    pub const FROM: &str = "from";
    pub const RECURSIVE: &str = "recursive";
    pub mod traverse {
        pub const TRAVERSE: &str = "H";
        pub const NO_TRAVERSE: &str = "P";
        pub const EVERY: &str = "L";
    }
    pub const REFERENCE: &str = "reference";
}

pub const ARG_OWNER: &str = "owner";
pub const ARG_FILES: &str = "files";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::verbosity::CHANGES)
                .short("c")
                .long(options::verbosity::CHANGES)
                .help("like verbose but report only when a change is made"),
        )
        .arg(Arg::with_name(options::dereference::DEREFERENCE).long(options::dereference::DEREFERENCE).help(
            "affect the referent of each symbolic link (this is the default), rather than the symbolic link itself",
        ))
        .arg(
            Arg::with_name(options::dereference::NO_DEREFERENCE)
                .short("h")
                .long(options::dereference::NO_DEREFERENCE)
                .help(
                    "affect symbolic links instead of any referenced file (useful only on systems that can change the ownership of a symlink)",
                ),
        )
        .arg(
            Arg::with_name(options::FROM)
                .long(options::FROM)
                .help(
                    "change the owner and/or group of each file only if its current owner and/or group match those specified here. Either may be omitted, in which case a match is not required for the omitted attribute",
                )
                .value_name("CURRENT_OWNER:CURRENT_GROUP"),
        )
        .arg(
            Arg::with_name(options::preserve_root::PRESERVE)
                .long(options::preserve_root::PRESERVE)
                .help("fail to operate recursively on '/'"),
        )
        .arg(
            Arg::with_name(options::preserve_root::NO_PRESERVE)
                .long(options::preserve_root::NO_PRESERVE)
                .help("do not treat '/' specially (the default)"),
        )
        .arg(
            Arg::with_name(options::verbosity::QUIET)
                .long(options::verbosity::QUIET)
                .help("suppress most error messages"),
        )
        .arg(
            Arg::with_name(options::RECURSIVE)
                .short("R")
                .long(options::RECURSIVE)
                .help("operate on files and directories recursively"),
        )
        .arg(
            Arg::with_name(options::REFERENCE)
                .long(options::REFERENCE)
                .help("use RFILE's owner and group rather than specifying OWNER:GROUP values")
                .value_name("RFILE")
                .min_values(1),
        )
        .arg(Arg::with_name(options::verbosity::SILENT).short("f").long(options::verbosity::SILENT))
        .arg(
            Arg::with_name(options::traverse::TRAVERSE)
                .short(options::traverse::TRAVERSE)
                .help("if a command line argument is a symbolic link to a directory, traverse it")
                .overrides_with_all(&[options::traverse::EVERY, options::traverse::NO_TRAVERSE]),
        )
        .arg(
            Arg::with_name(options::traverse::EVERY)
                .short(options::traverse::EVERY)
                .help("traverse every symbolic link to a directory encountered")
                .overrides_with_all(&[options::traverse::TRAVERSE, options::traverse::NO_TRAVERSE]),
        )
        .arg(
            Arg::with_name(options::traverse::NO_TRAVERSE)
                .short(options::traverse::NO_TRAVERSE)
                .help("do not traverse any symbolic links (default)")
                .overrides_with_all(&[options::traverse::TRAVERSE, options::traverse::EVERY]),
        )
        .arg(
            Arg::with_name(options::verbosity::VERBOSE)
                .long(options::verbosity::VERBOSE)
                .help("output a diagnostic for every file processed"),
        )
        .arg(
            Arg::with_name(ARG_OWNER)
                .multiple(false)
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(ARG_FILES)
                .multiple(true)
                .takes_value(true)
                .required(true)
                .min_values(1),
        )
}
