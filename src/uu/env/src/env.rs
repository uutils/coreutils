// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) chdir progname subcommand subcommands unsets setenv putenv spawnp SIGSEGV SIGBUS sigaction Sigmask sigprocmask

pub mod native_int_str;
pub mod split_iterator;
pub mod string_expander;
pub mod string_parser;
pub mod variable_parser;

use clap::builder::ValueParser;
use clap::{Arg, ArgAction, Command, crate_name};
use ini::Ini;
use native_int_str::{
    Convert, NCvt, NativeIntStr, NativeIntString, NativeStr, from_native_int_representation_owned,
};
#[cfg(unix)]
use nix::libc;
#[cfg(unix)]
use nix::sys::signal::{
    SigHandler::{SigDfl, SigIgn},
    SigSet, SigmaskHow, Signal, signal, sigprocmask,
};
#[cfg(unix)]
use nix::unistd::execvp;
use std::borrow::Cow;
#[cfg(unix)]
use std::collections::{BTreeMap, BTreeSet};
use std::env;
#[cfg(unix)]
use std::ffi::CString;
use std::ffi::{OsStr, OsString};
use std::io;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

use uucore::display::{Quotable, print_all_env_vars};
use uucore::error::{ExitCode, UError, UResult, USimpleError, UUsageError};
use uucore::line_ending::LineEnding;
#[cfg(unix)]
use uucore::signals::{ALL_SIGNALS, signal_by_name_or_value, signal_name_by_value};
use uucore::translate;
use uucore::{format_usage, show_warning};

use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum EnvError {
    #[error("{}", translate!("env-error-missing-closing-quote", "position" => .0, "quote" => .1))]
    EnvMissingClosingQuote(usize, char),
    #[error("{}", translate!("env-error-invalid-backslash-at-end", "position" => .0, "context" => .1.clone()))]
    EnvInvalidBackslashAtEndOfStringInMinusS(usize, String),
    #[error("{}", translate!("env-error-backslash-c-not-allowed", "position" => .0))]
    EnvBackslashCNotAllowedInDoubleQuotes(usize),
    #[error("{}", translate!("env-error-invalid-sequence", "position" => .0, "char" => .1))]
    EnvInvalidSequenceBackslashXInMinusS(usize, char),
    #[error("{}", translate!("env-error-missing-closing-brace", "position" => .0))]
    EnvParsingOfVariableMissingClosingBrace(usize),
    #[error("{}", translate!("env-error-missing-variable", "position" => .0))]
    EnvParsingOfMissingVariable(usize),
    #[error("{}", translate!("env-error-missing-closing-brace-after-value", "position" => .0))]
    EnvParsingOfVariableMissingClosingBraceAfterValue(usize),
    #[error("{}", translate!("env-error-unexpected-number", "position" => .0, "char" => .1.clone()))]
    EnvParsingOfVariableUnexpectedNumber(usize, String),
    #[error("{}", translate!("env-error-expected-brace-or-colon", "position" => .0, "char" => .1.clone()))]
    EnvParsingOfVariableExceptedBraceOrColon(usize, String),
    #[error("")]
    EnvReachedEnd,
    #[error("")]
    EnvContinueWithDelimiter,
    #[error("{}{:?}",.0,.1)]
    EnvInternalError(usize, string_parser::Error),
}

impl From<string_parser::Error> for EnvError {
    fn from(value: string_parser::Error) -> Self {
        Self::EnvInternalError(value.peek_position, value)
    }
}

mod options {
    pub const IGNORE_ENVIRONMENT: &str = "ignore-environment";
    pub const CHDIR: &str = "chdir";
    pub const NULL: &str = "null";
    pub const FILE: &str = "file";
    pub const UNSET: &str = "unset";
    pub const DEBUG: &str = "debug";
    pub const SPLIT_STRING: &str = "split-string";
    pub const ARGV0: &str = "argv0";
    pub const IGNORE_SIGNAL: &str = "ignore-signal";
    pub const DEFAULT_SIGNAL: &str = "default-signal";
    pub const BLOCK_SIGNAL: &str = "block-signal";
    pub const LIST_SIGNAL_HANDLING: &str = "list-signal-handling";
}

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
    ignore_signal: SignalRequest,
    #[cfg(unix)]
    default_signal: SignalRequest,
    #[cfg(unix)]
    block_signal: SignalRequest,
    #[cfg(unix)]
    list_signal_handling: bool,
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
            translate!("env-error-cannot-specify-null-with-command"),
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
    let error = USimpleError::new(
        125,
        translate!("env-error-invalid-signal", "signal" => signal_name.quote()),
    );
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
fn parse_signal_opt(target: &mut SignalRequest, opt: &OsStr) -> UResult<()> {
    if opt.is_empty() {
        return Ok(());
    }
    if opt == "__ALL__" {
        target.apply_all = true;
        return Ok(());
    }

    for sig in opt
        .as_bytes()
        .split(|&b| b == b',')
        .filter(|chunk| !chunk.is_empty())
        .map(OsStr::from_bytes)
    {
        let Some(sig_str) = sig.to_str() else {
            return Err(USimpleError::new(
                1,
                translate!("env-error-invalid-signal", "signal" => sig.quote()),
            ));
        };
        let sig_val = parse_signal_value(sig_str)?;
        target.signals.insert(sig_val);
    }

    Ok(())
}

#[cfg(unix)]
#[derive(Default)]
struct SignalRequest {
    apply_all: bool,
    signals: BTreeSet<usize>,
}

#[cfg(unix)]
impl SignalRequest {
    fn is_empty(&self) -> bool {
        !self.apply_all && self.signals.is_empty()
    }

    fn for_each_signal<F>(&self, mut f: F) -> UResult<()>
    where
        F: FnMut(usize, bool) -> UResult<()>,
    {
        if self.is_empty() {
            return Ok(());
        }
        for &sig in &self.signals {
            f(sig, true)?;
        }
        if self.apply_all {
            for sig_value in 1..ALL_SIGNALS.len() {
                if self.signals.contains(&sig_value) {
                    continue;
                }
                // SIGKILL (9) and SIGSTOP (17 on mac, 19 on linux) cannot be caught or ignored
                if sig_value == libc::SIGKILL as usize || sig_value == libc::SIGSTOP as usize {
                    continue;
                }
                f(sig_value, false)?;
            }
        }
        Ok(())
    }
}

#[cfg(unix)]
#[derive(Copy, Clone)]
enum SignalActionKind {
    Default,
    Ignore,
    Block,
}

#[cfg(unix)]
#[derive(Copy, Clone)]
struct SignalActionRecord {
    kind: SignalActionKind,
    explicit: bool,
}

#[cfg(unix)]
#[derive(Default)]
struct SignalActionLog {
    records: BTreeMap<usize, SignalActionRecord>,
}

#[cfg(unix)]
impl SignalActionLog {
    fn record(&mut self, sig_value: usize, kind: SignalActionKind, explicit: bool) {
        self.records
            .entry(sig_value)
            .and_modify(|entry| {
                entry.kind = kind;
                if explicit {
                    entry.explicit = true;
                }
            })
            .or_insert(SignalActionRecord { kind, explicit });
    }
}

#[cfg(unix)]
fn build_signal_request(matches: &clap::ArgMatches, option: &str) -> UResult<SignalRequest> {
    let mut request = SignalRequest::default();
    let mut provided_values = 0usize;

    let mut explicit_empty = false;
    if let Some(iter) = matches.get_many::<OsString>(option) {
        for opt in iter {
            if opt.is_empty() {
                explicit_empty = true;
                continue;
            }
            provided_values += 1;
            parse_signal_opt(&mut request, opt)?;
        }
    }

    let present = matches.contains_id(option);
    if present && provided_values == 0 && !explicit_empty {
        request.apply_all = true;
    }

    Ok(request)
}

#[cfg(unix)]
fn signal_from_value(sig_value: usize) -> UResult<Signal> {
    Signal::try_from(sig_value as i32).map_err(|_| {
        USimpleError::new(
            125,
            translate!(
                "env-error-invalid-signal",
                "signal" => sig_value.to_string().quote()
            ),
        )
    })
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

        let conf = conf.map_err(|e| {
            USimpleError::new(
                1,
                translate!("env-error-config-file", "file" => file.maybe_quote(), "error" => e),
            )
        })?;

        for (_, prop) in &conf {
            // ignore all INI section lines (treat them as comments)
            for (key, value) in prop {
                unsafe {
                    env::set_var(key, value);
                }
            }
        }
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(crate_name!())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("env-about"))
        .override_usage(format_usage(&translate!("env-usage")))
        .after_help(translate!("env-after-help"))
        .infer_long_args(true)
        .trailing_var_arg(true)
        .arg(
            Arg::new(options::IGNORE_ENVIRONMENT)
                .short('i')
                .long(options::IGNORE_ENVIRONMENT)
                .help(translate!("env-help-ignore-environment"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::CHDIR)
                .short('C') // GNU env compatibility
                .long(options::CHDIR)
                .number_of_values(1)
                .value_name("DIR")
                .value_parser(ValueParser::os_string())
                .value_hint(clap::ValueHint::DirPath)
                .help(translate!("env-help-chdir")),
        )
        .arg(
            Arg::new(options::NULL)
                .short('0')
                .long(options::NULL)
                .help(translate!("env-help-null"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::FILE)
                .short('f')
                .long(options::FILE)
                .value_name("PATH")
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(ValueParser::os_string())
                .action(ArgAction::Append)
                .help(translate!("env-help-file")),
        )
        .arg(
            Arg::new(options::UNSET)
                .short('u')
                .long(options::UNSET)
                .value_name("NAME")
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string())
                .help(translate!("env-help-unset")),
        )
        .arg(
            Arg::new(options::DEBUG)
                .short('v')
                .long(options::DEBUG)
                .action(ArgAction::Count)
                .help(translate!("env-help-debug")),
        )
        .arg(
            Arg::new(options::SPLIT_STRING) // split string handling is implemented directly, not using CLAP. But this entry here is needed for the help information output.
                .short('S')
                .long(options::SPLIT_STRING)
                .value_name("S")
                .action(ArgAction::Set)
                .value_parser(ValueParser::os_string())
                .help(translate!("env-help-split-string")),
        )
        .arg(
            Arg::new(options::ARGV0)
                .overrides_with(options::ARGV0)
                .short('a')
                .long(options::ARGV0)
                .value_name("a")
                .action(ArgAction::Set)
                .value_parser(ValueParser::os_string())
                .help(translate!("env-help-argv0")),
        )
        .arg(
            Arg::new("vars")
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string()),
        )
        .arg(
            Arg::new(options::IGNORE_SIGNAL)
                .long(options::IGNORE_SIGNAL)
                .value_name("SIG")
                .num_args(0..=1)
                .require_equals(true)
                .action(ArgAction::Append)
                .default_missing_value("")
                .value_parser(ValueParser::os_string())
                .help(translate!("env-help-ignore-signal")),
        )
        .arg(
            Arg::new(options::DEFAULT_SIGNAL)
                .long(options::DEFAULT_SIGNAL)
                .value_name("SIG")
                .num_args(0..=1)
                .require_equals(true)
                .action(ArgAction::Append)
                .default_missing_value("")
                .value_parser(ValueParser::os_string())
                .help(translate!("env-help-default-signal")),
        )
        .arg(
            Arg::new(options::BLOCK_SIGNAL)
                .long(options::BLOCK_SIGNAL)
                .value_name("SIG")
                .num_args(0..=1)
                .require_equals(true)
                .action(ArgAction::Append)
                .default_missing_value("")
                .value_parser(ValueParser::os_string())
                .help(translate!("env-help-block-signal")),
        )
        .arg(
            Arg::new(options::LIST_SIGNAL_HANDLING)
                .long(options::LIST_SIGNAL_HANDLING)
                .action(ArgAction::SetTrue)
                .help(translate!("env-help-list-signal-handling")),
        )
}

pub fn parse_args_from_str(text: &NativeIntStr) -> UResult<Vec<NativeIntString>> {
    split_iterator::split(text).map_err(|e| match e {
        EnvError::EnvBackslashCNotAllowedInDoubleQuotes(_) => USimpleError::new(125, e.to_string()),
        EnvError::EnvInvalidBackslashAtEndOfStringInMinusS(_, _) => {
            USimpleError::new(125, e.to_string())
        }
        EnvError::EnvInvalidSequenceBackslashXInMinusS(_, _) => {
            USimpleError::new(125, e.to_string())
        }
        EnvError::EnvMissingClosingQuote(_, _) => USimpleError::new(125, e.to_string()),
        EnvError::EnvParsingOfVariableMissingClosingBrace(pos) => USimpleError::new(
            125,
            translate!("env-error-variable-name-issue", "position" => pos, "error" => e),
        ),
        EnvError::EnvParsingOfMissingVariable(pos) => USimpleError::new(
            125,
            translate!("env-error-variable-name-issue", "position" => pos, "error" => e),
        ),
        EnvError::EnvParsingOfVariableMissingClosingBraceAfterValue(pos) => USimpleError::new(
            125,
            translate!("env-error-variable-name-issue", "position" => pos, "error" => e),
        ),
        EnvError::EnvParsingOfVariableUnexpectedNumber(pos, _) => USimpleError::new(
            125,
            translate!("env-error-variable-name-issue", "position" => pos, "error" => e),
        ),
        EnvError::EnvParsingOfVariableExceptedBraceOrColon(pos, _) => USimpleError::new(
            125,
            translate!("env-error-variable-name-issue", "position" => pos, "error" => e),
        ),
        _ => USimpleError::new(
            125,
            translate!("env-error-generic", "error" => format!("{e:?}")),
        ),
    })
}

fn debug_print_args(args: &[OsString]) {
    eprintln!("input args:");
    for (i, arg) in args.iter().enumerate() {
        eprintln!("arg[{i}]: {}", arg.quote());
    }
}

fn check_and_handle_string_args(
    arg: &OsString,
    prefix_to_test: &str,
    all_args: &mut Vec<OsString>,
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
        uucore::show_error!(
            "{}",
            translate!("env-error-no-such-file", "program" => prog.quote())
        );
        if !self.had_string_argument {
            uucore::show_error!("{}", translate!("env-error-use-s-shebang"));
        }
        ExitCode::new(127)
    }

    fn process_all_string_arguments(
        &mut self,
        original_args: &Vec<OsString>,
    ) -> UResult<Vec<OsString>> {
        let mut all_args: Vec<OsString> = Vec::new();
        let mut process_flags = true;
        let mut expecting_arg = false;
        // Leave out split-string since it's a special case below
        let flags_with_args = [
            options::ARGV0,
            options::CHDIR,
            options::FILE,
            options::UNSET,
        ];
        let short_flags_with_args = ['a', 'C', 'f', 'u'];
        for (n, arg) in original_args.iter().enumerate() {
            let arg_str = arg.to_string_lossy();
            // Stop processing env flags once we reach the command or -- argument
            if 0 < n
                && !expecting_arg
                && (arg == "--" || !(arg_str.starts_with('-') || arg_str.contains('=')))
            {
                process_flags = false;
            }
            if !process_flags {
                all_args.push(arg.clone());
                continue;
            }
            expecting_arg = false;
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
                    if let Some(flag) = arg_str.strip_prefix("--") {
                        if flags_with_args.contains(&flag) {
                            expecting_arg = true;
                        }
                    } else if let Some(flag) = arg_str.strip_prefix("-") {
                        for c in flag.chars() {
                            expecting_arg = short_flags_with_args.contains(&c);
                        }
                    }
                    // Short unset option (-u) is not allowed to contain '='
                    if arg_str.contains('=')
                        && arg_str.starts_with("-u")
                        && !arg_str.starts_with("--")
                    {
                        let name = &arg_str[arg_str.find('=').unwrap()..];
                        return Err(USimpleError::new(
                            125,
                            translate!("env-error-cannot-unset", "name" => name),
                        ));
                    }

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
        let mut args = self.process_all_string_arguments(&original_args)?;

        for arg in &mut args {
            if arg == "--ignore-signal" {
                *arg = OsString::from("--ignore-signal=__ALL__");
            } else if arg == "--default-signal" {
                *arg = OsString::from("--default-signal=__ALL__");
            } else if arg == "--block-signal" {
                *arg = OsString::from("--block-signal=__ALL__");
            }
        }

        let app = uu_app();
        let matches = match app.try_get_matches_from(args) {
            Ok(matches) => matches,
            Err(e) => {
                match e.kind() {
                    clap::error::ErrorKind::DisplayHelp
                    | clap::error::ErrorKind::DisplayVersion => return Err(e.into()),
                    _ => {
                        // Use ErrorFormatter directly to handle error with shebang message callback
                        let formatter =
                            uucore::clap_localization::ErrorFormatter::new(uucore::util_name());
                        formatter.print_error_and_exit_with_callback(&e, 125, || {
                            eprintln!(
                                "{}: {}",
                                uucore::util_name(),
                                translate!("env-error-use-s-shebang")
                            );
                        });
                    }
                }
            }
        };
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
        {
            let mut signal_action_log = SignalActionLog::default();

            // Rust ignores SIGPIPE (see https://github.com/rust-lang/rust/issues/62569).
            // We restore its default action here, but only if the user hasn't explicitly
            // specified signal handling for SIGPIPE via --ignore-signal or --block-signal.
            let sigpipe_value = nix::libc::SIGPIPE as usize;
            let user_handled_sigpipe = opts.ignore_signal.signals.contains(&sigpipe_value)
                || opts.block_signal.signals.contains(&sigpipe_value);

            if !user_handled_sigpipe
                && !opts.ignore_signal.apply_all
                && !opts.block_signal.apply_all
            {
                // Only restore SIGPIPE to default if user hasn't explicitly handled it
                unsafe {
                    libc::signal(libc::SIGPIPE, libc::SIG_DFL);
                }
            }

            apply_signal_action(
                &opts.default_signal,
                &mut signal_action_log,
                SignalActionKind::Default,
                reset_signal,
            )?;
            apply_signal_action(
                &opts.ignore_signal,
                &mut signal_action_log,
                SignalActionKind::Ignore,
                ignore_signal,
            )?;
            apply_signal_action(
                &opts.block_signal,
                &mut signal_action_log,
                SignalActionKind::Block,
                block_signal,
            )?;
            if opts.list_signal_handling {
                list_signal_handling(&signal_action_log);
            }
        }

        if opts.program.is_empty() {
            // no program provided, so just dump all env vars to stdout
            print_all_env_vars(opts.line_ending)?;
        } else {
            return self.run_program(&opts, self.do_debug_printing);
        }

        Ok(())
    }

    /// Run the program specified by the options.
    ///
    /// Note that the env command must exec the program, not spawn it. See
    /// <https://github.com/uutils/coreutils/issues/8361> for more information.
    ///
    /// Exit status:
    /// - 125: if the env command itself fails
    /// - 126: if the program is found but cannot be invoked
    /// - 127: if the program cannot be found
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

        if let Some(_argv0) = opts.argv0 {
            #[cfg(unix)]
            {
                arg0 = Cow::Borrowed(_argv0);
                if do_debug_printing {
                    eprintln!("argv0:     {}", arg0.quote());
                }
            }

            #[cfg(not(unix))]
            return Err(USimpleError::new(
                2,
                translate!("env-error-argv0-not-supported"),
            ));
        }

        if do_debug_printing {
            eprintln!("executing: {}", prog.maybe_quote());
            let arg_prefix = "   arg";
            eprintln!("{arg_prefix}[{}]= {}", 0, arg0.quote());
            for (i, arg) in args.iter().enumerate() {
                eprintln!("{arg_prefix}[{}]= {}", i + 1, arg.quote());
            }
        }

        #[cfg(unix)]
        {
            // Convert program name to CString.
            let prog_os: &OsStr = prog.as_ref();
            let Ok(prog_cstring) = CString::new(prog_os.as_bytes()) else {
                return Err(self.make_error_no_such_file_or_dir(&prog));
            };

            // Prepare arguments for execvp.
            let mut argv = Vec::new();

            // Convert arg0 to CString.
            let arg0_os: &OsStr = arg0.as_ref();
            let Ok(arg0_cstring) = CString::new(arg0_os.as_bytes()) else {
                return Err(self.make_error_no_such_file_or_dir(&prog));
            };
            argv.push(arg0_cstring);

            // Convert remaining arguments to CString.
            for arg in args {
                let arg_os = arg;
                let Ok(arg_cstring) = CString::new(arg_os.as_bytes()) else {
                    return Err(self.make_error_no_such_file_or_dir(&prog));
                };
                argv.push(arg_cstring);
            }

            // Execute the program using execvp. this replaces the current
            // process. The execvp function takes care of appending a NULL
            // argument to the argument list so that we don't have to.
            match execvp(&prog_cstring, &argv) {
                Err(nix::errno::Errno::ENOENT) => Err(self.make_error_no_such_file_or_dir(&prog)),
                Err(nix::errno::Errno::EACCES) => {
                    uucore::show_error!(
                        "{}",
                        translate!(
                            "env-error-permission-denied",
                            "program" => prog.quote()
                        )
                    );
                    Err(126.into())
                }
                Err(_) => {
                    uucore::show_error!(
                        "{}",
                        translate!(
                            "env-error-unknown",
                            "error" => "execvp failed"
                        )
                    );
                    Err(126.into())
                }
                Ok(_) => {
                    unreachable!("execvp should never return on success")
                }
            }
        }

        #[cfg(not(unix))]
        {
            // Fallback to Command::status for non-Unix systems
            let mut cmd = std::process::Command::new(&*prog);
            cmd.args(args);

            match cmd.status() {
                Ok(exit) if !exit.success() => Err(exit.code().unwrap_or(1).into()),
                Err(ref err) => match err.kind() {
                    io::ErrorKind::NotFound | io::ErrorKind::InvalidInput => {
                        Err(self.make_error_no_such_file_or_dir(&prog))
                    }
                    io::ErrorKind::PermissionDenied => {
                        uucore::show_error!(
                            "{}",
                            translate!("env-error-permission-denied", "program" => prog.quote())
                        );
                        Err(126.into())
                    }
                    _ => {
                        uucore::show_error!(
                            "{}",
                            translate!("env-error-unknown", "error" => format!("{err:?}"))
                        );
                        Err(126.into())
                    }
                },
                Ok(_) => Ok(()),
            }
        }
    }
}

fn apply_removal_of_all_env_vars(opts: &Options<'_>) {
    // remove all env vars if told to ignore presets
    if opts.ignore_env {
        for (ref name, _) in env::vars_os() {
            unsafe {
                env::remove_var(name);
            }
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

    #[cfg(unix)]
    let ignore_signal = build_signal_request(matches, options::IGNORE_SIGNAL)?;
    #[cfg(unix)]
    let default_signal = build_signal_request(matches, options::DEFAULT_SIGNAL)?;
    #[cfg(unix)]
    let block_signal = build_signal_request(matches, options::BLOCK_SIGNAL)?;
    #[cfg(unix)]
    let list_signal_handling = matches.get_flag(options::LIST_SIGNAL_HANDLING);

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
        ignore_signal,
        #[cfg(unix)]
        default_signal,
        #[cfg(unix)]
        block_signal,
        #[cfg(unix)]
        list_signal_handling,
    };

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
                translate!("env-error-cannot-unset-invalid", "name" => name.quote()),
            ));
        }
        unsafe {
            env::remove_var(name);
        }
    }
    Ok(())
}

fn apply_change_directory(opts: &Options<'_>) -> Result<(), Box<dyn UError>> {
    // GNU env tests this behavior
    if opts.program.is_empty() && opts.running_directory.is_some() {
        return Err(UUsageError::new(
            125,
            translate!("env-error-must-specify-command-with-chdir"),
        ));
    }

    if let Some(d) = opts.running_directory {
        match env::set_current_dir(d) {
            Ok(()) => d,
            Err(error) => {
                return Err(USimpleError::new(
                    125,
                    translate!("env-error-cannot-change-directory", "directory" => d.quote(), "error" => error),
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
            show_warning!(
                "{}",
                translate!("env-warning-no-name-specified", "value" => val.quote())
            );
            continue;
        }
        unsafe {
            env::set_var(name, val);
        }
    }
}

#[cfg(unix)]
fn apply_signal_action<F>(
    request: &SignalRequest,
    log: &mut SignalActionLog,
    action_kind: SignalActionKind,
    signal_fn: F,
) -> UResult<()>
where
    F: Fn(Signal) -> UResult<()>,
{
    request.for_each_signal(|sig_value, explicit| {
        // On some platforms ALL_SIGNALS may contain values that are not valid in libc.
        // Skip those invalid ones and continue (GNU env also ignores undefined signals).
        let Ok(sig) = signal_from_value(sig_value) else {
            return Ok(());
        };
        signal_fn(sig)?;
        log.record(sig_value, action_kind, explicit);

        // Set environment variable to communicate to Rust child processes
        // that SIGPIPE should be default (not ignored)
        if matches!(action_kind, SignalActionKind::Default)
            && sig_value == nix::libc::SIGPIPE as usize
        {
            unsafe {
                std::env::set_var("RUST_SIGPIPE", "default");
            }
        }

        Ok(())
    })
}

#[cfg(unix)]
fn ignore_signal(sig: Signal) -> UResult<()> {
    // SAFETY: This is safe because we write the handler for each signal only once, and therefore "the current handler is the default", as the documentation requires it.
    let result = unsafe { signal(sig, SigIgn) };
    if let Err(err) = result {
        return Err(USimpleError::new(
            125,
            translate!("env-error-failed-set-signal-action", "signal" => (sig as i32), "error" => err.desc()),
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn reset_signal(sig: Signal) -> UResult<()> {
    let result = unsafe { signal(sig, SigDfl) };
    if let Err(err) = result {
        return Err(USimpleError::new(
            125,
            translate!("env-error-failed-set-signal-action", "signal" => (sig as i32), "error" => err.desc()),
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn block_signal(sig: Signal) -> UResult<()> {
    let mut set = SigSet::empty();
    set.add(sig);
    if let Err(err) = sigprocmask(SigmaskHow::SIG_BLOCK, Some(&set), None) {
        return Err(USimpleError::new(
            125,
            translate!(
                "env-error-failed-set-signal-action",
                "signal" => (sig as i32),
                "error" => err.desc()
            ),
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn list_signal_handling(log: &SignalActionLog) {
    for (&sig_value, record) in &log.records {
        if !record.explicit {
            continue;
        }
        let action = match record.kind {
            SignalActionKind::Default => "DEFAULT",
            SignalActionKind::Ignore => "IGNORE",
            SignalActionKind::Block => "BLOCK",
        };
        let signal_name = signal_name_by_value(sig_value).unwrap_or("?");
        eprintln!("{:<10} ({}): {}", signal_name, sig_value as i32, action);
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    EnvAppData::default().run_env(args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uucore::locale;

    #[test]
    fn test_split_string_environment_vars_test() {
        unsafe { env::set_var("FOO", "BAR") };
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
            parse_args_from_str(&NCvt::convert(r"A=B FOO=AR  sh -c 'echo $A$FOO'")).unwrap()
        );
        assert_eq!(
            NCvt::convert(vec!["A=B", "FOO=AR", "sh", "-c", "echo $A$FOO"]),
            parse_args_from_str(&NCvt::convert(r"A=B FOO=AR  sh -c 'echo $A$FOO'")).unwrap()
        );

        assert_eq!(
            NCvt::convert(vec!["-i", "A=B ' C"]),
            parse_args_from_str(&NCvt::convert(r"-i A='B \' C'")).unwrap()
        );
    }

    #[test]
    fn test_error_cases() {
        let _ = locale::setup_localization("env");

        // Test EnvBackslashCNotAllowedInDoubleQuotes
        let result = parse_args_from_str(&NCvt::convert(r#"sh -c "echo \c""#));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "'\\c' must not appear in double-quoted -S string at position 13"
        );

        // Test EnvInvalidBackslashAtEndOfStringInMinusS
        let result = parse_args_from_str(&NCvt::convert(r#"sh -c "echo \"#));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "no terminating quote in -S string at position 13 for quote '\"'"
        );

        // Test EnvInvalidSequenceBackslashXInMinusS
        let result = parse_args_from_str(&NCvt::convert(r#"sh -c "echo \x""#));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("invalid sequence '\\x' in -S")
        );

        // Test EnvMissingClosingQuote
        let result = parse_args_from_str(&NCvt::convert(r#"sh -c "echo "#));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "no terminating quote in -S string at position 12 for quote '\"'"
        );

        // Test variable-related errors
        let result = parse_args_from_str(&NCvt::convert(r"echo ${FOO"));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("variable name issue (at 10): Missing closing brace")
        );

        let result = parse_args_from_str(&NCvt::convert(r"echo ${FOO:-value"));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("variable name issue (at 17): Missing closing brace after default value")
        );

        let result = parse_args_from_str(&NCvt::convert(r"echo ${1FOO}"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("variable name issue (at 7): Unexpected character: '1', expected variable name must not start with 0..9"));

        let result = parse_args_from_str(&NCvt::convert(r"echo ${FOO?}"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("variable name issue (at 10): Unexpected character: '?', expected a closing brace ('}') or colon (':')"));
    }
}
