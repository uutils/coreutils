// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) chdir execvp progname subcommand subcommands unsets setenv putenv spawnp SIGSEGV SIGBUS sigaction

use clap::{crate_name, crate_version, Arg, ArgAction, Command};
use ini::Ini;
#[cfg(unix)]
use nix::sys::signal::{raise, sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
use std::borrow::Cow;
use std::env;
use std::ffi::OsString;
use std::io::{self, Write};
use std::iter::Iterator;
use std::ops::Deref;
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{self};
use uucore::display::Quotable;
use uucore::error::{ExitCode, UError, UResult, USimpleError, UUsageError};
use uucore::line_ending::LineEnding;
use uucore::{format_usage, help_about, help_section, help_usage, show_warning};

const ABOUT: &str = help_about!("env.md");
const USAGE: &str = help_usage!("env.md");
const AFTER_HELP: &str = help_section!("after help", "env.md");

const ERROR_MSG_S_SHEBANG: &str = "use -[v]S to pass options in shebang lines";

struct Options<'a> {
    ignore_env: bool,
    line_ending: LineEnding,
    running_directory: Option<&'a str>,
    files: Vec<&'a str>,
    unsets: Vec<&'a str>,
    sets: Vec<(&'a str, &'a str)>,
    program: Vec<&'a str>,
}

// print name=value env pairs on screen
// if null is true, separate pairs with a \0, \n otherwise
fn print_env(line_ending: LineEnding) {
    let stdout_raw = io::stdout();
    let mut stdout = stdout_raw.lock();
    for (n, v) in env::vars() {
        write!(stdout, "{}={}{}", n, v, line_ending).unwrap();
    }
}

fn parse_name_value_opt<'a>(opts: &mut Options<'a>, opt: &'a str) -> UResult<bool> {
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

fn parse_program_opt<'a>(opts: &mut Options<'a>, opt: &'a str) -> UResult<()> {
    if opts.line_ending == LineEnding::Nul {
        Err(UUsageError::new(
            125,
            "cannot specify --null (-0) with command".to_string(),
        ))
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

        let conf =
            conf.map_err(|e| USimpleError::new(1, format!("{}: {}", file.maybe_quote(), e)))?;

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
#[allow(clippy::ptr_arg)]
fn build_command<'a, 'b>(args: &'a Vec<&'b str>) -> (Cow<'b, str>, &'a [&'b str]) {
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

pub fn uu_app() -> Command {
    Command::new(crate_name!())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .after_help(AFTER_HELP)
        .infer_long_args(true)
        .trailing_var_arg(true)
        .arg(
            Arg::new("ignore-environment")
                .short('i')
                .long("ignore-environment")
                .help("start with an empty environment")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("chdir")
                .short('C') // GNU env compatibility
                .long("chdir")
                .number_of_values(1)
                .value_name("DIR")
                .value_hint(clap::ValueHint::DirPath)
                .help("change working directory to DIR"),
        )
        .arg(
            Arg::new("null")
                .short('0')
                .long("null")
                .help(
                    "end each output line with a 0 byte rather than a newline (only \
                valid when printing the environment)",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("file")
                .short('f')
                .long("file")
                .value_name("PATH")
                .value_hint(clap::ValueHint::FilePath)
                .action(ArgAction::Append)
                .help(
                    "read and set variables from a \".env\"-style configuration file \
                (prior to any unset and/or set)",
                ),
        )
        .arg(
            Arg::new("unset")
                .short('u')
                .long("unset")
                .value_name("NAME")
                .action(ArgAction::Append)
                .help("remove variable from the environment"),
        )
        .arg(
            Arg::new("debug")
                .short('v')
                .long("debug")
                .action(ArgAction::SetTrue)
                .help("print verbose information for each processing step"),
        )
        .arg(
            Arg::new("split-string") // split string handling is implemented directly, not using CLAP. But this entry here is needed for the help information output.
                .short('S')
                .long("split-string")
                .value_name("S")
                .action(ArgAction::Set)
                .help("process and split S into separate arguments; used to pass multiple arguments on shebang lines")
        )
        .arg(Arg::new("vars").action(ArgAction::Append))
}

fn command_to_argvec(
    shell: &mut nsh::shell::Shell,
    command: &nsh::parser::Command,
) -> UResult<Vec<String>> {
    use nsh::expand;
    use nsh::parser;

    let mut get_assignment_strings =
        |assignments: &Vec<parser::Assignment>| -> UResult<Vec<String>> {
            let mut assignment_strings = Vec::new();
            for assignment in assignments {
                let name = nsh::eval::evaluate_initializer_string(shell, &assignment.name)
                    .expect("failed to evaluate the name");
                let value = nsh::eval::evaluate_initializer(shell, &assignment.initializer)
                    .expect("failed to evaluate the initializer");
                match value {
                    nsh::variable::Value::String(s) => {
                        std::env::set_var(&name, &s);
                        assignment_strings.push(format!("{}={}", &name, s));
                    }
                    nsh::variable::Value::Array(_) => {
                        return Err(USimpleError::new(
                            1,
                            "Array assignments in a command is not supported.",
                        ));
                    }
                    nsh::variable::Value::Function(_) => (),
                }
            }
            Ok(assignment_strings)
        };

    match command {
        parser::Command::Assignment { assignments } => {
            let assignment_strings = get_assignment_strings(assignments)?;
            Ok(assignment_strings)
        }
        parser::Command::SimpleCommand {
            argv,
            redirects: _,
            assignments,
        } => {
            let mut assignment_strings = get_assignment_strings(assignments)?;

            let mut argv_strings = expand::expand_words(shell, argv.as_slice()).unwrap();

            assignment_strings.append(&mut argv_strings);
            Ok(assignment_strings)
        }
        _ => Err(USimpleError::new(
            1,
            format!("unexpected command type: {:?}", command),
        )),
    }
}

pub fn parse_args_from_str(text: &str) -> UResult<Vec<String>> {
    use nsh::parser;

    let mut parser = parser::ShellParser::new();

    let result = parser.parse(text);
    if let Ok(ast) = result {
        let mut shell = nsh::shell::Shell::new(Path::new(""));
        // Import environment variables.
        for (key, value) in std::env::vars() {
            shell.set(&key, nsh::variable::Value::String(value.to_owned()), false);
        }

        for term in &ast.terms {
            for pipeline in &term.pipelines {
                for command in &pipeline.commands {
                    let arg_vec = command_to_argvec(&mut shell, &command)?;
                    return Ok(arg_vec);
                }
            }
        }

        return Err(USimpleError::new(
            1,
            format!("no elements in ast: {:?}", ast),
        ));
    } else {
        let e = result.unwrap_err();
        match e {
            nsh::parser::ParseError::Empty => {
                return Ok(Vec::default());
            },
            nsh::parser::ParseError::Fatal(s)
                if s.contains("expected command_span, backtick_span, expr_span, param_ex_span, param_span, or literal_in_double_quoted_span")
                    || s.contains("expected literal_in_single_quoted_span")
                 => {
                    return Err(USimpleError::new(
                            125,
                            "no terminating quote in -S string"
                        ));
                },
            nsh::parser::ParseError::Fatal(s) => {
                    return Err(USimpleError::new(
                            125,
                            format!("parsing failed: {}", s),
                        ));
                },
        };
    };
}

fn check_and_handle_string_args(
    bytes: &[u8],
    prefix_to_test: &str,
    all_args: &mut Vec<std::ffi::OsString>,
) -> UResult<bool> {
    if !bytes.starts_with(prefix_to_test.as_bytes()) {
        return Ok(false);
    }

    let remaining_bytes = bytes.get(prefix_to_test.len()..).unwrap();

    if remaining_bytes.ends_with("\\".as_bytes()) && !remaining_bytes.ends_with("\\\\".as_bytes()) {
        return Err(USimpleError::new(
            125,
            "invalid backslash at end of string in -S",
        ));
    }

    let string = String::from_utf8(remaining_bytes.to_owned()).unwrap();

    let arg_strings = parse_args_from_str(string.as_str())?;
    for part in arg_strings {
        match part {
            s if s.contains(r"\c") => {
                return Err(USimpleError::new(
                    125,
                    "'\\c' must not appear in double-quoted -S string",
                ))
            }
            s if s.contains(r"\q") => {
                return Err(USimpleError::new(125, "invalid sequence '\\q' in -S"))
            }
            _ => {}
        }

        all_args.push(OsString::from(part));
    }

    Ok(true)
}

#[allow(clippy::cognitive_complexity)]
fn run_env(original_args: impl uucore::Args) -> UResult<()> {
    let mut do_debug_printing = false;
    let original_args: Vec<_> = original_args.collect();
    let mut all_args: Vec<std::ffi::OsString> = Vec::new();
    for arg in &original_args {
        match arg.as_bytes() {
            b if check_and_handle_string_args(b, "-S", &mut all_args)? => {}
            b if check_and_handle_string_args(b, "-vS", &mut all_args)? => {
                do_debug_printing = true;
            }
            b if check_and_handle_string_args(b, "--split-string", &mut all_args)? => {}
            _ => {
                all_args.push(OsString::from(arg));
            }
        }
    }

    let args = all_args;

    let app = uu_app();
    let matches =
        app.try_get_matches_from(args)
            .map_err(|e| -> Box<dyn UError> {
                match e.kind() {
                    clap::error::ErrorKind::DisplayHelp
                    | clap::error::ErrorKind::DisplayVersion => e.into(),
                    _ => {
                        let s = format!("{}", e);
                        if s != "" {
                            let s = s.trim_end();
                            uucore::show_error!("{}", s);
                        }
                        uucore::show_error!("{}", ERROR_MSG_S_SHEBANG);
                        uucore::error::ExitCode::new(125)
                    }
                }
            })?;

    let ignore_env = matches.get_flag("ignore-environment");
    let do_debug_printing = do_debug_printing || matches.get_flag("debug");
    if do_debug_printing {
        eprintln!("input args:");
        for (i, arg) in original_args.iter().enumerate() {
            eprintln!("arg[{}]: {}", i, arg.to_string_lossy());
        }
    }

    let line_ending = LineEnding::from_zero_flag(matches.get_flag("null"));
    let running_directory = matches.get_one::<String>("chdir").map(|s| s.as_str());
    let files = match matches.get_many::<String>("file") {
        Some(v) => v.map(|s| s.as_str()).collect(),
        None => Vec::with_capacity(0),
    };
    let unsets = match matches.get_many::<String>("unset") {
        Some(v) => v.map(|s| s.as_str()).collect(),
        None => Vec::with_capacity(0),
    };

    let mut opts = Options {
        ignore_env,
        line_ending,
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
                    format!("cannot change directory to \"{d}\": {error}"),
                ));
            }
        };
    }

    let mut begin_prog_opts = false;
    if let Some(mut iter) = matches.get_many::<String>("vars") {
        // read NAME=VALUE arguments (and up to a single program argument)
        while !begin_prog_opts {
            if let Some(opt) = iter.next() {
                if opt == "-" {
                    opts.ignore_env = true;
                } else {
                    begin_prog_opts = parse_name_value_opt(&mut opts, opt)?;
                }
            } else {
                break;
            }
        }

        // read any leftover program arguments
        for opt in iter {
            parse_program_opt(&mut opts, opt)?;
        }
    }

    // GNU env tests this behavior
    if opts.program.is_empty() && running_directory.is_some() {
        return Err(UUsageError::new(
            125,
            "must specify command with --chdir (-C)".to_string(),
        ));
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
        if name.is_empty() || name.contains(0 as char) || name.contains('=') {
            return Err(USimpleError::new(
                125,
                format!("cannot unset {}: Invalid argument", name.quote()),
            ));
        }

        env::remove_var(name);
    }

    // set specified env vars
    for &(name, val) in &opts.sets {
        /*
         * set_var panics if name is an empty string
         * set_var internally calls setenv (on unix at least), while GNU env calls putenv instead.
         *
         * putenv returns successfully if provided with something like "=a" and modifies the environ
         * variable to contain "=a" inside it, effectively modifying the process' current environment
         * to contain a malformed string in it. Using GNU's implementation, the command `env =a`
         * prints out the malformed string and even invokes the child process with that environment.
         * This can be seen by using `env -i =a env` or `env -i =a cat /proc/self/environ`
         *
         * POSIX.1-2017 doesn't seem to mention what to do if the string is malformed (at least
         * not in "Chapter 8, Environment Variables" or in the definition for environ and various
         * exec*'s or in the description of env in the "Shell & Utilities" volume).
         *
         * It also doesn't specify any checks for putenv before modifying the environ variable, which
         * is likely why glibc doesn't do so. However, the first set_var argument cannot point to
         * an empty string or a string containing '='.
         *
         * There is no benefit in replicating GNU's env behavior, since it will only modify the
         * environment in weird ways
         */

        if name.is_empty() {
            show_warning!("no name specified for value {}", val.quote());
            continue;
        }
        env::set_var(name, val);
    }

    if opts.program.is_empty() {
        // no program provided, so just dump all env vars to stdout
        print_env(opts.line_ending);
    } else {
        // we need to execute a command
        #[cfg(windows)]
        let (prog, args) = build_command(&mut opts.program);
        #[cfg(not(windows))]
        let (prog, args) = build_command(&opts.program);

        if do_debug_printing {
            eprintln!("executable: {}", prog);
            for (i, arg) in args.iter().enumerate() {
                eprintln!("arg[{}]: {}", i, arg);
            }
        }

        /*
         * On Unix-like systems Command::status either ends up calling either fork or posix_spawnp
         * (which ends up calling clone). Keep using the current process would be ideal, but the
         * standard library contains many checks and fail-safes to ensure the process ends up being
         * created. This is much simpler than dealing with the hassles of calling execvp directly.
         */
        match process::Command::new(&*prog).args(args).status() {
            Ok(exit) if !exit.success() => {
                #[cfg(unix)]
                if let Some(exit_code) = exit.code() {
                    return Err(exit_code.into());
                } else {
                    // `exit.code()` returns `None` on Unix when the process is terminated by a signal.
                    // See std::os::unix::process::ExitStatusExt for more information. This prints out
                    // the interrupted process and the signal it received.
                    let signal_code = exit.signal().unwrap();
                    let signal = Signal::try_from(signal_code).unwrap();

                    // We have to disable any handler that's installed by default.
                    // This ensures that we exit on this signal.
                    // For example, `SIGSEGV` and `SIGBUS` have default handlers installed in Rust.
                    // We ignore the errors because there is not much we can do if that fails anyway.
                    // SAFETY: The function is unsafe because installing functions is unsafe, but we are
                    // just defaulting to default behavior and not installing a function. Hence, the call
                    // is safe.
                    let _ = unsafe {
                        sigaction(
                            signal,
                            &SigAction::new(SigHandler::SigDfl, SaFlags::empty(), SigSet::all()),
                        )
                    };

                    let _ = raise(signal);
                }
                #[cfg(not(unix))]
                return Err(exit.code().unwrap().into());
            }
            Err(ref err) if err.kind() == io::ErrorKind::NotFound => {
                uucore::show_error!("'{}': No such file or directory", prog.deref());
                uucore::show_error!("{}", ERROR_MSG_S_SHEBANG);
                return Err(ExitCode::new(127));
            }
            Err(_) => return Err(126.into()),
            Ok(_) => (),
        }
    }

    Ok(())
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    run_env(args)
}
