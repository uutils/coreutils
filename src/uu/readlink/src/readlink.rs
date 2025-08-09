// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) errno

use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use std::fs;
use std::io::{Write, stdout};
use std::path::{Path, PathBuf};
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};
use uucore::fs::{MissingHandling, ResolveMode, canonicalize};
use uucore::line_ending::LineEnding;
use uucore::translate;
use uucore::{format_usage, show_error};

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
    let matches = uu_app().try_get_matches_from(args)?;

    let mut no_trailing_delimiter = matches.get_flag(OPT_NO_NEWLINE);
    let use_zero = matches.get_flag(OPT_ZERO);
    let silent = matches.get_flag(OPT_SILENT) || matches.get_flag(OPT_QUIET);
    let verbose = matches.get_flag(OPT_VERBOSE);

    let res_mode = if matches.get_flag(OPT_CANONICALIZE)
        || matches.get_flag(OPT_CANONICALIZE_EXISTING)
        || matches.get_flag(OPT_CANONICALIZE_MISSING)
    {
        ResolveMode::Logical
    } else {
        ResolveMode::None
    };

    let can_mode = if matches.get_flag(OPT_CANONICALIZE_EXISTING) {
        MissingHandling::Existing
    } else if matches.get_flag(OPT_CANONICALIZE_MISSING) {
        MissingHandling::Missing
    } else {
        MissingHandling::Normal
    };

    let files: Vec<PathBuf> = matches
        .get_many::<OsString>(ARG_FILES)
        .map(|v| v.map(PathBuf::from).collect())
        .unwrap_or_default();

    if files.is_empty() {
        return Err(UUsageError::new(
            1,
            translate!("readlink-error-missing-operand"),
        ));
    }

    if no_trailing_delimiter && files.len() > 1 && !silent {
        show_error!("{}", translate!("readlink-error-ignoring-no-newline"));
        no_trailing_delimiter = false;
    }

    let line_ending = if no_trailing_delimiter {
        None
    } else {
        Some(LineEnding::from_zero_flag(use_zero))
    };

    for f in &files {
        let p = f;
        let path_result = if res_mode == ResolveMode::None {
            fs::read_link(p)
        } else {
            canonicalize(p, can_mode, res_mode)
        };

        match path_result {
            Ok(path) => {
                show(&path, line_ending).map_err_context(String::new)?;
            }
            Err(err) => {
                return if verbose {
                    Err(USimpleError::new(
                        1,
                        err.map_err_context(move || f.to_string_lossy().to_string())
                            .to_string(),
                    ))
                } else {
                    Err(1.into())
                };
            }
        }
    }
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(translate!("readlink-about"))
        .override_usage(format_usage(&translate!("readlink-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(OPT_CANONICALIZE)
                .short('f')
                .long(OPT_CANONICALIZE)
                .help(translate!("readlink-help-canonicalize"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_CANONICALIZE_EXISTING)
                .short('e')
                .long("canonicalize-existing")
                .help(translate!("readlink-help-canonicalize-existing"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_CANONICALIZE_MISSING)
                .short('m')
                .long(OPT_CANONICALIZE_MISSING)
                .help(translate!("readlink-help-canonicalize-missing"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_NO_NEWLINE)
                .short('n')
                .long(OPT_NO_NEWLINE)
                .help(translate!("readlink-help-no-newline"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_QUIET)
                .short('q')
                .long(OPT_QUIET)
                .help(translate!("readlink-help-quiet"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_SILENT)
                .short('s')
                .long(OPT_SILENT)
                .help(translate!("readlink-help-silent"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_VERBOSE)
                .short('v')
                .long(OPT_VERBOSE)
                .help(translate!("readlink-help-verbose"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_ZERO)
                .short('z')
                .long(OPT_ZERO)
                .help(translate!("readlink-help-zero"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(ARG_FILES)
                .action(ArgAction::Append)
                .value_parser(clap::value_parser!(OsString))
                .value_hint(clap::ValueHint::AnyPath),
        )
}

fn show(path: &Path, line_ending: Option<LineEnding>) -> std::io::Result<()> {
    use uucore::display::print_verbatim;
    print_verbatim(path)?;
    if let Some(line_ending) = line_ending {
        print!("{line_ending}");
    }
    stdout().flush()
}
