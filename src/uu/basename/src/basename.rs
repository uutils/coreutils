// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) fullname

use clap::{Arg, ArgAction, Command};
use std::collections::HashMap;
use std::path::PathBuf;
use uucore::display::Quotable;
use uucore::error::{UResult, UUsageError};
use uucore::format_usage;
use uucore::line_ending::LineEnding;

use uucore::locale::{get_message, get_message_with_args};

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
        return Err(UUsageError::new(
            1,
            get_message("basename-error-missing-operand"),
        ));
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
                    get_message_with_args(
                        "basename-error-extra-operand",
                        HashMap::from([("operand".to_string(), name_args[2].quote().to_string())]),
                    ),
                ));
            }
        }
    };

    //
    // Main Program Processing
    //

    for path in name_args {
        print!("{}{line_ending}", basename(path, &suffix));
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(get_message("basename-about"))
        .override_usage(format_usage(&get_message("basename-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(options::MULTIPLE)
                .short('a')
                .long(options::MULTIPLE)
                .help(get_message("basename-help-multiple"))
                .action(ArgAction::SetTrue)
                .overrides_with(options::MULTIPLE),
        )
        .arg(
            Arg::new(options::NAME)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath)
                .hide(true)
                .trailing_var_arg(true),
        )
        .arg(
            Arg::new(options::SUFFIX)
                .short('s')
                .long(options::SUFFIX)
                .value_name("SUFFIX")
                .help(get_message("basename-help-suffix"))
                .overrides_with(options::SUFFIX),
        )
        .arg(
            Arg::new(options::ZERO)
                .short('z')
                .long(options::ZERO)
                .help(get_message("basename-help-zero"))
                .action(ArgAction::SetTrue)
                .overrides_with(options::ZERO),
        )
}

fn basename(fullname: &str, suffix: &str) -> String {
    // Convert to path buffer and get last path component
    let pb = PathBuf::from(fullname);

    pb.components().next_back().map_or_else(String::new, |c| {
        let name = c.as_os_str().to_str().unwrap();
        if name == suffix {
            name.to_string()
        } else {
            name.strip_suffix(suffix).unwrap_or(name).to_string()
        }
    })
}
