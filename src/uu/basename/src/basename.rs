// This file is part of the uutils coreutils package.
//
// (c) Jimmy Lu <jimmy.lu.2011@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) fullname

use clap::{crate_version, Arg, Command};
use std::path::{is_separator, PathBuf};
use uucore::display::Quotable;
use uucore::error::{UResult, UUsageError};
use uucore::{format_usage, InvalidEncodingHandling};

static SUMMARY: &str = "Print NAME with any leading directory components removed
If specified, also remove a trailing SUFFIX";

const USAGE: &str = "{} NAME [SUFFIX]
    {} OPTION... NAME...";

pub mod options {
    pub static MULTIPLE: &str = "multiple";
    pub static NAME: &str = "name";
    pub static SUFFIX: &str = "suffix";
    pub static ZERO: &str = "zero";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();
    //
    // Argument parsing
    //
    let matches = uu_app().get_matches_from(args);

    // too few arguments
    if !matches.is_present(options::NAME) {
        return Err(UUsageError::new(1, "missing operand".to_string()));
    }

    let opt_suffix = matches.is_present(options::SUFFIX);
    let opt_multiple = matches.is_present(options::MULTIPLE);
    let opt_zero = matches.is_present(options::ZERO);
    let multiple_paths = opt_suffix || opt_multiple;
    // too many arguments
    if !multiple_paths && matches.occurrences_of(options::NAME) > 2 {
        return Err(UUsageError::new(
            1,
            format!(
                "extra operand {}",
                matches
                    .values_of(options::NAME)
                    .unwrap()
                    .nth(2)
                    .unwrap()
                    .quote()
            ),
        ));
    }

    let suffix = if opt_suffix {
        matches.value_of(options::SUFFIX).unwrap()
    } else if !opt_multiple && matches.occurrences_of(options::NAME) > 1 {
        matches.values_of(options::NAME).unwrap().nth(1).unwrap()
    } else {
        ""
    };

    //
    // Main Program Processing
    //

    let paths: Vec<_> = if multiple_paths {
        matches.values_of(options::NAME).unwrap().collect()
    } else {
        matches.values_of(options::NAME).unwrap().take(1).collect()
    };

    let line_ending = if opt_zero { "\0" } else { "\n" };
    for path in paths {
        print!("{}{}", basename(path, suffix), line_ending);
    }

    Ok(())
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(SUMMARY)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::MULTIPLE)
                .short('a')
                .long(options::MULTIPLE)
                .help("support multiple arguments and treat each as a NAME"),
        )
        .arg(
            Arg::new(options::NAME)
                .multiple_occurrences(true)
                .hide(true),
        )
        .arg(
            Arg::new(options::SUFFIX)
                .short('s')
                .long(options::SUFFIX)
                .value_name("SUFFIX")
                .help("remove a trailing SUFFIX; implies -a"),
        )
        .arg(
            Arg::new(options::ZERO)
                .short('z')
                .long(options::ZERO)
                .help("end each output line with NUL, not newline"),
        )
}

fn basename(fullname: &str, suffix: &str) -> String {
    // Remove all platform-specific path separators from the end.
    let path = fullname.trim_end_matches(is_separator);

    // If the path contained *only* suffix characters (for example, if
    // `fullname` were "///" and `suffix` were "/"), then `path` would
    // be left with the empty string. In that case, we set `path` to be
    // the original `fullname` to avoid returning the empty path.
    let path = if path.is_empty() { fullname } else { path };

    // Convert to path buffer and get last path component
    let pb = PathBuf::from(path);
    match pb.components().last() {
        Some(c) => {
            let name = c.as_os_str().to_str().unwrap();
            if name == suffix {
                name.to_string()
            } else {
                name.strip_suffix(suffix).unwrap_or(name).to_string()
            }
        }

        None => "".to_owned(),
    }
}
