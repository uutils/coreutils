// spell-checker:ignore (ToDO) RFILE RFILE's

use clap::{crate_version, App, Arg};

const ABOUT: &str = "Change the group of each FILE to GROUP.";

pub mod options {
    pub mod verbosity {
        pub static CHANGES: &str = "changes";
        pub static QUIET: &str = "quiet";
        pub static SILENT: &str = "silent";
        pub static VERBOSE: &str = "verbose";
    }
    pub mod preserve_root {
        pub static PRESERVE: &str = "preserve-root";
        pub static NO_PRESERVE: &str = "no-preserve-root";
    }
    pub mod dereference {
        pub static DEREFERENCE: &str = "dereference";
        pub static NO_DEREFERENCE: &str = "no-dereference";
    }
    pub static RECURSIVE: &str = "recursive";
    pub mod traverse {
        pub static TRAVERSE: &str = "H";
        pub static NO_TRAVERSE: &str = "P";
        pub static EVERY: &str = "L";
    }
    pub static REFERENCE: &str = "reference";
    pub static ARG_GROUP: &str = "GROUP";
    pub static ARG_FILES: &str = "FILE";
}

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
        .arg(
            Arg::with_name(options::verbosity::SILENT)
                .short("f")
                .long(options::verbosity::SILENT),
        )
        .arg(
            Arg::with_name(options::verbosity::QUIET)
                .long(options::verbosity::QUIET)
                .help("suppress most error messages"),
        )
        .arg(
            Arg::with_name(options::verbosity::VERBOSE)
                .short("v")
                .long(options::verbosity::VERBOSE)
                .help("output a diagnostic for every file processed"),
        )
        .arg(
            Arg::with_name(options::dereference::DEREFERENCE)
                .long(options::dereference::DEREFERENCE),
        )
        .arg(
           Arg::with_name(options::dereference::NO_DEREFERENCE)
               .short("h")
               .long(options::dereference::NO_DEREFERENCE)
               .help(
                   "affect symbolic links instead of any referenced file (useful only on systems that can change the ownership of a symlink)",
               ),
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
            Arg::with_name(options::REFERENCE)
                .long(options::REFERENCE)
                .value_name("RFILE")
                .help("use RFILE's group rather than specifying GROUP values")
                .takes_value(true)
                .multiple(false),
        )
        .arg(
            Arg::with_name(options::RECURSIVE)
                .short("R")
                .long(options::RECURSIVE)
                .help("operate on files and directories recursively"),
        )
        .arg(
            Arg::with_name(options::traverse::TRAVERSE)
                .short(options::traverse::TRAVERSE)
                .help("if a command line argument is a symbolic link to a directory, traverse it"),
        )
        .arg(
            Arg::with_name(options::traverse::NO_TRAVERSE)
                .short(options::traverse::NO_TRAVERSE)
                .help("do not traverse any symbolic links (default)")
                .overrides_with_all(&[options::traverse::TRAVERSE, options::traverse::EVERY]),
        )
        .arg(
            Arg::with_name(options::traverse::EVERY)
                .short(options::traverse::EVERY)
                .help("traverse every symbolic link to a directory encountered"),
        )
}
