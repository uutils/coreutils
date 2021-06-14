use clap::{crate_version, App, AppSettings, Arg};

const ABOUT: &str = "Start COMMAND, and kill it if still running after DURATION.";

pub mod options {
    pub const FOREGROUND: &str = "foreground";
    pub const KILL_AFTER: &str = "kill-after";
    pub const SIGNAL: &str = "signal";
    pub const PRESERVE_STATUS: &str = "preserve-status";

    // Positional args.
    pub const DURATION: &str = "duration";
    pub const COMMAND: &str = "command";
    pub const ARGS: &str = "args";
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::FOREGROUND)
                .long(options::FOREGROUND)
                .help("when not running timeout directly from a shell prompt, allow COMMAND to read from the TTY and get TTY signals; in this mode, children of COMMAND will not be timed out")
        )
        .arg(
            Arg::with_name(options::KILL_AFTER)
                .short("k")
                .takes_value(true))
        .arg(
            Arg::with_name(options::PRESERVE_STATUS)
                .long(options::PRESERVE_STATUS)
                .help("exit with the same status as COMMAND, even when the command times out")
        )
        .arg(
            Arg::with_name(options::SIGNAL)
                .short("s")
                .long(options::SIGNAL)
                .help("specify the signal to be sent on timeout; SIGNAL may be a name like 'HUP' or a number; see 'kill -l' for a list of signals")
                .takes_value(true)
        )
        .arg(
            Arg::with_name(options::DURATION)
                .index(1)
                .required(true)
        )
        .arg(
            Arg::with_name(options::COMMAND)
                .index(2)
                .required(true)
        )
        .arg(
            Arg::with_name(options::ARGS).multiple(true)
        )
        .setting(AppSettings::TrailingVarArg)
}
