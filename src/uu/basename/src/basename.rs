// This file is part of the uutils coreutils package.
//
// (c) Jimmy Lu <jimmy.lu.2011@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) fullname

#[macro_use]
extern crate uucore;

use clap::{crate_version, App, Arg};
use std::path::{is_separator, PathBuf};
use uucore::InvalidEncodingHandling;

static SUMMARY: &str = "Print NAME with any leading directory components removed
If specified, also remove a trailing SUFFIX";

fn usage() -> String {
    format!(
        "{0} NAME [SUFFIX]
    {0} OPTION... NAME...",
        uucore::execution_phrase()
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
    let usage = usage();
    //
    // Argument parsing
    //
    let matches = uu_app().usage(&usage[..]).get_matches_from(args);

    // too few arguments
    if !matches.is_present(options::NAME) {
        crash!(
            1,
            "{1}\nTry '{0} --help' for more information.",
            uucore::execution_phrase(),
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
            uucore::execution_phrase(),
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

pub fn uu_app() -> App<'static, 'static> {
    App::new(uucore::util_name())
        .version(crate_version!())
        .about(SUMMARY)
        .arg(
            Arg::with_name(options::MULTIPLE)
                .short("a")
                .long(options::MULTIPLE)
                .help("support multiple arguments and treat each as a NAME"),
        )
        .arg(Arg::with_name(options::NAME).multiple(true).hidden(true))
        .arg(
            Arg::with_name(options::SUFFIX)
                .short("s")
                .long(options::SUFFIX)
                .value_name("SUFFIX")
                .help("remove a trailing SUFFIX; implies -a"),
        )
        .arg(
            Arg::with_name(options::ZERO)
                .short("z")
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
            let name: &str = match c.as_os_str().to_str() {
                Some(s) => s,
                None => "",
            };

            if name == suffix {
                return name.to_owned();
            }

            match name.strip_suffix(suffix) {
                Some(s) => s.to_owned(),
                None => {
                    if cfg!(windows) {
                        "\\".to_owned()
                    } else {
                        "/".to_owned()
                    }
                }
            }
        }
        None => "".to_owned(),
    }
}
