// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) fullname

use clap::builder::ValueParser;
use clap::{Arg, ArgAction, Command};
use std::ffi::{OsStr, OsString};
use uucore::display::{Quotable, print_verbatim};
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
        print_verbatim(basename(path, &suffix))?;
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

fn basename_bytes<'a>(path: &'a [u8], suffix: &'_ [u8]) -> &'a [u8] {
    // Skip any trailing slashes
    let Some(i) = path.iter().rposition(|&b| b != b'/') else {
        return if path.is_empty() { b"" } else { b"/" }; // path was all slashes
    };
    // Extract final component
    let j = path[..i].iter().rposition(|&b| b == b'/');
    let base = &path[j.map_or(0, |j| j + 1)..=i];
    // Remove suffix if it's not the entire basename
    if let Some(stripped @ [_, ..]) = base.strip_suffix(suffix) {
        stripped
    } else {
        base
    }
}

fn basename<'a>(path: &'a OsStr, suffix: &OsStr) -> &'a OsStr {
    let path_bytes = path.as_encoded_bytes();
    let suffix_bytes = suffix.as_encoded_bytes();
    let base_bytes = basename_bytes(path_bytes, suffix_bytes);
    // SAFETY: The internal encoding of OsStr is documented to be a
    // self-synchronizing superset of UTF-8. Since base_bytes was computed as a
    // subslice of path_bytes adjacent only to b'/' and suffix_bytes, it is also
    // valid as an OsStr. (The experimental os_str_slice feature may allow this
    // to be rewritten without unsafe in the future.)
    unsafe { OsStr::from_encoded_bytes_unchecked(base_bytes) }
}
