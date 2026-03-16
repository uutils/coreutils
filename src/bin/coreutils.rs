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
    let display_list = utils.keys().copied().join(", ");
    let width = cmp::min(textwrap::termwidth(), 100) - 4 * 2; // (opinion/heuristic) max 100 chars wide with 4 character side indentions
    println!(
        "{}",
        textwrap::indent(&textwrap::fill(&display_list, width), "    ")
    );
}

/// all defined coreutils options
const COREUTILS_OPTIONS: [&str; 5] = ["--list", "-V", "--version", "-h", "--help"];

/// Entry into Coreutils
///
/// # Arguments
/// * first arg needs to be the binary/executable. \
///   This is usually coreutils, but can be the util name itself, e.g. 'ls'. \
///   The util name will be checked against the list of enabled utils, where
///   * the name exactly matches the name of an applet/util or
///   * the name matches <PREFIX><UTIL_NAME> pattern, e.g.
///     'my_own_directory_service_ls' as long as the last letters match the utility.
/// * coreutils arg: --list, --version, -V, --help, -h (or shortened long versions): \
///   Output information about coreutils itself. \
///   Multiple of these arguments, output limited to one, with help > version > list.
/// * util name and any number of arguments: \
///   Will get passed on to the selected utility. \
///   Error if util name is not recognized.
/// * --help or -h and a following util name: \
///   Output help for that specific utility. \
///   So 'coreutils sum --help' is the same as 'coreutils --help sum'.
#[allow(clippy::cognitive_complexity)]
fn main() {
    uucore::panic::mute_sigpipe_panic();

    let utils = util_map();
    let mut args = uucore::args_os();

    // get binary which is always the first argument and remove it from args
    let binary = validation::binary_path(&mut args);
    let binary_as_util = validation::name(&binary).unwrap_or_else(|| {
        // non UTF-8 name
        usage(&utils, "<unknown binary name>");
        process::exit(0);
    });

    // get the called util
    let util_os = if binary_as_util.ends_with("utils") || binary_as_util.ends_with("box") {
        // todo: Remove support of "*box" from binary, but required for busy_box tests
        // coreutils
        uucore::set_utility_is_second_arg();
        if let Some(u_name) = args.next() {
            u_name
        } else {
            // no arguments provided
            usage(&utils, binary_as_util);
            process::exit(0);
        }
    } else {
        // Is the binary name a prefixed util name?
        // Prefer stty more than tty. *utils is not ls
        let name = if let Some(matched_util) = utils
            .keys()
            .filter(|&&util_name| binary_as_util.ends_with(util_name))
            .max_by_key(|u| u.len())
        {
            *matched_util
        } else {
            binary_as_util
        };

        OsString::from(name)
    };

    let Some(util) = util_os.to_str() else {
        // non-UTF-8 name
        validation::not_found(&util_os)
    };

    if let Some(&(uumain, _)) = utils.get(util) {
        // TODO: plug the deactivation of the translation
        // and load the English strings directly at compilation time in the
        // binary to avoid the load of the flt
        // Could be something like:
        // #[cfg(not(feature = "only_english"))]
        validation::setup_localization_or_exit(util);
        process::exit(uumain(vec![util_os].into_iter().chain(args)));
    } else {
        let (option, help_util) = find_dominant_option(&util_os, &mut args);
        match option {
            SelectedOption::Help => match help_util {
                // see if they want help on a specific util and if it is valid
                Some(u_os) => match utils.get(&u_os.to_string_lossy()) {
                    Some(&(uumain, _)) => {
                        let code = uumain(
                            vec![u_os, OsString::from("--help")]
                                .into_iter()
                                // Function requires a chain like in the Some case, but
                                // the args are discarded as clap returns help immediately.
                                .chain(args),
                        );
                        io::stdout().flush().expect("could not flush stdout");
                        process::exit(code);
                    }
                    None => validation::not_found(&u_os),
                },
                // show coreutils help
                None => usage(&utils, binary_as_util),
            },
            SelectedOption::Version => {
                println!("{binary_as_util} {VERSION} (multi-call binary)");
            }
            SelectedOption::List => {
                let utils: Vec<_> = utils.keys().collect();
                for util in utils {
                    println!("{util}");
                }
            }
            SelectedOption::Unrecognized(arg) => {
                // Argument looks like an option but wasn't recognized
                validation::unrecognized_option(binary_as_util, &arg);
            }
        }
        // process::exit(0);
    }
}

/// The dominant selected option.
#[derive(Debug, Clone, PartialEq)]
enum SelectedOption {
    Help,
    Version,
    List,
    Unrecognized(OsString),
}

/// Coreutils only accepts one single option,
/// if multiple are given, use the most dominant one.
///
/// Help > Version > List (e.g. 'coreutils --list --version' will return version)
/// Unrecognized will return immediately.
///
/// # Returns
/// (SelectedOption, Util for help request, if any)
fn find_dominant_option(
    first_arg: &OsString,
    args: &mut impl Iterator<Item = OsString>,
) -> (SelectedOption, Option<OsString>) {
    let mut sel = identify_option_from_partial_text(first_arg);
    match sel {
        SelectedOption::Help => return (SelectedOption::Help, args.next()),
        SelectedOption::Unrecognized(_) => {
            return (sel, None);
        }
        _ => {}
    }
    // check remaining options, allows multiple
    while let Some(arg) = args.next() {
        let so = identify_option_from_partial_text(&arg);
        match so {
            // most dominant, return directly
            SelectedOption::Help => {
                // if help is wanted, check if a tool was named
                return (so, args.next());
            }
            // best after help, can be set directly
            SelectedOption::Version => sel = SelectedOption::Version,
            SelectedOption::List => {
                if sel != SelectedOption::Version {
                    sel = SelectedOption::List;
                }
            }
            // unrecognized is not allowed
            SelectedOption::Unrecognized(_) => {
                return (so, None);
            }
        }
    }

    (sel, None)
}

// Will identify one, SelectedOption::None cannot be returned.
fn identify_option_from_partial_text(arg: &OsString) -> SelectedOption {
    let mut option = &arg.to_string_lossy()[..];
    if let Some(p) = option.find('=') {
        option = &option[0..p];
    }
    // // don't care about hyphens, -h and --h(elp) are identical
    // let option = option.replace("-", "");
    let l = option.len();
    let possible_opts: Vec<usize> = COREUTILS_OPTIONS
        .iter()
        .enumerate()
        .filter(|(_, it)| it.len() >= l && &it[0..l] == option)
        .map(|(id, _)| id)
        .collect();

    match possible_opts.len() {
        // exactly one hit
        1 => match &possible_opts[0] {
            0 => SelectedOption::List,
            1 | 2 => SelectedOption::Version,
            _ => SelectedOption::Help,
        },
        // None or more hits. The latter can not happen with the allowed options.
        _ => SelectedOption::Unrecognized(arg.clone()),
    }
}
