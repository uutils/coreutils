// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) errno

use std::fs;
use std::io::{stdout, Write};
use std::path::{Path, PathBuf};
use uucore::display::Quotable;
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};
use uucore::fs::{canonicalize, MissingHandling, ResolveMode};
use uucore::line_ending::LineEnding;
use uucore::show_error;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = crate::uu_app().try_get_matches_from(args)?;

    let mut no_trailing_delimiter = matches.get_flag(crate::options::OPT_NO_NEWLINE);
    let use_zero = matches.get_flag(crate::options::OPT_ZERO);
    let silent =
        matches.get_flag(crate::options::OPT_SILENT) || matches.get_flag(crate::options::OPT_QUIET);
    let verbose = matches.get_flag(crate::options::OPT_VERBOSE);

    let res_mode = if matches.get_flag(crate::options::OPT_CANONICALIZE)
        || matches.get_flag(crate::options::OPT_CANONICALIZE_EXISTING)
        || matches.get_flag(crate::options::OPT_CANONICALIZE_MISSING)
    {
        ResolveMode::Logical
    } else {
        ResolveMode::None
    };

    let can_mode = if matches.get_flag(crate::options::OPT_CANONICALIZE_EXISTING) {
        MissingHandling::Existing
    } else if matches.get_flag(crate::options::OPT_CANONICALIZE_MISSING) {
        MissingHandling::Missing
    } else {
        MissingHandling::Normal
    };

    let files: Vec<String> = matches
        .get_many::<String>(crate::options::ARG_FILES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();
    if files.is_empty() {
        return Err(UUsageError::new(1, "missing operand"));
    }

    if no_trailing_delimiter && files.len() > 1 && !silent {
        show_error!("ignoring --no-newline with multiple arguments");
        no_trailing_delimiter = false;
    }
    let line_ending = if no_trailing_delimiter {
        None
    } else {
        Some(LineEnding::from_zero_flag(use_zero))
    };

    for f in &files {
        let p = PathBuf::from(f);
        let path_result = if res_mode == ResolveMode::None {
            fs::read_link(&p)
        } else {
            canonicalize(&p, can_mode, res_mode)
        };
        match path_result {
            Ok(path) => {
                show(&path, line_ending).map_err_context(String::new)?;
            }
            Err(err) => {
                if verbose {
                    return Err(USimpleError::new(
                        1,
                        err.map_err_context(move || f.maybe_quote().to_string())
                            .to_string(),
                    ));
                } else {
                    return Err(1.into());
                }
            }
        }
    }
    Ok(())
}

fn show(path: &Path, line_ending: Option<LineEnding>) -> std::io::Result<()> {
    let path = path.to_str().unwrap();
    print!("{path}");
    if let Some(line_ending) = line_ending {
        print!("{line_ending}");
    }
    stdout().flush()
}
