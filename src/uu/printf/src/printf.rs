// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) devnull

use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use std::io::stdout;
use std::ops::ControlFlow;
use uucore::display::Quotable;
use uucore::error::{UResult, UUsageError};
use uucore::format::{FormatArgument, FormatArguments, FormatItem, parse_spec_and_escape};
use uucore::translate;
use uucore::{format_usage, os_str_as_bytes, show_warning};

const VERSION: &str = "version";
const HELP: &str = "help";

mod options {
    pub const FORMAT: &str = "FORMAT";
    pub const ARGUMENT: &str = "ARGUMENT";
}

mod stdout_state {
    use std::io;
    use std::io::Write;
    use std::sync::atomic::{AtomicBool, Ordering};

    static STDOUT_WRITTEN: AtomicBool = AtomicBool::new(false);
    static STDOUT_WAS_CLOSED: AtomicBool = AtomicBool::new(false);
    static STDOUT_WAS_CLOSED_SET: AtomicBool = AtomicBool::new(false);

    pub fn reset_stdout_written() {
        STDOUT_WRITTEN.store(false, Ordering::Relaxed);
    }

    fn mark_stdout_written() {
        STDOUT_WRITTEN.store(true, Ordering::Relaxed);
    }

    fn stdout_was_written() -> bool {
        STDOUT_WRITTEN.load(Ordering::Relaxed)
    }

    fn set_stdout_was_closed(value: bool) {
        STDOUT_WAS_CLOSED.store(value, Ordering::Relaxed);
        STDOUT_WAS_CLOSED_SET.store(true, Ordering::Relaxed);
    }

    fn stdout_was_closed() -> bool {
        STDOUT_WAS_CLOSED.load(Ordering::Relaxed)
    }

    pub fn init_stdout_state() {
        if !STDOUT_WAS_CLOSED_SET.load(Ordering::Relaxed) {
            set_stdout_was_closed(stdout_is_closed());
        }
    }

    #[cfg(unix)]
    fn stdout_is_closed() -> bool {
        let res = unsafe { libc::fcntl(libc::STDOUT_FILENO, libc::F_GETFL) };
        if res != -1 {
            return false;
        }
        matches!(io::Error::last_os_error().raw_os_error(), Some(libc::EBADF))
    }

    #[cfg(not(unix))]
    fn stdout_is_closed() -> bool {
        false
    }

    #[cfg(unix)]
    mod early_stdout_state {
        use super::{set_stdout_was_closed, stdout_is_closed};

        extern "C" fn init() {
            set_stdout_was_closed(stdout_is_closed());
        }

        #[used]
        #[cfg_attr(target_os = "macos", unsafe(link_section = "__DATA,__mod_init_func"))]
        #[cfg_attr(not(target_os = "macos"), unsafe(link_section = ".init_array"))]
        static INIT: extern "C" fn() = init;
    }

    pub fn check_stdout_write(len: usize) -> io::Result<()> {
        if len == 0 {
            return Ok(());
        }
        mark_stdout_written();
        if stdout_was_closed() {
            #[cfg(unix)]
            {
                return Err(io::Error::from_raw_os_error(libc::EBADF));
            }
            #[cfg(not(unix))]
            {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "stdout was closed"));
            }
        }
        Ok(())
    }

    #[cfg(unix)]
    fn redirect_stdout_to_devnull() {
        use std::fs::OpenOptions;
        use std::os::unix::io::AsRawFd;

        if let Ok(devnull) = OpenOptions::new().write(true).open("/dev/null") {
            unsafe {
                libc::dup2(devnull.as_raw_fd(), libc::STDOUT_FILENO);
            }
        }
    }

    #[cfg(not(unix))]
    fn redirect_stdout_to_devnull() {}

    fn suppress_closed_stdout_flush() {
        if stdout_was_closed() && !stdout_was_written() {
            redirect_stdout_to_devnull();
        }
    }

    pub struct StdoutFlushGuard;

    impl StdoutFlushGuard {
        pub fn new() -> Self {
            Self
        }
    }

    impl Drop for StdoutFlushGuard {
        fn drop(&mut self) {
            suppress_closed_stdout_flush();
        }
    }

    pub struct TrackingWriter<W> {
        inner: W,
    }

    impl<W> TrackingWriter<W> {
        pub fn new(inner: W) -> Self {
            Self { inner }
        }
    }

    impl<W: Write> Write for TrackingWriter<W> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            check_stdout_write(buf.len())?;
            self.inner.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.inner.flush()
        }
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    stdout_state::init_stdout_state();
    stdout_state::reset_stdout_written();
    let _stdout_guard = stdout_state::StdoutFlushGuard::new();
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    let format = matches
        .get_one::<OsString>(options::FORMAT)
        .ok_or_else(|| UUsageError::new(1, translate!("printf-error-missing-operand")))?;
    let format = os_str_as_bytes(format)?;

    let values: Vec<_> = match matches.get_many::<OsString>(options::ARGUMENT) {
        Some(s) => s
            .map(|os_string| FormatArgument::Unparsed(os_string.to_owned()))
            .collect(),
        None => vec![],
    };

    let mut format_seen = false;
    // Parse and process the format string
    let mut stdout = stdout_state::TrackingWriter::new(stdout());
    let mut args = FormatArguments::new(&values);
    for item in parse_spec_and_escape(format) {
        if let Ok(FormatItem::Spec(_)) = item {
            format_seen = true;
        }
        match item?.write(&mut stdout, &mut args)? {
            ControlFlow::Continue(()) => {}
            ControlFlow::Break(()) => return Ok(()),
        }
    }
    args.start_next_batch();

    // Without format specs in the string, the iter would not consume any args,
    // leading to an infinite loop. Thus, we exit early.
    if !format_seen {
        if !args.is_exhausted() {
            let Some(FormatArgument::Unparsed(arg_str)) = args.peek_arg() else {
                unreachable!("All args are transformed to Unparsed")
            };
            show_warning!(
                "{}",
                translate!(
                    "printf-warning-ignoring-excess-arguments",
                    "arg" => arg_str.quote()
                )
            );
        }
        return Ok(());
    }

    while !args.is_exhausted() {
        for item in parse_spec_and_escape(format) {
            match item?.write(&mut stdout, &mut args)? {
                ControlFlow::Continue(()) => {}
                ControlFlow::Break(()) => return Ok(()),
            }
        }
        args.start_next_batch();
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .allow_hyphen_values(true)
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("printf-about"))
        .after_help(translate!("printf-after-help"))
        .override_usage(format_usage(&translate!("printf-usage")))
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new(HELP)
                .long(HELP)
                .help(translate!("printf-help-help"))
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new(VERSION)
                .long(VERSION)
                .help(translate!("printf-help-version"))
                .action(ArgAction::Version),
        )
        .arg(Arg::new(options::FORMAT).value_parser(clap::value_parser!(OsString)))
        .arg(
            Arg::new(options::ARGUMENT)
                .action(ArgAction::Append)
                .value_parser(clap::value_parser!(OsString)),
        )
}
