//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Haitao Li <lihaitao@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) errno

#[macro_use]
extern crate uucore;

use clap::{crate_version, Arg, Command};
use std::fs;
use std::io::{stdout, Write};
use std::path::{Path, PathBuf};
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};
use uucore::format_usage;
use uucore::fs::{canonicalize, MissingHandling, ResolveMode};

const ABOUT: &str = "Print value of a symbolic link or canonical file name.";
const USAGE: &str = "{} [OPTION]... [FILE]...";
const OPT_CANONICALIZE: &str = "canonicalize";
const OPT_CANONICALIZE_MISSING: &str = "canonicalize-missing";
const OPT_CANONICALIZE_EXISTING: &str = "canonicalize-existing";
const OPT_NO_NEWLINE: &str = "no-newline";
const OPT_QUIET: &str = "quiet";
const OPT_SILENT: &str = "silent";
const OPT_VERBOSE: &str = "verbose";
const OPT_ZERO: &str = "zero";

const ARG_FILES: &str = "files";

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(args);

    let mut no_newline = matches.is_present(OPT_NO_NEWLINE);
    let use_zero = matches.is_present(OPT_ZERO);
    let silent = matches.is_present(OPT_SILENT) || matches.is_present(OPT_QUIET);
    let verbose = matches.is_present(OPT_VERBOSE);

    let res_mode = if matches.is_present(OPT_CANONICALIZE)
        || matches.is_present(OPT_CANONICALIZE_EXISTING)
        || matches.is_present(OPT_CANONICALIZE_MISSING)
    {
        ResolveMode::Logical
    } else {
        ResolveMode::None
    };

    let can_mode = if matches.is_present(OPT_CANONICALIZE_EXISTING) {
        MissingHandling::Existing
    } else if matches.is_present(OPT_CANONICALIZE_MISSING) {
        MissingHandling::Missing
    } else {
        MissingHandling::Normal
    };

    let files: Vec<String> = matches
        .values_of(ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();
    if files.is_empty() {
        return Err(UUsageError::new(1, "missing operand"));
    }

    if no_newline && files.len() > 1 && !silent {
        show_error!("ignoring --no-newline with multiple arguments");
        no_newline = false;
    }

    for f in &files {
        let p = PathBuf::from(f);
        let path_result = if res_mode == ResolveMode::None {
            fs::read_link(&p)
        } else {
            canonicalize(&p, can_mode, res_mode)
        };
        match path_result {
            Ok(path) => show(&path, no_newline, use_zero).map_err_context(String::new)?,
            Err(err) => {
                if verbose {
                    return Err(USimpleError::new(
                        1,
                        format!("{}: errno {}", f.maybe_quote(), err.raw_os_error().unwrap()),
                    ));
                } else {
                    return Err(1.into());
                }
            }
        }
    }
    Ok(())
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_help(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(OPT_CANONICALIZE)
                .short('f')
                .long(OPT_CANONICALIZE)
                .help(
                    "canonicalize by following every symlink in every component of the \
                     given name recursively; all but the last component must exist",
                ),
        )
        .arg(
            Arg::new(OPT_CANONICALIZE_EXISTING)
                .short('e')
                .long("canonicalize-existing")
                .help(
                    "canonicalize by following every symlink in every component of the \
                     given name recursively, all components must exist",
                ),
        )
        .arg(
            Arg::new(OPT_CANONICALIZE_MISSING)
                .short('m')
                .long(OPT_CANONICALIZE_MISSING)
                .help(
                    "canonicalize by following every symlink in every component of the \
                     given name recursively, without requirements on components existence",
                ),
        )
        .arg(
            Arg::new(OPT_NO_NEWLINE)
                .short('n')
                .long(OPT_NO_NEWLINE)
                .help("do not output the trailing delimiter"),
        )
        .arg(
            Arg::new(OPT_QUIET)
                .short('q')
                .long(OPT_QUIET)
                .help("suppress most error messages"),
        )
        .arg(
            Arg::new(OPT_SILENT)
                .short('s')
                .long(OPT_SILENT)
                .help("suppress most error messages"),
        )
        .arg(
            Arg::new(OPT_VERBOSE)
                .short('v')
                .long(OPT_VERBOSE)
                .help("report error message"),
        )
        .arg(
            Arg::new(OPT_ZERO)
                .short('z')
                .long(OPT_ZERO)
                .help("separate output with NUL rather than newline"),
        )
        .arg(
            Arg::new(ARG_FILES)
                .multiple_occurrences(true)
                .takes_value(true),
        )
}

fn show(path: &Path, no_newline: bool, use_zero: bool) -> std::io::Result<()> {
    let path = path.to_str().unwrap();
    if use_zero {
        print!("{}\0", path);
    } else if no_newline {
        print!("{}", path);
    } else {
        println!("{}", path);
    }
    stdout().flush()
}
