// This file is part of the uutils coreutils package.
//
// (c) Jimmy Lu <jimmy.lu.2011@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) fullname

#[macro_use]
extern crate uucore;

pub mod app;

use std::path::{is_separator, PathBuf};
use uucore::InvalidEncodingHandling;

use crate::app::get_app;

fn get_usage() -> String {
    format!(
        "{0} NAME [SUFFIX]
    {0} OPTION... NAME...",
        executable!()
    )
}

pub mod options {
    pub static MULTIPLE: &str = "multiple";
    pub static NAME: &str = "name";
    pub static SUFFIX: &str = "suffix";
    pub static ZERO: &str = "zero";
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();
    let usage = get_usage();
    //
    // Argument parsing
    //
    let matches = get_app(executable!())
        .usage(&usage[..])
        .get_matches_from(args);

    // too few arguments
    if !matches.is_present(options::NAME) {
        crash!(
            1,
            "{1}\nTry '{0} --help' for more information.",
            executable!(),
            "missing operand"
        );
    }

    let opt_suffix = matches.is_present(options::SUFFIX);
    let opt_multiple = matches.is_present(options::MULTIPLE);
    let opt_zero = matches.is_present(options::ZERO);
    let multiple_paths = opt_suffix || opt_multiple;
    // too many arguments
    if !multiple_paths && matches.occurrences_of(options::NAME) > 2 {
        crash!(
            1,
            "extra operand '{1}'\nTry '{0} --help' for more information.",
            executable!(),
            matches.values_of(options::NAME).unwrap().nth(2).unwrap()
        );
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

    0
}

fn basename(fullname: &str, suffix: &str) -> String {
    // Remove all platform-specific path separators from the end
    let path = fullname.trim_end_matches(is_separator);

    // Convert to path buffer and get last path component
    let pb = PathBuf::from(path);
    match pb.components().last() {
        Some(c) => strip_suffix(c.as_os_str().to_str().unwrap(), suffix),
        None => "".to_owned(),
    }
}

// can be replaced with strip_suffix once MSRV is 1.45
#[allow(clippy::manual_strip)]
fn strip_suffix(name: &str, suffix: &str) -> String {
    if name == suffix {
        return name.to_owned();
    }

    if name.ends_with(suffix) {
        return name[..name.len() - suffix.len()].to_owned();
    }

    name.to_owned()
}
