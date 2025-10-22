// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::cmp;
use std::ffi::OsString;
use std::io::{self, Write};
use std::process;

use clap::Command;

use coreutils::validation;

const VERSION: &str = env!("CARGO_PKG_VERSION");

include!(concat!(env!("OUT_DIR"), "/uutils_map.rs"));

fn usage<T>(utils: &UtilityMap<T>, name: &str) {
    println!("{name} {VERSION} (multi-call binary)\n");
    println!("Usage: {name} [function [arguments...]]");
    println!("       {name} --list\n");
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

    // binary name equals util name?
    if let Some(&(uumain, _)) = utils.get(binary_as_util) {
        validation::setup_localization_or_exit(binary_as_util);
        process::exit(uumain(vec![binary.into()].into_iter().chain(args)));
    }

    // binary name equals prefixed util name?
    // * prefix/stem may be any string ending in a non-alphanumeric character
    // For example, if the binary is named `uu_test`, it will match `test` as a utility.
    let util_name =
        if let Some(util) = validation::find_prefixed_util(binary_as_util, utils.keys().copied()) {
            // prefixed util => replace 0th (aka, executable name) argument
            Some(OsString::from(util))
        } else {
            // unmatched binary name => regard as multi-binary container and advance argument list
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
                                io::stdout().flush().expect("could not flush stdout");
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
