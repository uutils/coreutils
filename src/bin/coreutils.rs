// This file is part of the uutils coreutils package.
//
// (c) Michael Gehring <mg@ebfe.org>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{App, Shell};
use std::cmp;
use std::collections::hash_map::HashMap;
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

const VERSION: &str = env!("CARGO_PKG_VERSION");

include!(concat!(env!("OUT_DIR"), "/uutils_map.rs"));
include!(concat!(env!("OUT_DIR"), "/apps_map.rs"));

fn usage<T>(utils: &UtilityMap<T>, name: &str) {
    println!("{} {} (multi-call binary)\n", name, VERSION);
    println!("Usage: {} [function [arguments...]]\n", name);
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

fn name(binary_path: &Path) -> &str {
    binary_path.file_stem().unwrap().to_str().unwrap()
}

fn main() {
    uucore::panic::mute_sigpipe_panic();

    let utils = util_map();
    let mut args = uucore::args_os();

    let binary = binary_path(&mut args);
    let binary_as_util = name(&binary);

    // binary name equals util name?
    if let Some(&uumain) = utils.get(binary_as_util) {
        process::exit(uumain((vec![binary.into()].into_iter()).chain(args)));
    }

    // binary name equals prefixed util name?
    // * prefix/stem may be any string ending in a non-alphanumeric character
    let util_name = if let Some(util) = utils.keys().find(|util| {
        binary_as_util.ends_with(*util)
            && !(&binary_as_util[..binary_as_util.len() - (*util).len()])
                .ends_with(char::is_alphanumeric)
    }) {
        // prefixed util => replace 0th (aka, executable name) argument
        Some(OsString::from(*util))
    } else {
        // unmatched binary name => regard as multi-binary container and advance argument list
        args.next()
    };

    // 0th argument equals util name?
    if let Some(util_os) = util_name {
        let util = util_os.as_os_str().to_string_lossy();

        if util == "completion" {
            gen_completions(args);
        }

        match utils.get(&util[..]) {
            Some(&uumain) => {
                process::exit(uumain((vec![util_os].into_iter()).chain(args)));
            }
            None => {
                if util == "--help" || util == "-h" {
                    // see if they want help on a specific util
                    if let Some(util_os) = args.next() {
                        let util = util_os.as_os_str().to_string_lossy();

                        match utils.get(&util[..]) {
                            Some(&uumain) => {
                                let code = uumain(
                                    (vec![util_os, OsString::from("--help")].into_iter())
                                        .chain(args),
                                );
                                io::stdout().flush().expect("could not flush stdout");
                                process::exit(code);
                            }
                            None => {
                                println!("{}: function/utility not found", util);
                                process::exit(1);
                            }
                        }
                    }
                    usage(&utils, binary_as_util);
                    process::exit(0);
                } else {
                    println!("{}: function/utility not found", util);
                    process::exit(1);
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
fn gen_completions(mut args: impl Iterator<Item = OsString>) -> ! {
    let app_map = app_map();
    let utility = args
        .next()
        .expect("expected utility as the first parameter")
        .to_str()
        .expect("utility name was not valid utf-8")
        .to_owned();
    let shell = args
        .next()
        .expect("expected shell as the second parameter")
        .to_str()
        .expect("shell name was not valid utf-8")
        .to_owned();
    let mut app = if utility == "coreutils" {
        gen_coreutils_app()
    } else if let Some(app) = app_map.get(utility.as_str()) {
        app(&utility)
    } else {
        eprintln!("{} is not a valid utility", utility);
        process::exit(1)
    };
    let shell: Shell = shell.parse().unwrap();
    let bin_name = std::env::var("PROG_PREFIX").unwrap_or_default() + &utility;
    app.gen_completions_to(bin_name, shell, &mut io::stdout());
    io::stdout().flush().unwrap();
    process::exit(0);
}

fn gen_coreutils_app<'a, 'b>() -> App<'a, 'b> {
    let mut app = App::new("coreutils");
    for (name, sub_app) in app_map() {
        app = app.subcommand(sub_app(name));
    }
    app
}
