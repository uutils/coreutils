// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) COMFOLLOW Chowner RFILE RFILE's derefer dgid nonblank nonprint nonprinting

#[macro_use]
extern crate uucore;
pub use uucore::entries;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::perms::{
    ChownExecutor, IfFrom, Verbosity, VerbosityLevel, FTS_COMFOLLOW, FTS_LOGICAL, FTS_PHYSICAL,
};

use clap::{App, Arg};

use std::fs;
use std::os::unix::fs::MetadataExt;

use uucore::InvalidEncodingHandling;

static ABOUT: &str = "Change the group of each FILE to GROUP.";
static VERSION: &str = env!("CARGO_PKG_VERSION");

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

fn get_usage() -> String {
    format!(
        "{0} [OPTION]... GROUP FILE...\n    {0} [OPTION]... --reference=RFILE FILE...",
        uucore::execution_phrase()
    )
}

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let usage = get_usage();

    let mut app = uu_app().usage(&usage[..]);

    // we change the positional args based on whether
    // --reference was used.
    let mut reference = false;
    let mut help = false;
    // stop processing options on --
    for arg in args.iter().take_while(|s| *s != "--") {
        if arg.starts_with("--reference=") || arg == "--reference" {
            reference = true;
        } else if arg == "--help" {
            // we stop processing once we see --help,
            // as it doesn't matter if we've seen reference or not
            help = true;
            break;
        }
    }

    if help || !reference {
        // add both positional arguments
        app = app.arg(
            Arg::with_name(options::ARG_GROUP)
                .value_name(options::ARG_GROUP)
                .required(true)
                .takes_value(true)
                .multiple(false),
        )
    }
    app = app.arg(
        Arg::with_name(options::ARG_FILES)
            .value_name(options::ARG_FILES)
            .multiple(true)
            .takes_value(true)
            .required(true)
            .min_values(1),
    );

    let matches = app.get_matches_from(args);

    /* Get the list of files */
    let files: Vec<String> = matches
        .values_of(options::ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let preserve_root = matches.is_present(options::preserve_root::PRESERVE);

    let mut derefer = if matches.is_present(options::dereference::DEREFERENCE) {
        1
    } else if matches.is_present(options::dereference::NO_DEREFERENCE) {
        0
    } else {
        -1
    };

    let mut bit_flag = if matches.is_present(options::traverse::TRAVERSE) {
        FTS_COMFOLLOW | FTS_PHYSICAL
    } else if matches.is_present(options::traverse::EVERY) {
        FTS_LOGICAL
    } else {
        FTS_PHYSICAL
    };

    let recursive = matches.is_present(options::RECURSIVE);
    if recursive {
        if bit_flag == FTS_PHYSICAL {
            if derefer == 1 {
                return Err(USimpleError::new(1, "-R --dereference requires -H or -L"));
            }
            derefer = 0;
        }
    } else {
        bit_flag = FTS_PHYSICAL;
    }

    let verbosity_level = if matches.is_present(options::verbosity::CHANGES) {
        VerbosityLevel::Changes
    } else if matches.is_present(options::verbosity::SILENT)
        || matches.is_present(options::verbosity::QUIET)
    {
        VerbosityLevel::Silent
    } else if matches.is_present(options::verbosity::VERBOSE) {
        VerbosityLevel::Verbose
    } else {
        VerbosityLevel::Normal
    };

    let dest_gid = if let Some(file) = matches.value_of(options::REFERENCE) {
        fs::metadata(&file)
            .map(|meta| Some(meta.gid()))
            .map_err_context(|| format!("failed to get attributes of '{}'", file))?
    } else {
        let group = matches.value_of(options::ARG_GROUP).unwrap_or_default();
        if group.is_empty() {
            None
        } else {
            match entries::grp2gid(group) {
                Ok(g) => Some(g),
                _ => return Err(USimpleError::new(1, format!("invalid group: '{}'", group))),
            }
        }
    };

    let executor = ChownExecutor {
        bit_flag,
        dest_gid,
        verbosity: Verbosity {
            groups_only: true,
            level: verbosity_level,
        },
        recursive,
        dereference: derefer != 0,
        preserve_root,
        files,
        filter: IfFrom::All,
        dest_uid: None,
    };
    executor.exec()
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(uucore::util_name())
        .version(VERSION)
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
