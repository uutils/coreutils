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
        process::exit(1);
    }
}

/// Entry into Coreutils
///
/// # Arguments
/// * first arg needs to be the binary/executable. \
///   This is usually coreutils, but can be the util name itself, e.g. 'ls'. \
///   It can also be extended with "box" for some tests with busybox. \
///   The util name will be checked against the list of enabled utils, where
///   * the name exactly matches the name of an applet/util or
///   * the name matches <PREFIX><UTIL_NAME> pattern, e.g.
///     'my_own_directory_service_ls' as long as the last letters match the utility.
/// * coreutils arg: --list, --version, -V, --help, -h (or shortened long versions): \
///   Output information about coreutils itself. \
/// * util name and any number of arguments: \
///   Will get passed on to the selected utility. \
///   Error if util name is not recognized.#[allow(clippy::cognitive_complexity)]
fn main() {
    let utils = util_map();
    let mut args = uucore::args_os();

    let binary = validation::binary_path(&mut args);
    let binary_as_util = validation::name(&binary).unwrap_or_else(|| {
        usage(&utils, "<unknown binary name>");
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
            // Not UTF-8
            validation::not_found(&util_os)
        };

        // Util in known list?
        if let Some(&(uumain, _)) = utils.get(util) {
            // TODO: plug the deactivation of the translation
            // and load the English strings directly at compilation time in the
            // binary to avoid the load of the flt
            // Could be something like:
            // #[cfg(not(feature = "only_english"))]
            validation::setup_localization_or_exit(util);
            process::exit(uumain(vec![util_os].into_iter().chain(args)));
        } else {
            let l = util.len();
            // GNU coreutils --help string shows help for coreutils
            if util == "-h" || (l <= 6 && util[0..l] == "--help"[0..l]) {
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
            // GNU coreutils --list string shows available utilities as list
            } else if l <= 6 && util[0..l] == "--list"[0..l] {
                // we should fail with additional args https://github.com/uutils/coreutils/issues/11383#issuecomment-4082564058
                if args.next().is_some() {
                    let _ = writeln!(io::stderr(), "coreutils: invalid argument");
                    process::exit(1);
                }
                let mut out = io::stdout().lock();
                for util in utils.keys() {
                    if let Err(e) = writeln!(out, "{util}")
                        && e.kind() != io::ErrorKind::BrokenPipe
                    {
                        let _ = writeln!(io::stderr(), "coreutils: {}", strip_errno(&e));
                        process::exit(1);
                    }
                }
                process::exit(0);
            // GNU coreutils --version string shows version
            } else if util == "-V" || (l <= 9 && util[0..l] == "--version"[0..l]) {
                if let Err(e) = writeln!(io::stdout(), "coreutils {VERSION} (multi-call binary)")
                    && e.kind() != io::ErrorKind::BrokenPipe
                {
                    let _ = writeln!(io::stderr(), "coreutils: {}", strip_errno(&e));
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
    } else {
        // GNU just fails, but busybox tests needs usage
        // todo: patch the test suite instead
        if binary_as_util.ends_with("box") {
            usage(&utils, binary_as_util);
        } else {
            let _ = writeln!(io::stderr(), "coreutils: missing argument");
        }
        process::exit(1);
    }
}
