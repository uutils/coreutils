// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) chdir execvp progname subcommand subcommands unsets setenv putenv spawnp SIGSEGV SIGBUS sigaction

pub mod native_int_str;
pub mod parse_error;
pub mod split_iterator;
pub mod string_expander;
pub mod string_parser;
pub mod variable_parser;

use clap::builder::ValueParser;
use clap::{crate_name, crate_version, Arg, ArgAction, Command};
use ini::Ini;
use native_int_str::{
    from_native_int_representation_owned, Convert, NCvt, NativeIntStr, NativeIntString, NativeStr,
};
#[cfg(unix)]
use nix::sys::signal::{
    raise, sigaction, signal, SaFlags, SigAction, SigHandler, SigHandler::SigIgn, SigSet, Signal,
};
use std::borrow::Cow;
use std::env;
use std::ffi::{OsStr, OsString};
use std::io::{self, Write};
use std::ops::Deref;

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{self};
use uucore::display::Quotable;
use uucore::error::{ExitCode, UError, UResult, USimpleError, UUsageError};
use uucore::line_ending::LineEnding;
#[cfg(unix)]
use uucore::signals::signal_by_name_or_value;
use uucore::{format_usage, help_about, help_section, help_usage, show_warning};

const ABOUT: &str = help_about!("env.md");
const USAGE: &str = help_usage!("env.md");
const AFTER_HELP: &str = help_section!("after help", "env.md");

const ERROR_MSG_S_SHEBANG: &str = "use -[v]S to pass options in shebang lines";

struct Options<'a> {
    ignore_env: bool,
    line_ending: LineEnding,
    running_directory: Option<&'a OsStr>,
    files: Vec<&'a OsStr>,
    unsets: Vec<&'a OsStr>,
    sets: Vec<(Cow<'a, OsStr>, Cow<'a, OsStr>)>,
    program: Vec<&'a OsStr>,
    argv0: Option<&'a OsStr>,
    #[cfg(unix)]
    ignore_signal: Vec<usize>,
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

fn parse_name_value_opt<'a>(opts: &mut Options<'a>, opt: &'a OsStr) -> UResult<bool> {
    // is it a NAME=VALUE like opt ?
    let wrap = NativeStr::<'a>::new(opt);
    let split_o = wrap.split_once(&'=');
    if let Some((name, value)) = split_o {
        // yes, so push name, value pair
        opts.sets.push((name, value));
        Ok(false)
    } else {
        // no, it's a program-like opt
        parse_program_opt(opts, opt).map(|_| true)
    }
}

fn parse_program_opt<'a>(opts: &mut Options<'a>, opt: &'a OsStr) -> UResult<()> {
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

#[cfg(unix)]
fn parse_signal_value(signal_name: &str) -> UResult<usize> {
    let signal_name_upcase = signal_name.to_uppercase();
    let optional_signal_value = signal_by_name_or_value(&signal_name_upcase);
    let error = USimpleError::new(125, format!("{}: invalid signal", signal_name.quote()));
    match optional_signal_value {
        Some(sig_val) => {
            if sig_val == 0 {
                Err(error)
            } else {
                Ok(sig_val)
            }
        }
        None => Err(error),
    }
}

#[cfg(unix)]
fn parse_signal_opt<'a>(opts: &mut Options<'a>, opt: &'a OsStr) -> UResult<()> {
    if opt.is_empty() {
        return Ok(());
    }
    let signals: Vec<&'a OsStr> = opt
        .as_bytes()
        .split(|&b| b == b',')
        .map(OsStr::from_bytes)
        .collect();

    let mut sig_vec = Vec::with_capacity(signals.len());
    signals.into_iter().for_each(|sig| {
        if !(sig.is_empty()) {
            sig_vec.push(sig);
        }
    });
    for sig in sig_vec {
        let sig_str = match sig.to_str() {
            Some(s) => s,
            None => {
                return Err(USimpleError::new(
                    1,
                    format!("{}: invalid signal", sig.quote()),
                ))
            }
        };
        let sig_val = parse_signal_value(sig_str)?;
        if !opts.ignore_signal.contains(&sig_val) {
            opts.ignore_signal.push(sig_val);
        }
    }

    Ok(())
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
            for (key, value) in prop {
                env::set_var(key, value);
            }
        }
    }

    Ok(())
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
                .value_parser(ValueParser::os_string())
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
                .value_parser(ValueParser::os_string())
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
                .value_parser(ValueParser::os_string())
                .help("remove variable from the environment"),
        )
        .arg(
            Arg::new("debug")
                .short('v')
                .long("debug")
                .action(ArgAction::Count)
                .help("print verbose information for each processing step"),
        )
        .arg(
            Arg::new("split-string") // split string handling is implemented directly, not using CLAP. But this entry here is needed for the help information output.
                .short('S')
                .long("split-string")
                .value_name("S")
                .action(ArgAction::Set)
                .value_parser(ValueParser::os_string())
                .help("process and split S into separate arguments; used to pass multiple arguments on shebang lines")
        ).arg(
            Arg::new("argv0")
                .overrides_with("argv0")
                .short('a')
                .long("argv0")
                .value_name("a")
                .action(ArgAction::Set)
                .value_parser(ValueParser::os_string())
                .help("Override the zeroth argument passed to the command being executed. \
                       Without this option a default value of `command` is used.")
        )
        .arg(
            Arg::new("vars")
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
        )
        .arg(
            Arg::new("ignore-signal")
                .long("ignore-signal")
                .value_name("SIG")
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .help("set handling of SIG signal(s) to do nothing")
        )
}

pub fn parse_args_from_str(text: &NativeIntStr) -> UResult<Vec<NativeIntString>> {
    split_iterator::split(text).map_err(|e| match e {
        parse_error::ParseError::BackslashCNotAllowedInDoubleQuotes { pos: _ } => {
            USimpleError::new(125, "'\\c' must not appear in double-quoted -S string")
        }
        parse_error::ParseError::InvalidBackslashAtEndOfStringInMinusS { pos: _, quoting: _ } => {
            USimpleError::new(125, "invalid backslash at end of string in -S")
        }
        parse_error::ParseError::InvalidSequenceBackslashXInMinusS { pos: _, c } => {
            USimpleError::new(125, format!("invalid sequence '\\{}' in -S", c))
        }
        parse_error::ParseError::MissingClosingQuote { pos: _, c: _ } => {
            USimpleError::new(125, "no terminating quote in -S string")
        }
        parse_error::ParseError::ParsingOfVariableNameFailed { pos, msg } => {
            USimpleError::new(125, format!("variable name issue (at {}): {}", pos, msg,))
        }
        _ => USimpleError::new(125, format!("Error: {:?}", e)),
    })
}

fn debug_print_args(args: &[OsString]) {
    eprintln!("input args:");
    for (i, arg) in args.iter().enumerate() {
        eprintln!("arg[{}]: {}", i, arg.quote());
    }
}

fn check_and_handle_string_args(
    arg: &OsString,
    prefix_to_test: &str,
    all_args: &mut Vec<std::ffi::OsString>,
    do_debug_print_args: Option<&Vec<OsString>>,
) -> UResult<bool> {
    let native_arg = NCvt::convert(arg);
    if let Some(remaining_arg) = native_arg.strip_prefix(&*NCvt::convert(prefix_to_test)) {
        if let Some(input_args) = do_debug_print_args {
            debug_print_args(input_args); // do it here, such that its also printed when we get an error/panic during parsing
        }

        let arg_strings = parse_args_from_str(remaining_arg)?;
        all_args.extend(
            arg_strings
                .into_iter()
                .map(from_native_int_representation_owned),
        );

        Ok(true)
    } else {
        Ok(false)
    }
}

#[derive(Default)]
struct EnvAppData {
    do_debug_printing: bool,
    do_input_debug_printing: Option<bool>,
    had_string_argument: bool,
}

impl EnvAppData {
    fn make_error_no_such_file_or_dir(&self, prog: &OsStr) -> Box<dyn UError> {
        uucore::show_error!("{}: No such file or directory", prog.quote());
        if !self.had_string_argument {
            uucore::show_error!("{}", ERROR_MSG_S_SHEBANG);
        }
        ExitCode::new(127)
    }

    fn process_all_string_arguments(
        &mut self,
        original_args: &Vec<OsString>,
    ) -> UResult<Vec<std::ffi::OsString>> {
        let mut all_args: Vec<std::ffi::OsString> = Vec::new();
        for arg in original_args {
            match arg {
                b if check_and_handle_string_args(b, "--split-string", &mut all_args, None)? => {
                    self.had_string_argument = true;
                }
                b if check_and_handle_string_args(b, "-S", &mut all_args, None)? => {
                    self.had_string_argument = true;
                }
                b if check_and_handle_string_args(b, "-vS", &mut all_args, None)? => {
                    self.do_debug_printing = true;
                    self.had_string_argument = true;
                }
                b if check_and_handle_string_args(
                    b,
                    "-vvS",
                    &mut all_args,
                    Some(original_args),
                )? =>
                {
                    self.do_debug_printing = true;
                    self.do_input_debug_printing = Some(false); // already done
                    self.had_string_argument = true;
                }
                _ => {
                    all_args.push(arg.clone());
                }
            }
        }

        Ok(all_args)
    }

    fn parse_arguments(
        &mut self,
        original_args: impl uucore::Args,
    ) -> Result<(Vec<OsString>, clap::ArgMatches), Box<dyn UError>> {
        let original_args: Vec<OsString> = original_args.collect();
        let args = self.process_all_string_arguments(&original_args)?;
        let app = uu_app();
        let matches = app
            .try_get_matches_from(args)
            .map_err(|e| -> Box<dyn UError> {
                match e.kind() {
                    clap::error::ErrorKind::DisplayHelp
                    | clap::error::ErrorKind::DisplayVersion => e.into(),
                    _ => {
                        // extent any real issue with parameter parsing by the ERROR_MSG_S_SHEBANG
                        let s = format!("{}", e);
                        if !s.is_empty() {
                            let s = s.trim_end();
                            uucore::show_error!("{}", s);
                        }
                        uucore::show_error!("{}", ERROR_MSG_S_SHEBANG);
                        uucore::error::ExitCode::new(125)
                    }
                }
            })?;
        Ok((original_args, matches))
    }

    fn run_env(&mut self, original_args: impl uucore::Args) -> UResult<()> {
        let (original_args, matches) = self.parse_arguments(original_args)?;

        self.do_debug_printing = self.do_debug_printing || (0 != matches.get_count("debug"));
        self.do_input_debug_printing = self
            .do_input_debug_printing
            .or(Some(matches.get_count("debug") >= 2));
        if let Some(value) = self.do_input_debug_printing {
            if value {
                debug_print_args(&original_args);
                self.do_input_debug_printing = Some(false);
            }
        }

        let mut opts = make_options(&matches)?;

        apply_change_directory(&opts)?;

        // NOTE: we manually set and unset the env vars below rather than using Command::env() to more
        //       easily handle the case where no command is given

        apply_removal_of_all_env_vars(&opts);

        // load .env-style config file prior to those given on the command-line
        load_config_file(&mut opts)?;

        apply_unset_env_vars(&opts)?;

        apply_specified_env_vars(&opts);

        #[cfg(unix)]
        apply_ignore_signal(&opts)?;

        if opts.program.is_empty() {
            // no program provided, so just dump all env vars to stdout
            print_env(opts.line_ending);
        } else {
            return self.run_program(&opts, self.do_debug_printing);
        }

        Ok(())
    }

    fn run_program(
        &mut self,
        opts: &Options<'_>,
        do_debug_printing: bool,
    ) -> Result<(), Box<dyn UError>> {
        let prog = Cow::from(opts.program[0]);
        #[cfg(unix)]
        let mut arg0 = prog.clone();
        #[cfg(not(unix))]
        let arg0 = prog.clone();
        let args = &opts.program[1..];

        /*
         * On Unix-like systems Command::status either ends up calling either fork or posix_spawnp
         * (which ends up calling clone). Keep using the current process would be ideal, but the
         * standard library contains many checks and fail-safes to ensure the process ends up being
         * created. This is much simpler than dealing with the hassles of calling execvp directly.
         */
        let mut cmd = process::Command::new(&*prog);
        cmd.args(args);

        if let Some(_argv0) = opts.argv0 {
            #[cfg(unix)]
            {
                cmd.arg0(_argv0);
                arg0 = Cow::Borrowed(_argv0);
                if do_debug_printing {
                    eprintln!("argv0:     {}", arg0.quote());
                }
            }

            #[cfg(not(unix))]
            return Err(USimpleError::new(
                2,
                "--argv0 is currently not supported on this platform",
            ));
        }

        if do_debug_printing {
            eprintln!("executing: {}", prog.maybe_quote());
            let arg_prefix = "   arg";
            eprintln!("{}[{}]= {}", arg_prefix, 0, arg0.quote());
            for (i, arg) in args.iter().enumerate() {
                eprintln!("{}[{}]= {}", arg_prefix, i + 1, arg.quote());
            }
        }

        match cmd.status() {
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
                return Err(exit.code().unwrap().into());
            }
            Err(ref err)
                if (err.kind() == io::ErrorKind::NotFound)
                    || (err.kind() == io::ErrorKind::InvalidInput) =>
            {
                return Err(self.make_error_no_such_file_or_dir(prog.deref()));
            }
            Err(e) => {
                uucore::show_error!("unknown error: {:?}", e);
                return Err(126.into());
            }
            Ok(_) => (),
        }
        Ok(())
    }
}

fn apply_removal_of_all_env_vars(opts: &Options<'_>) {
    // remove all env vars if told to ignore presets
    if opts.ignore_env {
        for (ref name, _) in env::vars_os() {
            env::remove_var(name);
        }
    }
}

fn make_options(matches: &clap::ArgMatches) -> UResult<Options<'_>> {
    let ignore_env = matches.get_flag("ignore-environment");
    let line_ending = LineEnding::from_zero_flag(matches.get_flag("null"));
    let running_directory = matches.get_one::<OsString>("chdir").map(|s| s.as_os_str());
    let files = match matches.get_many::<OsString>("file") {
        Some(v) => v.map(|s| s.as_os_str()).collect(),
        None => Vec::with_capacity(0),
    };
    let unsets = match matches.get_many::<OsString>("unset") {
        Some(v) => v.map(|s| s.as_os_str()).collect(),
        None => Vec::with_capacity(0),
    };
    let argv0 = matches.get_one::<OsString>("argv0").map(|s| s.as_os_str());

    let mut opts = Options {
        ignore_env,
        line_ending,
        running_directory,
        files,
        unsets,
        sets: vec![],
        program: vec![],
        argv0,
        #[cfg(unix)]
        ignore_signal: vec![],
    };

    #[cfg(unix)]
    if let Some(iter) = matches.get_many::<OsString>("ignore-signal") {
        for opt in iter {
            parse_signal_opt(&mut opts, opt)?;
        }
    }

    let mut begin_prog_opts = false;
    if let Some(mut iter) = matches.get_many::<OsString>("vars") {
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

    Ok(opts)
}

fn apply_unset_env_vars(opts: &Options<'_>) -> Result<(), Box<dyn UError>> {
    for name in &opts.unsets {
        let native_name = NativeStr::new(name);
        if name.is_empty()
            || native_name.contains(&'\0').unwrap()
            || native_name.contains(&'=').unwrap()
        {
            return Err(USimpleError::new(
                125,
                format!("cannot unset {}: Invalid argument", name.quote()),
            ));
        }

        env::remove_var(name);
    }
    Ok(())
}

fn apply_change_directory(opts: &Options<'_>) -> Result<(), Box<dyn UError>> {
    // GNU env tests this behavior
    if opts.program.is_empty() && opts.running_directory.is_some() {
        return Err(UUsageError::new(
            125,
            "must specify command with --chdir (-C)".to_string(),
        ));
    }

    if let Some(d) = opts.running_directory {
        match env::set_current_dir(d) {
            Ok(()) => d,
            Err(error) => {
                return Err(USimpleError::new(
                    125,
                    format!("cannot change directory to {}: {error}", d.quote()),
                ));
            }
        };
    }
    Ok(())
}

fn apply_specified_env_vars(opts: &Options<'_>) {
    // set specified env vars
    for (name, val) in &opts.sets {
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
}

#[cfg(unix)]
fn apply_ignore_signal(opts: &Options<'_>) -> UResult<()> {
    for &sig_value in &opts.ignore_signal {
        let sig: Signal = (sig_value as i32)
            .try_into()
            .map_err(|e| std::io::Error::from_raw_os_error(e as i32))?;

        ignore_signal(sig)?;
    }
    Ok(())
}

#[cfg(unix)]
fn ignore_signal(sig: Signal) -> UResult<()> {
    // SAFETY: This is safe because we write the handler for each signal only once, and therefore "the current handler is the default", as the documentation requires it.
    let result = unsafe { signal(sig, SigIgn) };
    if let Err(err) = result {
        return Err(USimpleError::new(
            125,
            format!(
                "failed to set signal action for signal {}: {}",
                sig as i32,
                err.desc()
            ),
        ));
    }
    Ok(())
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    EnvAppData::default().run_env(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_string_environment_vars_test() {
        std::env::set_var("FOO", "BAR");
        assert_eq!(
            NCvt::convert(vec!["FOO=bar", "sh", "-c", "echo xBARx =$FOO="]),
            parse_args_from_str(&NCvt::convert(r#"FOO=bar sh -c "echo x${FOO}x =\$FOO=""#))
                .unwrap(),
        );
    }

    #[test]
    fn test_split_string_misc() {
        assert_eq!(
            NCvt::convert(vec!["A=B", "FOO=AR", "sh", "-c", "echo $A$FOO"]),
            parse_args_from_str(&NCvt::convert(r#"A=B FOO=AR  sh -c "echo \$A\$FOO""#)).unwrap(),
        );
        assert_eq!(
            NCvt::convert(vec!["A=B", "FOO=AR", "sh", "-c", "echo $A$FOO"]),
            parse_args_from_str(&NCvt::convert(r#"A=B FOO=AR  sh -c 'echo $A$FOO'"#)).unwrap()
        );
        assert_eq!(
            NCvt::convert(vec!["A=B", "FOO=AR", "sh", "-c", "echo $A$FOO"]),
            parse_args_from_str(&NCvt::convert(r#"A=B FOO=AR  sh -c 'echo $A$FOO'"#)).unwrap()
        );

        assert_eq!(
            NCvt::convert(vec!["-i", "A=B ' C"]),
            parse_args_from_str(&NCvt::convert(r#"-i A='B \' C'"#)).unwrap()
        );
    }
}
