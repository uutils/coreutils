// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{crate_version, Arg, ArgAction, Command};
use uucore::{format_usage, help_about, help_usage};

const ABOUT: &str = help_about!("timeout.md");
const USAGE: &str = help_usage!("timeout.md");

pub mod options {
    pub static FOREGROUND: &str = "foreground";
    pub static KILL_AFTER: &str = "kill-after";
    pub static SIGNAL: &str = "signal";
    pub static PRESERVE_STATUS: &str = "preserve-status";
    pub static VERBOSE: &str = "verbose";

    // Positional args.
    pub static DURATION: &str = "duration";
    pub static COMMAND: &str = "command";
}

pub fn uu_app() -> Command {
    Command::new("timeout")
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .arg(
            Arg::new(options::FOREGROUND)
                .long(options::FOREGROUND)
                .help(
                    "when not running timeout directly from a shell prompt, allow \
                COMMAND to read from the TTY and get TTY signals; in this mode, \
                children of COMMAND will not be timed out",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::KILL_AFTER)
                .long(options::KILL_AFTER)
                .short('k')
                .help(
                    "also send a KILL signal if COMMAND is still running this long \
                after the initial signal was sent",
                ),
        )
        .arg(
            Arg::new(options::PRESERVE_STATUS)
                .long(options::PRESERVE_STATUS)
                .help("exit with the same status as COMMAND, even when the command times out")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::SIGNAL)
                .short('s')
                .long(options::SIGNAL)
                .help(
                    "specify the signal to be sent on timeout; SIGNAL may be a name like \
                'HUP' or a number; see 'kill -l' for a list of signals",
                )
                .value_name("SIGNAL"),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .short('v')
                .long(options::VERBOSE)
                .help("diagnose to stderr any signal sent upon timeout")
                .action(ArgAction::SetTrue),
        )
        .arg(Arg::new(options::DURATION).required(true))
        .arg(
            Arg::new(options::COMMAND)
                .required(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::CommandName),
        )
        .trailing_var_arg(true)
        .infer_long_args(true)
}
