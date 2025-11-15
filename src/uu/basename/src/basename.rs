// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) fullname

use clap::builder::ValueParser;
use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use std::io::{Write, stdout};
use std::path::PathBuf;
use uucore::display::Quotable;
use uucore::error::{UResult, UUsageError};
use uucore::format_usage;
use uucore::line_ending::LineEnding;

use uucore::translate;

pub mod options {
    pub static MULTIPLE: &str = "multiple";
    pub static NAME: &str = "name";
    pub static SUFFIX: &str = "suffix";
    pub static ZERO: &str = "zero";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    //
    // Argument parsing
    //
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let line_ending = LineEnding::from_zero_flag(matches.get_flag(options::ZERO));

    let mut name_args = matches
        .get_many::<OsString>(options::NAME)
        .unwrap_or_default()
        .collect::<Vec<_>>();
    if name_args.is_empty() {
        return Err(UUsageError::new(
            1,
            translate!("basename-error-missing-operand"),
        ));
    }
    let multiple_paths = matches.get_one::<OsString>(options::SUFFIX).is_some()
        || matches.get_flag(options::MULTIPLE);
    let suffix = if multiple_paths {
        matches
            .get_one::<OsString>(options::SUFFIX)
            .cloned()
            .unwrap_or_default()
    } else {
        // "simple format"
        match name_args.len() {
            0 => panic!("already checked"),
            1 => OsString::default(),
            2 => name_args.pop().unwrap().clone(),
            _ => {
                return Err(UUsageError::new(
                    1,
                    translate!("basename-error-extra-operand",
                               "operand" => name_args[2].quote()),
                ));
            }
        }
    };

    //
    // Main Program Processing
    //

    for path in name_args {
        stdout().write_all(&basename(path, &suffix)?)?;
        print!("{line_ending}");
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("basename-about"))
        .override_usage(format_usage(&translate!("basename-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(options::MULTIPLE)
                .short('a')
                .long(options::MULTIPLE)
                .help(translate!("basename-help-multiple"))
                .action(ArgAction::SetTrue)
                .overrides_with(options::MULTIPLE),
        )
        .arg(
            Arg::new(options::NAME)
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::AnyPath)
                .hide(true)
                .trailing_var_arg(true),
        )
        .arg(
            Arg::new(options::SUFFIX)
                .short('s')
                .long(options::SUFFIX)
                .value_name("SUFFIX")
                .value_parser(ValueParser::os_string())
                .help(translate!("basename-help-suffix"))
                .overrides_with(options::SUFFIX),
        )
        .arg(
            Arg::new(options::ZERO)
                .short('z')
                .long(options::ZERO)
                .help(translate!("basename-help-zero"))
                .action(ArgAction::SetTrue)
                .overrides_with(options::ZERO),
        )
}

// We return a Vec<u8>. Returning a seemingly more proper `OsString` would
// require back and forth conversions as we need a &[u8] for printing anyway.
fn basename(fullname: &OsString, suffix: &OsString) -> UResult<Vec<u8>> {
    let fullname_bytes = uucore::os_str_as_bytes(fullname)?;

    // Handle special case where path ends with /.
    if fullname_bytes.ends_with(b"/.") {
        return Ok(b".".into());
    }

    // Convert to path buffer and get last path component
    let pb = PathBuf::from(fullname);

    pb.components().next_back().map_or(Ok([].into()), |c| {
        let name = c.as_os_str();
        let name_bytes = uucore::os_str_as_bytes(name)?;
        if name == suffix {
            Ok(name_bytes.into())
        } else {
            let suffix_bytes = uucore::os_str_as_bytes(suffix)?;
            Ok(name_bytes
                .strip_suffix(suffix_bytes)
                .unwrap_or(name_bytes)
                .into())
        }
    })
}
