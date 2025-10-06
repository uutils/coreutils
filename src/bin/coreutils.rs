// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore manpages mangen prefixcat testcat

use clap::{Arg, Command};
use clap_complete::Shell;
use std::cmp;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use uucore::display::Quotable;
use uucore::locale;

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

/// # Panics
/// Panics if the binary path cannot be determined
fn binary_path(args: &mut impl Iterator<Item = OsString>) -> PathBuf {
    match args.next() {
        Some(ref s) if !s.is_empty() => PathBuf::from(s),
        _ => std::env::current_exe().unwrap(),
    }
}

fn name(binary_path: &Path) -> Option<&str> {
    binary_path.file_stem()?.to_str()
}

fn get_canonical_util_name(util_name: &str) -> &str {
    match util_name {
        // uu_test aliases - '[' is an alias for test
        "[" => "test",

        // hashsum aliases - all these hash commands are aliases for hashsum
        "md5sum" | "sha1sum" | "sha224sum" | "sha256sum" | "sha384sum" | "sha512sum"
        | "sha3sum" | "sha3-224sum" | "sha3-256sum" | "sha3-384sum" | "sha3-512sum"
        | "shake128sum" | "shake256sum" | "b2sum" | "b3sum" => "hashsum",

        "dir" => "ls", // dir is an alias for ls

        // Default case - return the util name as is
        _ => util_name,
    }
}

fn find_prefixed_util<'a>(
    binary_name: &str,
    mut util_keys: impl Iterator<Item = &'a str>,
) -> Option<&'a str> {
    util_keys.find(|util| {
        binary_name.ends_with(*util)
            && binary_name.len() > util.len() // Ensure there's actually a prefix
            && !binary_name[..binary_name.len() - (*util).len()]
                .ends_with(char::is_alphanumeric)
    })
}

fn setup_localization_or_exit(util_name: &str) {
    locale::setup_localization(get_canonical_util_name(util_name)).unwrap_or_else(|err| {
        match err {
            uucore::locale::LocalizationError::ParseResource {
                error: err_msg,
                snippet,
            } => eprintln!("Localization parse error at {snippet}: {err_msg}"),
            other => eprintln!("Could not init the localization system: {other}"),
        }
        process::exit(99)
    });
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
        setup_localization_or_exit(binary_as_util);
        process::exit(uumain(vec![binary.into()].into_iter().chain(args)));
    }

    // binary name equals prefixed util name?
    // * prefix/stem may be any string ending in a non-alphanumeric character
    // For example, if the binary is named `uu_test`, it will match `test` as a utility.
    let util_name = if let Some(util) = find_prefixed_util(binary_as_util, utils.keys().copied()) {
        // prefixed util => replace 0th (aka, executable name) argument
        Some(OsString::from(util))
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

        let Some(util) = util_os.to_str() else {
            not_found(&util_os)
        };

        match util {
            "completion" => gen_completions(args, &utils),
            "manpage" => gen_manpage(args, &utils),
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
                setup_localization_or_exit(util);
                process::exit(uumain(vec![util_os].into_iter().chain(args)));
            }
            None => {
                if util == "--help" || util == "-h" {
                    // see if they want help on a specific util
                    if let Some(util_os) = args.next() {
                        let Some(util) = util_os.to_str() else {
                            not_found(&util_os)
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
/// # Panics
/// Panics if the utility map is empty
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
/// # Panics
/// Panics if the utility map is empty
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
        setup_localization_or_exit(utility);
        util_map.get(utility).unwrap().1()
    };

    let man = clap_mangen::Man::new(command);
    man.render(&mut io::stdout())
        .expect("Man page generation failed");
    io::stdout().flush().unwrap();
    process::exit(0);
}

/// # Panics
/// Panics if the utility map is empty
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_get_canonical_util_name() {
        // Test a few key aliases
        assert_eq!(get_canonical_util_name("["), "test");
        assert_eq!(get_canonical_util_name("md5sum"), "hashsum");
        assert_eq!(get_canonical_util_name("dir"), "ls");

        // Test passthrough case
        assert_eq!(get_canonical_util_name("cat"), "cat");
    }

    #[test]
    fn test_name() {
        // Test normal executable name
        assert_eq!(name(Path::new("/usr/bin/ls")), Some("ls"));
        assert_eq!(name(Path::new("cat")), Some("cat"));
        assert_eq!(
            name(Path::new("./target/debug/coreutils")),
            Some("coreutils")
        );

        // Test with extensions
        assert_eq!(name(Path::new("program.exe")), Some("program"));
        assert_eq!(name(Path::new("/path/to/utility.bin")), Some("utility"));

        // Test edge cases
        assert_eq!(name(Path::new("")), None);
        assert_eq!(name(Path::new("/")), None);
    }

    #[test]
    fn test_find_prefixed_util() {
        let utils = ["test", "cat", "ls", "cp"];

        // Test exact prefixed matches
        assert_eq!(
            find_prefixed_util("uu_test", utils.iter().copied()),
            Some("test")
        );
        assert_eq!(
            find_prefixed_util("my-cat", utils.iter().copied()),
            Some("cat")
        );
        assert_eq!(
            find_prefixed_util("prefix_ls", utils.iter().copied()),
            Some("ls")
        );

        // Test non-alphanumeric separator requirement
        assert_eq!(find_prefixed_util("prefixcat", utils.iter().copied()), None); // no separator
        assert_eq!(find_prefixed_util("testcat", utils.iter().copied()), None); // no separator

        // Test no match
        assert_eq!(find_prefixed_util("unknown", utils.iter().copied()), None);
        assert_eq!(find_prefixed_util("", utils.iter().copied()), None);

        // Test exact util name (should not match as prefixed)
        assert_eq!(find_prefixed_util("test", utils.iter().copied()), None);
        assert_eq!(find_prefixed_util("cat", utils.iter().copied()), None);
    }
}
