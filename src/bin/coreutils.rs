// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::Command;
use coreutils::validation;
use itertools::Itertools as _;
use std::cmp;
use std::ffi::OsString;
use std::io::{self, Write};
use std::process;
use uucore::Args;

const VERSION: &str = env!("CARGO_PKG_VERSION");

include!(concat!(env!("OUT_DIR"), "/uutils_map.rs"));

fn usage<T>(utils: &UtilityMap<T>, name: &str) -> bool {
    let mut out = io::stdout();
    let ok = writeln!(out, "{name} {VERSION} (multi-call binary)\n").is_ok()
        && writeln!(out, "Usage: {name} [function [arguments...]]").is_ok()
        && writeln!(out, "       {name} --list").is_ok()
        && writeln!(out).is_ok();
    #[cfg(feature = "feat_common_core")]
    let ok = ok
        && writeln!(out, "Functions:").is_ok()
        && writeln!(out, "      '<uutils>' [arguments...]").is_ok()
        && writeln!(out).is_ok();
    let display_list = utils.keys().copied().join(", ");
    let width = cmp::min(textwrap::termwidth(), 100) - 4 * 2;
    ok && writeln!(out, "Options:").is_ok()
        && writeln!(
            out,
            "      --list    lists all defined functions, one per row\n"
        )
        .is_ok()
        && writeln!(out, "Currently defined functions:\n").is_ok()
        && writeln!(
            out,
            "{}",
            textwrap::indent(&textwrap::fill(&display_list, width), "    ")
        )
        .is_ok()
}

#[allow(clippy::cognitive_complexity)]
fn main() {
    uucore::panic::mute_sigpipe_panic();

    let utils = util_map();
    let mut args = uucore::args_os();

    let binary = validation::binary_path(&mut args);
    let binary_as_util = validation::name(&binary).unwrap_or_else(|| {
        if !usage(&utils, "<unknown binary name>") {
            process::exit(1);
        }
        process::exit(0);
    });

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
        validation::not_found(&OsString::from(binary_as_util));
    };

    // 0th argument equals util name?
    if let Some(util_os) = util_name {
        let Some(util) = util_os.to_str() else {
            validation::not_found(&util_os)
        };

        match util {
            "--list" => {
                // If --help is also present, show usage instead of list
                if args.any(|arg| arg == "--help" || arg == "-h") {
                    if !usage(&utils, binary_as_util) {
                        process::exit(1);
                    }
                    process::exit(0);
                }
                let utils: Vec<_> = utils.keys().collect();
                for util in utils {
                    if writeln!(io::stdout(), "{util}").is_err() {
                        process::exit(1);
                    }
                }
                process::exit(0);
            }
            "--version" | "-V" => {
                if writeln!(
                    io::stdout(),
                    "{binary_as_util} {VERSION} (multi-call binary)"
                )
                .is_err()
                {
                    process::exit(1);
                }
                process::exit(0);
            }
            // Not a special command: fallthrough to calling a util
            _ => {}
        }

        match utils.get(util) {
            Some(&(uumain, _)) => {
                // TODO: plug the deactivation of the translation
                // and load the English strings directly at compilation time in the
                // binary to avoid the load of the flt
                // Could be something like:
                // #[cfg(not(feature = "only_english"))]
                validation::setup_localization_or_exit(util);
                process::exit(uumain(vec![util_os].into_iter().chain(args)));
            }
            None => {
                if util == "--help" || util == "-h" {
                    // see if they want help on a specific util
                    if let Some(util_os) = args.next() {
                        let Some(util) = util_os.to_str() else {
                            validation::not_found(&util_os)
                        };

                        match utils.get(util) {
                            Some(&(uumain, _)) => {
                                let code = uumain(
                                    vec![util_os, OsString::from("--help")]
                                        .into_iter()
                                        .chain(args),
                                );
                                let _ = io::stdout().flush();
                                process::exit(code);
                            }
                            None => validation::not_found(&util_os),
                        }
                    }
                    if !usage(&utils, binary_as_util) {
                        process::exit(1);
                    }
                    process::exit(0);
                } else if util.starts_with('-') {
                    // Argument looks like an option but wasn't recognized
                    validation::unrecognized_option(binary_as_util, &util_os);
                } else {
                    validation::not_found(&util_os);
                }
            }
        }
    } else {
        // no arguments provided
        if !usage(&utils, binary_as_util) {
            process::exit(1);
        }
        process::exit(0);
    }
}
