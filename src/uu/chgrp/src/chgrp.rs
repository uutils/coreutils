// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) COMFOLLOW Chowner RFILE RFILE's derefer dgid nonblank nonprint nonprinting

use uucore::display::Quotable;
pub use uucore::entries;
use uucore::error::{FromIo, UResult, USimpleError};
use uucore::perms::{chown_base, options, IfFrom};

use clap::{App, Arg, ArgMatches};

use std::fs;
use std::os::unix::fs::MetadataExt;

static ABOUT: &str = "Change the group of each FILE to GROUP.";
static VERSION: &str = env!("CARGO_PKG_VERSION");

fn get_usage() -> String {
    format!(
        "{0} [OPTION]... GROUP FILE...\n    {0} [OPTION]... --reference=RFILE FILE...",
        uucore::execution_phrase()
    )
}

fn parse_gid_and_uid(matches: &ArgMatches) -> UResult<(Option<u32>, Option<u32>, IfFrom)> {
    let dest_gid = if let Some(file) = matches.value_of(options::REFERENCE) {
        fs::metadata(&file)
            .map(|meta| Some(meta.gid()))
            .map_err_context(|| format!("failed to get attributes of {}", file.quote()))?
    } else {
        let group = matches.value_of(options::ARG_GROUP).unwrap_or_default();
        if group.is_empty() {
            None
        } else {
            match entries::grp2gid(group) {
                Ok(g) => Some(g),
                _ => {
                    return Err(USimpleError::new(
                        1,
                        format!("invalid group: {}", group.quote()),
                    ))
                }
            }
        }
    };
    Ok((dest_gid, None, IfFrom::All))
}

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let usage = get_usage();

    chown_base(
        uu_app().usage(&usage[..]),
        args,
        options::ARG_GROUP,
        parse_gid_and_uid,
        true,
    )
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
