// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::Command;
use coreutils::validation::{self, exit};
use itertools::Itertools as _;
use std::cmp;
use std::ffi::OsString;
use std::io::{self, Write};
use std::process::ExitCode;
use uucore::{Args, error::strip_errno};

const VERSION: &str = env!("CARGO_PKG_VERSION");

include!(concat!(env!("OUT_DIR"), "/uutils_map.rs"));

fn usage<T>(utils: &UtilityMap<T>, name: &str) {
    let display_list = utils.keys().copied().join(", ");
    let width = cmp::min(textwrap::termwidth(), 100) - 8; // (opinion/heuristic) max 100 chars wide with 4 character side indentions
    let indent_list = textwrap::indent(&textwrap::fill(&display_list, width), "    ");
    #[cfg(feature = "feat_common_core")]
    let common_core_string = "
Functions:
      '<uutils>' [arguments...]

";
    #[cfg(not(feature = "feat_common_core"))]
    let common_core_string = "";
    let s = format!(
        "{name} {VERSION} (multi-call binary)

Usage: {name} [function [arguments...]]
       {name} --list

{common_core_string}Options:
      --list    lists all defined functions, one per row

Currently defined functions:

{indent_list}"
    );
    if let Err(e) = writeln!(io::stdout(), "{s}")
        && e.kind() != io::ErrorKind::BrokenPipe
    {
        let _ = writeln!(io::stderr(), "coreutils: {}", strip_errno(&e));
        exit(1);
    }
}

#[allow(clippy::cognitive_complexity)]
fn main() -> ExitCode {
    let utils = util_map();
    let mut args = uucore::args_os();

    let binary = validation::binary_path(&mut args);
    let Some(binary_as_util) = validation::name(&binary) else {
        usage(&utils, "<unknown binary name>");
        return ExitCode::SUCCESS;
    };

    // binary name ends with util name?
    let is_coreutils = binary_as_util.ends_with("utils");
    let matched_util = utils
        .keys()
        .filter(|&&u| binary_as_util.ends_with(u) && !is_coreutils)
        .max_by_key(|u| u.len()); //Prefer stty more than tty. *utils is not ls

    let util_name = if let Some(&util) = matched_util {
        Some(OsString::from(util))
    } else if is_coreutils || binary_as_util.ends_with("box") {
        // todo: Remove support of "*box" from binary
        uucore::set_utility_is_second_arg();
        args.next()
    } else {
        return validation::not_found(&OsString::from(binary_as_util));
    };

    // 0th argument equals util name?
    if let Some(util_os) = util_name {
        let Some(util) = util_os.to_str() else {
            return validation::not_found(&util_os);
        };

        match util {
            "--list" => {
                // we should fail with additional args https://github.com/uutils/coreutils/issues/11383#issuecomment-4082564058
                if args.next().is_some() {
                    let _ = writeln!(io::stderr(), "coreutils: invalid argument");
                    return ExitCode::FAILURE;
                }
                let mut out = io::stdout().lock();
                for util in utils.keys() {
                    if let Err(e) = writeln!(out, "{util}")
                        && e.kind() != io::ErrorKind::BrokenPipe
                    {
                        let _ = writeln!(io::stderr(), "coreutils: {}", strip_errno(&e));
                        return ExitCode::FAILURE;
                    }
                }
                return ExitCode::SUCCESS;
            }
            "--version" | "-V" => {
                if let Err(e) = writeln!(io::stdout(), "coreutils {VERSION} (multi-call binary)")
                    && e.kind() != io::ErrorKind::BrokenPipe
                {
                    let _ = writeln!(io::stderr(), "coreutils: {}", strip_errno(&e));
                    return ExitCode::FAILURE;
                }
                return ExitCode::SUCCESS;
            }
            // Not a special command: fallthrough to calling a util
            _ => {}
        }

        if let Some(&(uumain, _)) = utils.get(util) {
            // TODO: plug the deactivation of the translation
            // and load the English strings directly at compilation time in the
            // binary to avoid the load of the flt
            // Could be something like:
            // #[cfg(not(feature = "only_english"))]
            validation::setup_localization_or_exit(util);
            let code = uumain(vec![util_os].into_iter().chain(args));
            // ExitCode only holds a u8; larger codes (Windows) go through exit.
            return u8::try_from(code).map_or_else(|_| exit(code), ExitCode::from);
        }
        // GNU coreutils --help string shows help for coreutils
        if util == "-h" || (util.starts_with("--h") && "--help".starts_with(util)) {
            usage(&utils, binary_as_util);
            return ExitCode::SUCCESS;
        } else if util.starts_with('-') {
            // Argument looks like an option but wasn't recognized
            return validation::unrecognized_option(binary_as_util, &util_os);
        }
        return validation::not_found(&util_os);
    }
    // GNU just fails, but busybox tests needs usage
    // todo: patch the test suite instead
    if binary_as_util.ends_with("box") {
        usage(&utils, binary_as_util);
    } else {
        let _ = writeln!(io::stderr(), "coreutils: missing argument");
    }
    ExitCode::FAILURE
}
