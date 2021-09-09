// This file is part of the uutils coreutils package.
//
// (c) Jordi Boggiano <j.boggiano@seld.be>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

/* last synced with: env (GNU coreutils) 8.13 */

// spell-checker:ignore (ToDO) chdir execvp progname subcommand subcommands unsets

#[macro_use]
extern crate clap;

#[macro_use]
extern crate uucore;

use clap::{App, AppSettings, Arg};
use ini::Ini;
use std::borrow::Cow;
use std::env;
use std::io::{self, Write};
use std::iter::Iterator;
use std::process::Command;
use uucore::display::Quotable;
use uucore::error::{UResult, USimpleError};

const USAGE: &str = "env [OPTION]... [-] [NAME=VALUE]... [COMMAND [ARG]...]";
const AFTER_HELP: &str = "\
A mere - implies -i. If no COMMAND, print the resulting environment.
";

struct Options<'a> {
    ignore_env: bool,
    null: bool,
    running_directory: Option<&'a str>,
    files: Vec<&'a str>,
    unsets: Vec<&'a str>,
    sets: Vec<(&'a str, &'a str)>,
    program: Vec<&'a str>,
}

// print name=value env pairs on screen
// if null is true, separate pairs with a \0, \n otherwise
fn print_env(null: bool) {
    let stdout_raw = io::stdout();
    let mut stdout = stdout_raw.lock();
    for (n, v) in env::vars() {
        write!(stdout, "{}={}{}", n, v, if null { '\0' } else { '\n' }).unwrap();
    }
}

fn parse_name_value_opt<'a>(opts: &mut Options<'a>, opt: &'a str) -> Result<bool, i32> {
    // is it a NAME=VALUE like opt ?
    if let Some(idx) = opt.find('=') {
        // yes, so push name, value pair
        let (name, value) = opt.split_at(idx);
        opts.sets.push((name, &value['='.len_utf8()..]));

        Ok(false)
    } else {
        // no, it's a program-like opt
        parse_program_opt(opts, opt).map(|_| true)
    }
}

fn parse_program_opt<'a>(opts: &mut Options<'a>, opt: &'a str) -> Result<(), i32> {
    if opts.null {
        eprintln!(
            "{}: cannot specify --null (-0) with command",
            uucore::util_name()
        );
        eprintln!(
            "Type \"{} --help\" for detailed information",
            uucore::execution_phrase()
        );
        Err(1)
    } else {
        opts.program.push(opt);
        Ok(())
    }
}

fn load_config_file(opts: &mut Options) -> UResult<()> {
    // NOTE: config files are parsed using an INI parser b/c it's available and compatible with ".env"-style files
    //   ... * but support for actual INI files, although working, is not intended, nor claimed
    for &file in &opts.files {
        let conf = if file == "-" {
            let stdin = io::stdin();
            let mut stdin_locked = stdin.lock();
            Ini::read_from(&mut stdin_locked)
        } else {
            Ini::load_from_file(file)
        };

        let conf = conf.map_err(|error| {
            show_error!("{}: {}", file.maybe_quote(), error);
            1
        })?;

        for (_, prop) in &conf {
            // ignore all INI section lines (treat them as comments)
            for (key, value) in prop.iter() {
                env::set_var(key, value);
            }
        }
    }

    Ok(())
}

#[cfg(not(windows))]
fn build_command<'a, 'b>(args: &'a mut Vec<&'b str>) -> (Cow<'b, str>, &'a [&'b str]) {
    let progname = Cow::from(args[0]);
    (progname, &args[1..])
}

#[cfg(windows)]
fn build_command<'a, 'b>(args: &'a mut Vec<&'b str>) -> (Cow<'b, str>, &'a [&'b str]) {
    args.insert(0, "/d/c");
    let progname = env::var("ComSpec")
        .map(Cow::from)
        .unwrap_or_else(|_| Cow::from("cmd"));

    (progname, &args[..])
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .usage(USAGE)
        .after_help(AFTER_HELP)
        .setting(AppSettings::AllowExternalSubcommands)
        .arg(Arg::with_name("ignore-environment")
            .short("i")
            .long("ignore-environment")
            .help("start with an empty environment"))
        .arg(Arg::with_name("chdir")
            .short("c")
            .long("chdir")
            .takes_value(true)
            .number_of_values(1)
            .value_name("DIR")
            .help("change working directory to DIR"))
        .arg(Arg::with_name("null")
            .short("0")
            .long("null")
            .help("end each output line with a 0 byte rather than a newline (only valid when \
                    printing the environment)"))
        .arg(Arg::with_name("file")
            .short("f")
            .long("file")
            .takes_value(true)
            .number_of_values(1)
            .value_name("PATH")
            .multiple(true)
            .help("read and set variables from a \".env\"-style configuration file (prior to any \
                    unset and/or set)"))
        .arg(Arg::with_name("unset")
            .short("u")
            .long("unset")
            .takes_value(true)
            .number_of_values(1)
            .value_name("NAME")
            .multiple(true)
            .help("remove variable from the environment"))
}

fn run_env(args: impl uucore::Args) -> UResult<()> {
    let app = uu_app();
    let matches = app.get_matches_from(args);

    let ignore_env = matches.is_present("ignore-environment");
    let null = matches.is_present("null");
    let running_directory = matches.value_of("chdir");
    let files = matches
        .values_of("file")
        .map(Iterator::collect)
        .unwrap_or_else(|| Vec::with_capacity(0));
    let unsets = matches
        .values_of("unset")
        .map(Iterator::collect)
        .unwrap_or_else(|| Vec::with_capacity(0));

    let mut opts = Options {
        ignore_env,
        null,
        running_directory,
        files,
        unsets,
        sets: vec![],
        program: vec![],
    };

    // change directory
    if let Some(d) = opts.running_directory {
        match env::set_current_dir(d) {
            Ok(()) => d,
            Err(error) => {
                return Err(USimpleError::new(
                    125,
                    format!("cannot change directory to \"{}\": {}", d, error),
                ));
            }
        };
    }

    // we handle the name, value pairs and the program to be executed by treating them as external
    // subcommands in clap
    if let (external, Some(matches)) = matches.subcommand() {
        let mut begin_prog_opts = false;

        if external == "-" {
            // "-" implies -i and stop parsing opts
            opts.ignore_env = true;
        } else {
            begin_prog_opts = parse_name_value_opt(&mut opts, external)?;
        }

        if let Some(mut iter) = matches.values_of("") {
            // read NAME=VALUE arguments (and up to a single program argument)
            while !begin_prog_opts {
                if let Some(opt) = iter.next() {
                    begin_prog_opts = parse_name_value_opt(&mut opts, opt)?;
                } else {
                    break;
                }
            }

            // read any leftover program arguments
            for opt in iter {
                parse_program_opt(&mut opts, opt)?;
            }
        }
    }

    // NOTE: we manually set and unset the env vars below rather than using Command::env() to more
    //       easily handle the case where no command is given

    // remove all env vars if told to ignore presets
    if opts.ignore_env {
        for (ref name, _) in env::vars() {
            env::remove_var(name);
        }
    }

    // load .env-style config file prior to those given on the command-line
    load_config_file(&mut opts)?;

    // unset specified env vars
    for name in &opts.unsets {
        env::remove_var(name);
    }

    // set specified env vars
    for &(name, val) in &opts.sets {
        // FIXME: set_var() panics if name is an empty string
        env::set_var(name, val);
    }

    if !opts.program.is_empty() {
        // we need to execute a command
        let (prog, args) = build_command(&mut opts.program);

        // FIXME: this should just use execvp() (no fork()) on Unix-like systems
        match Command::new(&*prog).args(args).status() {
            Ok(exit) if !exit.success() => return Err(exit.code().unwrap().into()),
            Err(ref err) if err.kind() == io::ErrorKind::NotFound => return Err(127.into()),
            Err(_) => return Err(126.into()),
            Ok(_) => (),
        }
    } else {
        // no program provided, so just dump all env vars to stdout
        print_env(opts.null);
    }

    Ok(())
}

#[uucore_procs::gen_uumain]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    run_env(args)
}
