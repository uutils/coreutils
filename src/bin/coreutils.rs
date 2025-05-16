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
use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use uucore::display::Quotable;
use zip::ZipArchive;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const COMPLETION: &str = "completion";
const MANPAGE: &str = "manpage";

include!(concat!(env!("OUT_DIR"), "/uutils_map.rs"));

fn usage<T>(utils: &UtilityMap<T>, name: &str) {
    println!("{name} {VERSION} (multi-call binary)\n");
    println!("Usage: {name} [function [arguments...]]");
    println!("       {name} --list");
    println!();
    println!("Functions:");
    println!("      {COMPLETION}",);
    println!("           {}", get_completion_args(utils).render_usage());
    println!("      '{MANPAGE}'",);
    println!("           {}", get_manpage_args(utils).render_usage());
    println!("      '<uutils>' [arguments...]");
    println!();
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
        process::exit(uumain(vec![binary.into()].into_iter().chain(args)));
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

        let Some(util) = util_os.to_str() else {
            not_found(&util_os)
        };

        match util {
            COMPLETION => {
                gen_completions(args, &utils);
                process::exit(0);
            }
            MANPAGE => {
                gen_manpage(args, &utils);
                process::exit(0);
            }
            "--list" => {
                let mut utils: Vec<_> = utils.keys().collect();
                utils.sort();
                for util in utils {
                    println!("{util}");
                }
                process::exit(0);
            }
            // Not a special command: fallthrough to calling a util
            _ => {}
        }

        match utils.get(util) {
            Some(&(uumain, _)) => {
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

fn get_completion_args<T>(util_map: &UtilityMap<T>) -> clap::Command {
    let all_utilities: Vec<_> = std::iter::once("coreutils")
        .chain(util_map.keys().copied())
        .collect();
    Command::new(COMPLETION)
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
}

/// Prints completions for the utility in the first parameter for the shell in the second parameter to stdout
/// # Panics
/// Panics if the utility map is empty
fn gen_completions<T: uucore::Args>(
    args: impl Iterator<Item = OsString>,
    util_map: &UtilityMap<T>,
) {
    let matches = get_completion_args(util_map)
        .get_matches_from(std::iter::once(OsString::from(COMPLETION)).chain(args));

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
}

fn get_manpage_args<T>(util_map: &UtilityMap<T>) -> clap::Command {
    let all_utilities: Vec<_> = std::iter::once("coreutils")
        .chain(util_map.keys().copied())
        .collect();
    Command::new(MANPAGE).about("Prints manpage to stdout").arg(
        Arg::new("utility")
            .value_parser(clap::builder::PossibleValuesParser::new(all_utilities))
            .required(true),
    )
}

/// Generate the manpage for the utility in the first parameter
/// # Panics
/// Panics if the utility map is empty
fn gen_manpage<T: uucore::Args>(args: impl Iterator<Item = OsString>, util_map: &UtilityMap<T>) {
    let tldr_zip = File::open("docs/tldr.zip")
        .ok()
        .and_then(|f| ZipArchive::new(f).ok());

    if tldr_zip.is_none() {
        // Could not open tldr.zip
    }
    let matches = get_manpage_args(util_map)
        .get_matches_from(std::iter::once(OsString::from(MANPAGE)).chain(args));

    let utility = matches.get_one::<String>("utility").unwrap();

    let command = if utility == "coreutils" {
        gen_coreutils_app(util_map)
    } else {
        let mut cmd = util_map.get(utility).unwrap().1();
        if let Ok(examples) = get_zip_examples(utility) {
            cmd = cmd.after_help(examples);
        }
        cmd
    };

    let man = clap_mangen::Man::new(command);
    man.render(&mut io::stdout())
        .expect("Man page generation failed");
    io::stdout().flush().unwrap();
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

/// # Errors
/// Returns an error if the tldr.zip file cannot be opened or read
fn get_zip_examples(name: &str) -> io::Result<String> {
    fn get_zip_content(archive: &mut ZipArchive<impl Read + Seek>, name: &str) -> Option<String> {
        let mut s = String::new();
        archive.by_name(name).ok()?.read_to_string(&mut s).unwrap();
        Some(s)
    }

    let mut w = io::BufWriter::new(Vec::new());
    let file = File::open("docs/tldr.zip")?;
    let mut tldr_zip = match ZipArchive::new(file) {
        Ok(zip) => zip,
        Err(e) => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Error reading tldr.zip: {}", e),
            ));
        }
    };

    let content = if let Some(f) =
        get_zip_content(&mut tldr_zip, &format!("pages/common/{}.md", name))
    {
        f
    } else if let Some(f) = get_zip_content(&mut tldr_zip, &format!("pages/linux/{}.md", name)) {
        f
    } else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Could not find tldr examples",
        ));
    };

    writeln!(w, "Examples")?;
    writeln!(w)?;
    for line in content.lines().skip_while(|l| !l.starts_with('-')) {
        if let Some(l) = line.strip_prefix("- ") {
            writeln!(w, "{l}")?;
        } else if line.starts_with('`') {
            writeln!(w, "{}", line.trim_matches('`'))?;
        } else if line.is_empty() {
            writeln!(w)?;
        } else {
            // println!("Not sure what to do with this line:");
            // println!("{line}");
        }
    }
    writeln!(w)?;
    writeln!(
        w,
        "> The examples are provided by the [tldr-pages project](https://tldr.sh) under the [CC BY 4.0 License](https://github.com/tldr-pages/tldr/blob/main/LICENSE.md)."
    )?;
    writeln!(w, "\n\n\n")?;
    writeln!(
        w,
        "> Please note that, as uutils is a work in progress, some examples might fail."
    )?;
    Ok(String::from_utf8(w.into_inner().unwrap()).unwrap())
}
