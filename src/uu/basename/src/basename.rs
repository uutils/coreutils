// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) fullname

use clap::{crate_version, Arg, ArgAction, Command};
use std::path::{is_separator, PathBuf};
use uucore::display::Quotable;
use uucore::error::{UResult, UUsageError};
use uucore::line_ending::LineEnding;
use uucore::{format_usage, help_about, help_usage};

static ABOUT: &str = help_about!("basename.md");

const USAGE: &str = help_usage!("basename.md");

pub mod options {
    pub static MULTIPLE: &str = "multiple";
    pub static NAME: &str = "name";
    pub static SUFFIX: &str = "suffix";
    pub static ZERO: &str = "zero";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args.collect_lossy();

    //
    // Argument parsing
    //
    let matches = uu_app().try_get_matches_from(args)?;

    let line_ending = LineEnding::from_zero_flag(matches.get_flag(options::ZERO));

    let mut name_args = matches
        .get_many::<String>(options::NAME)
        .unwrap_or_default()
        .collect::<Vec<_>>();
    if name_args.is_empty() {
        return Err(UUsageError::new(1, "missing operand".to_string()));
    }
    let multiple_paths =
        matches.get_one::<String>(options::SUFFIX).is_some() || matches.get_flag(options::MULTIPLE);
    let suffix = if multiple_paths {
        matches
            .get_one::<String>(options::SUFFIX)
            .cloned()
            .unwrap_or_default()
    } else {
        // "simple format"
        match name_args.len() {
            0 => panic!("already checked"),
            1 => String::default(),
            2 => name_args.pop().unwrap().clone(),
            _ => {
                return Err(UUsageError::new(
                    1,
                    format!("extra operand {}", name_args[2].quote(),),
                ));
            }
        }
    };

    //
    // Main Program Processing
    //

    for path in name_args {
        print!("{}{}", basename(path, &suffix), line_ending);
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::MULTIPLE)
                .short('a')
                .long(options::MULTIPLE)
                .help("support multiple arguments and treat each as a NAME")
                .action(ArgAction::SetTrue)
                .overrides_with(options::MULTIPLE),
        )
        .arg(
            Arg::new(options::NAME)
                .action(clap::ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath)
                .hide(true)
                .trailing_var_arg(true),
        )
        .arg(
            Arg::new(options::SUFFIX)
                .short('s')
                .long(options::SUFFIX)
                .value_name("SUFFIX")
                .help("remove a trailing SUFFIX; implies -a")
                .overrides_with(options::SUFFIX),
        )
        .arg(
            Arg::new(options::ZERO)
                .short('z')
                .long(options::ZERO)
                .help("end each output line with NUL, not newline")
                .action(ArgAction::SetTrue)
                .overrides_with(options::ZERO),
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

        None => String::new(),
    }
}
