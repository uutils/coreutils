// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::Command;
use coreutils::validation;
use std::cmp;
use std::ffi::OsString;
use std::io::{self, Write};
use std::process;

const VERSION: &str = env!("CARGO_PKG_VERSION");

include!(concat!(env!("OUT_DIR"), "/uutils_map.rs"));

fn usage<T>(utils: &UtilityMap<T>, name: &str) {
    println!("{name} {VERSION} (multi-call binary)\n");
    println!("Usage: {name} [function [arguments...]]");
    println!("       {name} --list");
    println!();
    #[cfg(feature = "feat_common_core")]
    {
        println!("Functions:");
        println!("      '<uutils>' [arguments...]");
        println!();
    }
    println!("Options:");
    println!("      --list    lists all defined functions, one per row\n");
    println!("Currently defined functions:\n");
    #[allow(clippy::map_clone)]
    let mut utils: Vec<&str> = utils.keys().map(|&s| s).collect();
    utils.sort_unstable();
    let display_list = utils.join(", ");
    let width = cmp::min(textwrap::termwidth(), 100) - 4 * 2; // (opinion/heuristic) max 100 chars wide with 4 character side indentions
    println!(
        "{}",
        textwrap::indent(&textwrap::fill(&display_list, width), "    ")
    );
}

#[allow(clippy::cognitive_complexity)]
fn main() {
    uucore::panic::mute_sigpipe_panic();

    let utils = util_map();
    let mut args = uucore::args_os();

    let binary = validation::binary_path(&mut args);
    let binary_as_util = validation::name(&binary).unwrap_or_else(|| {
        usage(&utils, "<unknown binary name>");
        process::exit(0);
    });

    // binary name ends with util name?
    let matched_util = utils
        .keys()
        .filter(|&&u| binary_as_util.ends_with(u) && !binary_as_util.ends_with("coreutils"))
        .max_by_key(|u| u.len()); //Prefer stty more than tty. coreutils is not ls

    let util_name = if let Some(&util) = matched_util {
        Some(OsString::from(util))
    } else {
        uucore::set_utility_is_second_arg();
        args.next()
    };

    // 0th argument equals util name?
    if let Some(util_os) = util_name {
        let Some(util) = util_os.to_str() else {
            validation::not_found(&util_os)
        };

        match util {
            "--list" => {
                let mut utils: Vec<_> = utils.keys().collect();
                utils.sort();
                for util in utils {
                    println!("{util}");
                }
                process::exit(0);
            }
            "--version" | "-V" => {
                println!("{binary_as_util} {VERSION} (multi-call binary)");
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
                let code = uumain(vec![util_os].into_iter().chain(args));
                // Flush stdout and exit with code 1 if flush fails (GNU compatibility)
                // But not for BrokenPipe - that's handled by utilities themselves
                let code = match io::stdout().flush() {
                    Ok(()) => code,
                    Err(e) if e.kind() == io::ErrorKind::BrokenPipe => code,
                    Err(_) if code == 0 => 1,
                    Err(_) => code,
                };
                process::exit(code);
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
                                // Flush stdout and exit with code 1 if flush fails (GNU compatibility)
                                // But not for BrokenPipe - that's handled by utilities themselves
                                let code = match io::stdout().flush() {
                                    Ok(()) => code,
                                    Err(e) if e.kind() == io::ErrorKind::BrokenPipe => code,
                                    Err(_) if code == 0 => 1,
                                    Err(_) => code,
                                };
                                process::exit(code);
                            }
                            None => validation::not_found(&util_os),
                        }
                    }
                    usage(&utils, binary_as_util);
                    process::exit(0);
                } else {
                    validation::not_found(&util_os);
                }
            }
        }
    } else {
        // no arguments provided
        usage(&utils, binary_as_util);
        process::exit(0);
    }
}
