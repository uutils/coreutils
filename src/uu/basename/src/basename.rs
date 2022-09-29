// This file is part of the uutils coreutils package.
//
// (c) Jimmy Lu <jimmy.lu.2011@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) fullname

use clap::{crate_version, Arg, ArgAction, Command};
use std::path::{is_separator, PathBuf};
use uucore::display::Quotable;
use uucore::error::{UResult, UUsageError};
use uucore::format_usage;

static ABOUT: &str = r#"Print NAME with any leading directory components removed
If specified, also remove a trailing SUFFIX"#;

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
    let args = args.collect_lossy();

    // Since options have to go before names,
    // if the first argument is not an option, then there is no option,
    // and that implies there is exactly one name (no option => no -a option),
    // so simple format is used
    if args.len() > 1 && !args[1].starts_with('-') {
        if args.len() > 3 {
            return Err(UUsageError::new(
                1,
                format!("extra operand {}", args[3].to_string().quote()),
            ));
        }
        let suffix = if args.len() > 2 { args[2].as_ref() } else { "" };
        println!("{}", basename(&args[1], suffix));
        return Ok(());
    }

    //
    // Argument parsing
    //
    let matches = uu_app().try_get_matches_from(args)?;

    // too few arguments
    if !matches.contains_id(options::NAME) {
        return Err(UUsageError::new(1, "missing operand".to_string()));
    }

    let opt_suffix = matches.get_one::<String>(options::SUFFIX).is_some();
    let opt_multiple = matches.get_flag(options::MULTIPLE);
    let opt_zero = matches.get_flag(options::ZERO);
    let multiple_paths = opt_suffix || opt_multiple;
    let name_args_count = matches
        .get_many::<String>(options::NAME)
        .map(|n| n.len())
        .unwrap_or(0);

    // too many arguments
    if !multiple_paths && name_args_count > 2 {
        return Err(UUsageError::new(
            1,
            format!(
                "extra operand {}",
                matches
                    .get_many::<String>(options::NAME)
                    .unwrap()
                    .nth(2)
                    .unwrap()
                    .quote()
            ),
        ));
    }

    let suffix = if opt_suffix {
        matches.get_one::<String>(options::SUFFIX).unwrap()
    } else if !opt_multiple && name_args_count > 1 {
        matches
            .get_many::<String>(options::NAME)
            .unwrap()
            .nth(1)
            .unwrap()
    } else {
        ""
    };

    //
    // Main Program Processing
    //

    let paths: Vec<_> = if multiple_paths {
        matches.get_many::<String>(options::NAME).unwrap().collect()
    } else {
        matches
            .get_many::<String>(options::NAME)
            .unwrap()
            .take(1)
            .collect()
    };

    let line_ending = if opt_zero { "\0" } else { "\n" };
    for path in paths {
        print!("{}{}", basename(path, suffix), line_ending);
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
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::NAME)
                .action(clap::ArgAction::Append)
                .value_hint(clap::ValueHint::AnyPath)
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
                .help("end each output line with NUL, not newline")
                .action(ArgAction::SetTrue),
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
