// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore manpages mangen

use clap::{Arg, Command};
use clap_complete::Shell;
use std::cmp;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use uucore::display::Quotable;

const VERSION: &str = env!("CARGO_PKG_VERSION");

include!(concat!(env!("OUT_DIR"), "/uutils_map.rs"));

fn usage<T>(utils: &UtilityMap<T>, name: &str) {
    println!("{name} {VERSION} (multi-call binary)\n");
    println!("Usage: {name} [function [arguments...]]\n");
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

fn binary_path(args: &mut impl Iterator<Item = OsString>) -> PathBuf {
    match args.next() {
        Some(ref s) if !s.is_empty() => PathBuf::from(s),
        _ => std::env::current_exe().unwrap(),
    }
}

fn name(binary_path: &Path) -> Option<&str> {
    binary_path.file_stem()?.to_str()
}

#[allow(clippy::cognitive_complexity)]
fn main() {
    uucore::panic::mute_sigpipe_panic();

    let utils = util_map();
    let mut args = uucore::args_os();

    let binary = binary_path(&mut args);
    let binary_as_util = name(&binary).unwrap_or_else(|| {
        usage(&utils, "<unknown binary name>");
        process::exit(0);
    });

    // binary name equals util name?
    if let Some(&(uumain, _)) = utils.get(binary_as_util) {
        process::exit(uumain((vec![binary.into()].into_iter()).chain(args)));
    }

    // binary name equals prefixed util name?
    // * prefix/stem may be any string ending in a non-alphanumeric character
    let util_name = if let Some(util) = utils.keys().find(|util| {
        binary_as_util.ends_with(*util)
            && !binary_as_util[..binary_as_util.len() - (*util).len()]
                .ends_with(char::is_alphanumeric)
    }) {
        // prefixed util => replace 0th (aka, executable name) argument
        Some(OsString::from(*util))
    } else {
        // unmatched binary name => regard as multi-binary container and advance argument list
        uucore::set_utility_is_second_arg();
        args.next()
    };

    // 0th argument equals util name?
    if let Some(util_os) = util_name {
        fn not_found(util: &OsStr) -> ! {
            println!("{}: function/utility not found", util.maybe_quote());
            process::exit(1);
        }

        let util = match util_os.to_str() {
            Some(util) => util,
            None => not_found(&util_os),
        };

        if util == "completion" {
            gen_completions(args, &utils);
        }

        if util == "manpage" {
            gen_manpage(args, &utils);
        }

        match utils.get(util) {
            Some(&(uumain, _)) => {
                process::exit(uumain((vec![util_os].into_iter()).chain(args)));
            }
            None => {
                if util == "--help" || util == "-h" {
                    // see if they want help on a specific util
                    if let Some(util_os) = args.next() {
                        let util = match util_os.to_str() {
                            Some(util) => util,
                            None => not_found(&util_os),
                        };

                        match utils.get(util) {
                            Some(&(uumain, _)) => {
                                let code = uumain(
                                    (vec![util_os, OsString::from("--help")].into_iter())
                                        .chain(args),
                                );
                                io::stdout().flush().expect("could not flush stdout");
                                process::exit(code);
                            }
                            None => not_found(&util_os),
                        }
                    }
                    usage(&utils, binary_as_util);
                    process::exit(0);
                } else {
                    not_found(&util_os);
                }
            }
        }
    } else {
        // no arguments provided
        usage(&utils, binary_as_util);
        process::exit(0);
    }
}

/// Prints completions for the utility in the first parameter for the shell in the second parameter to stdout
fn gen_completions<T: uucore::Args>(
    args: impl Iterator<Item = OsString>,
    util_map: &UtilityMap<T>,
) -> ! {
    let all_utilities: Vec<_> = std::iter::once("coreutils")
        .chain(util_map.keys().copied())
        .collect();

    let matches = Command::new("completion")
        .about("Prints completions to stdout")
        .arg(
            Arg::new("utility")
                .value_parser(clap::builder::PossibleValuesParser::new(all_utilities))
                .required(true),
        )
        .arg(
            Arg::new("shell")
                .value_parser(clap::builder::EnumValueParser::<Shell>::new())
                .required(true),
        )
        .get_matches_from(std::iter::once(OsString::from("completion")).chain(args));

    let utility = matches.get_one::<String>("utility").unwrap();
    let shell = *matches.get_one::<Shell>("shell").unwrap();

    let mut command = if utility == "coreutils" {
        gen_coreutils_app(util_map)
    } else {
        util_map.get(utility).unwrap().1()
    };
    let bin_name = std::env::var("PROG_PREFIX").unwrap_or_default() + utility;

    clap_complete::generate(shell, &mut command, bin_name, &mut io::stdout());
    io::stdout().flush().unwrap();
    process::exit(0);
}

/// Generate the manpage for the utility in the first parameter
fn gen_manpage<T: uucore::Args>(
    args: impl Iterator<Item = OsString>,
    util_map: &UtilityMap<T>,
) -> ! {
    let all_utilities: Vec<_> = std::iter::once("coreutils")
        .chain(util_map.keys().copied())
        .collect();

    let matches = Command::new("manpage")
        .about("Prints manpage to stdout")
        .arg(
            Arg::new("utility")
                .value_parser(clap::builder::PossibleValuesParser::new(all_utilities))
                .required(true),
        )
        .get_matches_from(std::iter::once(OsString::from("manpage")).chain(args));

    let utility = matches.get_one::<String>("utility").unwrap();

    let command = if utility == "coreutils" {
        gen_coreutils_app(util_map)
    } else {
        util_map.get(utility).unwrap().1()
    };

    let man = clap_mangen::Man::new(command);
    man.render(&mut io::stdout())
        .expect("Man page generation failed");
    io::stdout().flush().unwrap();
    process::exit(0);
}

fn gen_coreutils_app<T: uucore::Args>(util_map: &UtilityMap<T>) -> Command {
    let mut command = Command::new("coreutils");
    for (name, (_, sub_app)) in util_map {
        // Recreate a small subcommand with only the relevant info
        // (name & short description)
        let about = sub_app()
            .get_about()
            .expect("Could not get the 'about'")
            .to_string();
        let sub_app = Command::new(name).about(about);
        command = command.subcommand(sub_app);
    }
    command
}
