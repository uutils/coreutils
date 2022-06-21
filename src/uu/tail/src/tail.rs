//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Morten Olsen Lysgaard <morten@lysgaard.no>
//  * (c) Alexander Batischev <eual.jp@gmail.com>
//  * (c) Thomas Queiroz <thomasqueirozb@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) seekable seek'd tail'ing ringbuffer ringbuf unwatch Uncategorized
// spell-checker:ignore (libs) kqueue
// spell-checker:ignore (acronyms)
// spell-checker:ignore (env/flags)
// spell-checker:ignore (jargon) tailable untailable
// spell-checker:ignore (names)
// spell-checker:ignore (shell/tools)
// spell-checker:ignore (misc)

#[macro_use]
extern crate clap;

#[macro_use]
extern crate uucore;

mod chunks;
mod parse;
mod platform;
use crate::files::FileHandling;
use chunks::ReverseChunks;

use clap::{Arg, Command};
use notify::{RecommendedWatcher, RecursiveMode, Watcher, WatcherKind};
use std::collections::{HashMap, VecDeque};
use std::ffi::OsString;
use std::fmt;
use std::fs::{File, Metadata};
use std::io::{stdin, stdout, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, channel};
use std::time::Duration;
use uucore::display::Quotable;
use uucore::error::{
    get_exit_code, set_exit_code, FromIo, UError, UResult, USimpleError, UUsageError,
};
use uucore::format_usage;
use uucore::lines::lines;
use uucore::parse_size::{parse_size, ParseSizeError};
use uucore::ringbuffer::RingBuffer;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use std::os::unix::prelude::FileTypeExt;

const ABOUT: &str = "\
    Print the last 10 lines of each FILE to standard output.\n\
    With more than one FILE, precede each with a header giving the file name.\n\
    With no FILE, or when FILE is -, read standard input.\n\
    \n\
    Mandatory arguments to long flags are mandatory for short flags too.\
    ";
const USAGE: &str = "{} [FLAG]... [FILE]...";

pub mod text {
    pub static DASH: &str = "-";
    pub static DEV_STDIN: &str = "/dev/stdin";
    pub static STDIN_HEADER: &str = "standard input";
    pub static NO_FILES_REMAINING: &str = "no files remaining";
    pub static NO_SUCH_FILE: &str = "No such file or directory";
    pub static BECOME_INACCESSIBLE: &str = "has become inaccessible";
    pub static BAD_FD: &str = "Bad file descriptor";
    #[cfg(target_os = "linux")]
    pub static BACKEND: &str = "inotify";
    #[cfg(all(unix, not(target_os = "linux")))]
    pub static BACKEND: &str = "kqueue";
    #[cfg(target_os = "windows")]
    pub static BACKEND: &str = "ReadDirectoryChanges";
}

pub mod options {
    pub mod verbosity {
        pub static QUIET: &str = "quiet";
        pub static VERBOSE: &str = "verbose";
    }
    pub static BYTES: &str = "bytes";
    pub static FOLLOW: &str = "follow";
    pub static LINES: &str = "lines";
    pub static PID: &str = "pid";
    pub static SLEEP_INT: &str = "sleep-interval";
    pub static ZERO_TERM: &str = "zero-terminated";
    pub static DISABLE_INOTIFY_TERM: &str = "-disable-inotify"; // NOTE: three hyphens is correct
    pub static USE_POLLING: &str = "use-polling";
    pub static RETRY: &str = "retry";
    pub static FOLLOW_RETRY: &str = "F";
    pub static MAX_UNCHANGED_STATS: &str = "max-unchanged-stats";
    pub static ARG_FILES: &str = "files";
    pub static PRESUME_INPUT_PIPE: &str = "-presume-input-pipe"; // NOTE: three hyphens is correct
}

#[derive(Debug, PartialEq, Eq)]
enum FilterMode {
    Bytes(u64),
    Lines(u64, u8), // (number of lines, delimiter)
}

impl Default for FilterMode {
    fn default() -> Self {
        Self::Lines(10, b'\n')
    }
}

#[derive(Debug, PartialEq, Eq)]
enum FollowMode {
    Descriptor,
    Name,
}

#[derive(Debug, Default)]
pub struct Settings {
    beginning: bool,
    follow: Option<FollowMode>,
    max_unchanged_stats: u32,
    mode: FilterMode,
    paths: VecDeque<PathBuf>,
    pid: platform::Pid,
    retry: bool,
    sleep_sec: Duration,
    use_polling: bool,
    verbose: bool,
    stdin_is_pipe_or_fifo: bool,
}

impl Settings {
    pub fn from(matches: &clap::ArgMatches) -> UResult<Self> {
        let mut settings: Self = Self {
            sleep_sec: Duration::from_secs_f32(1.0),
            max_unchanged_stats: 5,
            ..Default::default()
        };

        settings.follow = if matches.is_present(options::FOLLOW_RETRY) {
            Some(FollowMode::Name)
        } else if matches.occurrences_of(options::FOLLOW) == 0 {
            None
        } else if matches.value_of(options::FOLLOW) == Some("name") {
            Some(FollowMode::Name)
        } else {
            Some(FollowMode::Descriptor)
        };

        if let Some(s) = matches.value_of(options::SLEEP_INT) {
            settings.sleep_sec = match s.parse::<f32>() {
                Ok(s) => Duration::from_secs_f32(s),
                Err(_) => {
                    return Err(UUsageError::new(
                        1,
                        format!("invalid number of seconds: {}", s.quote()),
                    ))
                }
            }
        }

        settings.use_polling = matches.is_present(options::USE_POLLING);

        if let Some(s) = matches.value_of(options::MAX_UNCHANGED_STATS) {
            settings.max_unchanged_stats = match s.parse::<u32>() {
                Ok(s) => s,
                Err(_) => {
                    // TODO: [2021-10; jhscheer] add test for this
                    return Err(UUsageError::new(
                        1,
                        format!(
                            "invalid maximum number of unchanged stats between opens: {}",
                            s.quote()
                        ),
                    ));
                }
            }
        }

        if let Some(pid_str) = matches.value_of(options::PID) {
            match pid_str.parse() {
                Ok(pid) => {
                    // NOTE: on unix platform::Pid is i32, on windows platform::Pid is u32
                    #[cfg(unix)]
                    if pid < 0 {
                        // NOTE: tail only accepts an unsigned pid
                        return Err(USimpleError::new(
                            1,
                            format!("invalid PID: {}", pid_str.quote()),
                        ));
                    }
                    settings.pid = pid;
                    if settings.follow.is_none() {
                        show_warning!("PID ignored; --pid=PID is useful only when following");
                    }
                    if !platform::supports_pid_checks(settings.pid) {
                        show_warning!("--pid=PID is not supported on this system");
                        settings.pid = 0;
                    }
                }
                Err(e) => {
                    return Err(USimpleError::new(
                        1,
                        format!("invalid PID: {}: {}", pid_str.quote(), e),
                    ));
                }
            }
        }

        let mut starts_with_plus = false; // support for legacy format (+0)
        let mode_and_beginning = if let Some(arg) = matches.value_of(options::BYTES) {
            starts_with_plus = arg.starts_with('+');
            match parse_num(arg) {
                Ok((n, beginning)) => (FilterMode::Bytes(n), beginning),
                Err(e) => {
                    return Err(UUsageError::new(
                        1,
                        format!("invalid number of bytes: {}", e),
                    ))
                }
            }
        } else if let Some(arg) = matches.value_of(options::LINES) {
            starts_with_plus = arg.starts_with('+');
            match parse_num(arg) {
                Ok((n, beginning)) => (FilterMode::Lines(n, b'\n'), beginning),
                Err(e) => {
                    return Err(UUsageError::new(
                        1,
                        format!("invalid number of lines: {}", e),
                    ))
                }
            }
        } else {
            (FilterMode::default(), false)
        };
        settings.mode = mode_and_beginning.0;
        settings.beginning = mode_and_beginning.1;

        // Mimic GNU's tail for -[nc]0 without -f and exit immediately
        if settings.follow.is_none() && !starts_with_plus && {
            if let FilterMode::Lines(l, _) = settings.mode {
                l == 0
            } else {
                settings.mode == FilterMode::Bytes(0)
            }
        } {
            std::process::exit(0)
        }

        settings.retry =
            matches.is_present(options::RETRY) || matches.is_present(options::FOLLOW_RETRY);

        if settings.retry && settings.follow.is_none() {
            show_warning!("--retry ignored; --retry is useful only when following");
        }

        if matches.is_present(options::ZERO_TERM) {
            if let FilterMode::Lines(count, _) = settings.mode {
                settings.mode = FilterMode::Lines(count, 0);
            }
        }

        settings.stdin_is_pipe_or_fifo = matches.is_present(options::PRESUME_INPUT_PIPE);

        settings.paths = matches
            .values_of(options::ARG_FILES)
            .map(|v| v.map(PathBuf::from).collect())
            .unwrap_or_default();

        settings.verbose = (matches.is_present(options::verbosity::VERBOSE)
            || settings.paths.len() > 1)
            && !matches.is_present(options::verbosity::QUIET);

        Ok(settings)
    }

    fn follow_descriptor(&self) -> bool {
        self.follow == Some(FollowMode::Descriptor)
    }

    fn follow_name(&self) -> bool {
        self.follow == Some(FollowMode::Name)
    }

    fn follow_descriptor_retry(&self) -> bool {
        self.follow_descriptor() && self.retry
    }

    fn follow_name_retry(&self) -> bool {
        self.follow_name() && self.retry
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().get_matches_from(arg_iterate(args)?);
    let mut settings = Settings::from(&matches)?;

    // skip expensive call to fstat if PRESUME_INPUT_PIPE is selected
    if !settings.stdin_is_pipe_or_fifo {
        settings.stdin_is_pipe_or_fifo = stdin_is_pipe_or_fifo();
    }

    uu_tail(settings)
}

fn uu_tail(mut settings: Settings) -> UResult<()> {
    let dash = PathBuf::from(text::DASH);

    // Mimic GNU's tail for `tail -F` and exit immediately
    if (settings.paths.is_empty() || settings.paths.contains(&dash)) && settings.follow_name() {
        return Err(USimpleError::new(
            1,
            format!("cannot follow {} by name", text::DASH.quote()),
        ));
    }

    // add '-' to paths
    if !settings.paths.contains(&dash) && settings.stdin_is_pipe_or_fifo
        || settings.paths.is_empty() && !settings.stdin_is_pipe_or_fifo
    {
        settings.paths.push_front(dash);
    }

    let mut first_header = true;
    let mut files = FileHandling::with_capacity(settings.paths.len());

    // Do an initial tail print of each path's content.
    // Add `path` to `files` map if `--follow` is selected.
    for path in &settings.paths {
        let mut path = path.to_owned();
        let mut display_name = path.to_owned();

        // Workaround to handle redirects, e.g. `touch f && tail -f - < f`
        if cfg!(unix) && path.is_stdin() {
            display_name = PathBuf::from(text::STDIN_HEADER);
            if let Ok(p) = Path::new(text::DEV_STDIN).canonicalize() {
                path = p;
            } else {
                path = PathBuf::from(text::DEV_STDIN);
            }
        }

        // TODO: is there a better way to check for a readable stdin?
        let mut buf = [0; 0]; // empty buffer to check if stdin().read().is_err()
        let stdin_read_possible = settings.stdin_is_pipe_or_fifo && stdin().read(&mut buf).is_ok();

        let path_is_tailable = path.is_tailable();

        if !path.is_stdin() && !path_is_tailable {
            if settings.follow_descriptor_retry() {
                show_warning!("--retry only effective for the initial open");
            }

            if !path.exists() && !settings.stdin_is_pipe_or_fifo {
                set_exit_code(1);
                show_error!(
                    "cannot open {} for reading: {}",
                    display_name.quote(),
                    text::NO_SUCH_FILE
                );
            } else if path.is_dir() || display_name.is_stdin() && !stdin_read_possible {
                if settings.verbose {
                    files.print_header(&display_name, !first_header);
                    first_header = false;
                }
                let err_msg = "Is a directory".to_string();

                // NOTE: On macOS path.is_dir() can be false for directories
                // if it was a redirect, e.g. `$ tail < DIR`
                // if !path.is_dir() {
                // TODO: match against ErrorKind if unstable
                // library feature "io_error_more" becomes stable
                // if let Err(e) = stdin().read(&mut buf) {
                //     if e.kind() != std::io::ErrorKind::IsADirectory {
                //         err_msg = e.message.to_string();
                //     }
                // }
                // }

                set_exit_code(1);
                show_error!("error reading {}: {}", display_name.quote(), err_msg);
                if settings.follow.is_some() {
                    let msg = if !settings.retry {
                        "; giving up on this name"
                    } else {
                        ""
                    };
                    show_error!(
                        "{}: cannot follow end of this type of file{}",
                        display_name.display(),
                        msg
                    );
                }
                if !(settings.follow_name_retry()) {
                    // skip directory if not retry
                    continue;
                }
            } else {
                // TODO: [2021-10; jhscheer] how to handle block device or socket?
                todo!();
            }
        }

        let metadata = path.metadata().ok();

        if display_name.is_stdin() && path_is_tailable {
            if settings.verbose {
                files.print_header(Path::new(text::STDIN_HEADER), !first_header);
                first_header = false;
            }

            let mut reader = BufReader::new(stdin());
            if !stdin_is_bad_fd() {
                unbounded_tail(&mut reader, &settings)?;
                if settings.follow_descriptor() {
                    // Insert `stdin` into `files.map`
                    files.insert(
                        &path,
                        PathData {
                            reader: Some(Box::new(reader)),
                            metadata: None,
                            display_name: PathBuf::from(text::STDIN_HEADER),
                        },
                        true,
                    );
                }
            } else {
                set_exit_code(1);
                show_error!(
                    "cannot fstat {}: {}",
                    text::STDIN_HEADER.quote(),
                    text::BAD_FD
                );
                if settings.follow.is_some() {
                    show_error!(
                        "error reading {}: {}",
                        text::STDIN_HEADER.quote(),
                        text::BAD_FD
                    );
                }
            }
        } else if path_is_tailable {
            match File::open(&path) {
                Ok(mut file) => {
                    if settings.verbose {
                        files.print_header(&path, !first_header);
                        first_header = false;
                    }
                    let mut reader;

                    if file.is_seekable() && metadata.as_ref().unwrap().get_block_size() > 0 {
                        bounded_tail(&mut file, &settings);
                        reader = BufReader::new(file);
                    } else {
                        reader = BufReader::new(file);
                        unbounded_tail(&mut reader, &settings)?;
                    }
                    if settings.follow.is_some() {
                        // Insert existing/file `path` into `files.map`
                        files.insert(
                            &path,
                            PathData {
                                reader: Some(Box::new(reader)),
                                metadata,
                                display_name,
                            },
                            true,
                        );
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                    show!(e.map_err_context(|| {
                        format!("cannot open {} for reading", display_name.quote())
                    }));
                }
                Err(e) => {
                    return Err(e.map_err_context(|| {
                        format!("cannot open {} for reading", display_name.quote())
                    }));
                }
            }
        } else if settings.retry && settings.follow.is_some() {
            if path.is_relative() {
                path = std::env::current_dir()?.join(&path);
            }
            // Insert non-is_tailable() paths into `files.map`
            files.insert(
                &path,
                PathData {
                    reader: None,
                    metadata,
                    display_name,
                },
                false,
            );
        }
    }

    if settings.follow.is_some() {
        /*
        POSIX specification regarding tail -f
        If the input file is a regular file or if the file operand specifies a FIFO, do not
        terminate after the last line of the input file has been copied, but read and copy
        further bytes from the input file when they become available. If no file operand is
        specified and standard input is a pipe or FIFO, the -f option shall be ignored. If
        the input file is not a FIFO, pipe, or regular file, it is unspecified whether or
        not the -f option shall be ignored.
        */
        if files.no_files_remaining(&settings) {
            if !files.only_stdin_remaining() {
                show_error!("{}", text::NO_FILES_REMAINING);
            }
        } else if !(settings.stdin_is_pipe_or_fifo && settings.paths.len() == 1) {
            follow(&mut files, &mut settings)?;
        }
    }

    if get_exit_code() > 0 && stdin_is_bad_fd() {
        show_error!("-: {}", text::BAD_FD);
    }

    Ok(())
}

fn arg_iterate<'a>(
    mut args: impl uucore::Args + 'a,
) -> Result<Box<dyn Iterator<Item = OsString> + 'a>, Box<(dyn UError + 'static)>> {
    // argv[0] is always present
    let first = args.next().unwrap();
    if let Some(second) = args.next() {
        if let Some(s) = second.to_str() {
            match parse::parse_obsolete(s) {
                Some(Ok(iter)) => Ok(Box::new(vec![first].into_iter().chain(iter).chain(args))),
                Some(Err(e)) => Err(UUsageError::new(
                    1,
                    match e {
                        parse::ParseError::Syntax => format!("bad argument format: {}", s.quote()),
                        parse::ParseError::Overflow => format!(
                            "invalid argument: {} Value too large for defined datatype",
                            s.quote()
                        ),
                    },
                )),
                None => Ok(Box::new(vec![first, second].into_iter().chain(args))),
            }
        } else {
            Err(UUsageError::new(1, "bad argument encoding".to_owned()))
        }
    } else {
        Ok(Box::new(vec![first].into_iter()))
    }
}

pub fn uu_app<'a>() -> Command<'a> {
    #[cfg(target_os = "linux")]
    pub static POLLING_HELP: &str = "Disable 'inotify' support and use polling instead";
    #[cfg(all(unix, not(target_os = "linux")))]
    pub static POLLING_HELP: &str = "Disable 'kqueue' support and use polling instead";
    #[cfg(target_os = "windows")]
    pub static POLLING_HELP: &str =
        "Disable 'ReadDirectoryChanges' support and use polling instead";

    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(options::BYTES)
                .short('c')
                .long(options::BYTES)
                .takes_value(true)
                .allow_hyphen_values(true)
                .overrides_with_all(&[options::BYTES, options::LINES])
                .help("Number of bytes to print"),
        )
        .arg(
            Arg::new(options::FOLLOW)
                .short('f')
                .long(options::FOLLOW)
                .default_value("descriptor")
                .takes_value(true)
                .min_values(0)
                .max_values(1)
                .require_equals(true)
                .possible_values(&["descriptor", "name"])
                .help("Print the file as it grows"),
        )
        .arg(
            Arg::new(options::LINES)
                .short('n')
                .long(options::LINES)
                .takes_value(true)
                .allow_hyphen_values(true)
                .overrides_with_all(&[options::BYTES, options::LINES])
                .help("Number of lines to print"),
        )
        .arg(
            Arg::new(options::PID)
                .long(options::PID)
                .takes_value(true)
                .help("With -f, terminate after process ID, PID dies"),
        )
        .arg(
            Arg::new(options::verbosity::QUIET)
                .short('q')
                .long(options::verbosity::QUIET)
                .visible_alias("silent")
                .overrides_with_all(&[options::verbosity::QUIET, options::verbosity::VERBOSE])
                .help("Never output headers giving file names"),
        )
        .arg(
            Arg::new(options::SLEEP_INT)
                .short('s')
                .takes_value(true)
                .long(options::SLEEP_INT)
                .help("Number of seconds to sleep between polling the file when running with -f"),
        )
        .arg(
            Arg::new(options::MAX_UNCHANGED_STATS)
                .takes_value(true)
                .long(options::MAX_UNCHANGED_STATS)
                .help(
                    "Reopen a FILE which has not changed size after N (default 5) iterations \
                        to see if it has been unlinked or renamed (this is the usual case of rotated \
                        log files); This option is meaningful only when polling \
                        (i.e., with --use-polling) and when --follow=name",
                ),
        )
        .arg(
            Arg::new(options::verbosity::VERBOSE)
                .short('v')
                .long(options::verbosity::VERBOSE)
                .overrides_with_all(&[options::verbosity::QUIET, options::verbosity::VERBOSE])
                .help("Always output headers giving file names"),
        )
        .arg(
            Arg::new(options::ZERO_TERM)
                .short('z')
                .long(options::ZERO_TERM)
                .help("Line delimiter is NUL, not newline"),
        )
        .arg(
            Arg::new(options::USE_POLLING)
                .alias(options::DISABLE_INOTIFY_TERM) // NOTE: Used by GNU's test suite
                .alias("dis") // NOTE: Used by GNU's test suite
                .long(options::USE_POLLING)
                .help(POLLING_HELP),
        )
        .arg(
            Arg::new(options::RETRY)
                .long(options::RETRY)
                .help("Keep trying to open a file if it is inaccessible"),
        )
        .arg(
            Arg::new(options::FOLLOW_RETRY)
                .short('F')
                .help("Same as --follow=name --retry")
                .overrides_with_all(&[options::RETRY, options::FOLLOW]),
        )
        .arg(
            Arg::new(options::PRESUME_INPUT_PIPE)
                .long(options::PRESUME_INPUT_PIPE)
                .alias(options::PRESUME_INPUT_PIPE)
                .hide(true),
        )
        .arg(
            Arg::new(options::ARG_FILES)
                .multiple_occurrences(true)
                .takes_value(true)
                .min_values(1)
                .value_hint(clap::ValueHint::FilePath),
        )
}

fn follow(files: &mut FileHandling, settings: &mut Settings) -> UResult<()> {
    let mut process = platform::ProcessChecker::new(settings.pid);

    let (tx, rx) = channel();

    /*
    Watcher is implemented per platform using the best implementation available on that
    platform. In addition to such event driven implementations, a polling implementation
    is also provided that should work on any platform.
    Linux / Android: inotify
    macOS: FSEvents / kqueue
    Windows: ReadDirectoryChangesWatcher
    FreeBSD / NetBSD / OpenBSD / DragonflyBSD: kqueue
    Fallback: polling every n seconds

    NOTE:
    We force the use of kqueue with: features=["macos_kqueue"].
    On macOS only `kqueue` is suitable for our use case because `FSEvents`
    waits for file close util it delivers a modify event. See:
    https://github.com/notify-rs/notify/issues/240
    */

    let mut watcher: Box<dyn Watcher>;
    let watcher_config = notify::poll::PollWatcherConfig {
        poll_interval: settings.sleep_sec,
        /*
        NOTE: By enabling compare_contents, performance will be significantly impacted
        as all files will need to be read and hashed at each `poll_interval`.
        However, this is necessary to pass: "gnu/tests/tail-2/F-vs-rename.sh"
        */
        compare_contents: true,
    };
    if settings.use_polling || RecommendedWatcher::kind() == WatcherKind::PollWatcher {
        settings.use_polling = true; // We have to use polling because there's no supported backend
        watcher = Box::new(notify::PollWatcher::with_config(tx, watcher_config).unwrap());
    } else {
        let tx_clone = tx.clone();
        match notify::RecommendedWatcher::new(tx) {
            Ok(w) => watcher = Box::new(w),
            Err(e) if e.to_string().starts_with("Too many open files") => {
                /*
                NOTE: This ErrorKind is `Uncategorized`, but it is not recommended
                to match an error against `Uncategorized`
                NOTE: Could be tested with decreasing `max_user_instances`, e.g.:
                `sudo sysctl fs.inotify.max_user_instances=64`
                */
                show_error!(
                    "{} cannot be used, reverting to polling: Too many open files",
                    text::BACKEND
                );
                set_exit_code(1);
                settings.use_polling = true;
                watcher =
                    Box::new(notify::PollWatcher::with_config(tx_clone, watcher_config).unwrap());
            }
            Err(e) => return Err(USimpleError::new(1, e.to_string())),
        };
    }

    // Iterate user provided `paths`.
    // Add existing regular files to `Watcher` (InotifyWatcher).
    // If `path` is not an existing file, add its parent to `Watcher`.
    // If there is no parent, add `path` to `orphans`.
    let mut orphans = Vec::new();
    for path in files.keys() {
        if path.is_tailable() {
            watcher.watch_with_parent(path)?;
        } else if settings.follow.is_some() && settings.retry {
            if path.is_orphan() {
                orphans.push(path.to_owned());
            } else {
                watcher.watch_with_parent(path.parent().unwrap())?;
            }
        } else {
            // TODO: [2022-05; jhscheer] do we need to handle this case?
            unimplemented!();
        }
    }

    // TODO: [2021-10; jhscheer]
    let mut _event_counter = 0;
    let mut _timeout_counter = 0;

    // main follow loop
    loop {
        let mut _read_some = false;

        // If `--pid=p`, tail checks whether process p
        // is alive at least every `--sleep-interval=N` seconds
        if settings.follow.is_some() && settings.pid != 0 && process.is_dead() {
            // p is dead, tail will also terminate
            break;
        }

        // For `-F` we need to poll if an orphan path becomes available during runtime.
        // If a path becomes an orphan during runtime, it will be added to orphans.
        // To be able to differentiate between the cases of test_retry8 and test_retry9,
        // here paths will not be removed from orphans if the path becomes available.
        if settings.follow_name_retry() {
            for new_path in &orphans {
                if new_path.exists() {
                    let pd = files.get(new_path);
                    let md = new_path.metadata().unwrap();
                    if md.is_tailable() && pd.reader.is_none() {
                        show_error!(
                            "{} has appeared;  following new file",
                            pd.display_name.quote()
                        );
                        files.update_metadata(new_path, Some(md));
                        files.update_reader(new_path)?;
                        _read_some = files.tail_file(new_path, settings.verbose)?;
                        watcher.watch_with_parent(new_path)?;
                    }
                }
            }
        }

        // With  -f, sleep for approximately N seconds (default 1.0) between iterations;
        // We wake up if Notify sends an Event or if we wait more than `sleep_sec`.
        let rx_result = rx.recv_timeout(settings.sleep_sec);
        if rx_result.is_ok() {
            _event_counter += 1;
            _timeout_counter = 0;
        }

        let mut paths = vec![]; // Paths worth checking for new content to print
        match rx_result {
            Ok(Ok(event)) => {
                if let Some(event_path) = event.paths.first() {
                    if files.contains_key(event_path) {
                        // Handle Event if it is about a path that we are monitoring
                        paths = handle_event(&event, files, settings, &mut watcher, &mut orphans)?;
                    }
                }
            }
            Ok(Err(notify::Error {
                kind: notify::ErrorKind::Io(ref e),
                paths,
            })) if e.kind() == std::io::ErrorKind::NotFound => {
                if let Some(event_path) = paths.first() {
                    if files.contains_key(event_path) {
                        let _ = watcher.unwatch(event_path);
                    }
                }
            }
            Ok(Err(notify::Error {
                kind: notify::ErrorKind::MaxFilesWatch,
                ..
            })) => {
                return Err(USimpleError::new(
                    1,
                    format!("{} resources exhausted", text::BACKEND),
                ))
            }
            Ok(Err(e)) => return Err(USimpleError::new(1, format!("NotifyError: {}", e))),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                _timeout_counter += 1;
            }
            Err(e) => return Err(USimpleError::new(1, format!("RecvTimeoutError: {}", e))),
        }

        if settings.use_polling && settings.follow.is_some() {
            // Consider all files to potentially have new content.
            // This is a workaround because `Notify::PollWatcher`
            // does not recognize the "renaming" of files.
            paths = files.keys().cloned().collect::<Vec<_>>();
        }

        // main print loop
        for path in &paths {
            _read_some = files.tail_file(path, settings.verbose)?;
        }

        if _timeout_counter == settings.max_unchanged_stats {
            /*
            TODO: [2021-10; jhscheer] implement timeout_counter for each file.
            ‘--max-unchanged-stats=n’
            When tailing a file by name, if there have been n (default n=5) consecutive iterations
            for which the file has not changed, then open/fstat the file to determine if that file
            name is still associated with the same device/inode-number pair as before. When
            following a log file that is rotated, this is approximately the number of seconds
            between when tail prints the last pre-rotation lines and when it prints the lines that
            have accumulated in the new log file. This option is meaningful only when polling
            (i.e., without inotify) and when following by name.
            */
        }
    }
    Ok(())
}

fn handle_event(
    event: &notify::Event,
    files: &mut FileHandling,
    settings: &Settings,
    watcher: &mut Box<dyn Watcher>,
    orphans: &mut Vec<PathBuf>,
) -> UResult<Vec<PathBuf>> {
    use notify::event::*;

    let event_path = event.paths.first().unwrap();
    let display_name = files.get_display_name(event_path);
    let mut paths: Vec<PathBuf> = vec![];

    match event.kind {
        EventKind::Modify(ModifyKind::Metadata(MetadataKind::Any | MetadataKind::WriteTime))
            // | EventKind::Access(AccessKind::Close(AccessMode::Write))
            | EventKind::Create(CreateKind::File | CreateKind::Folder | CreateKind::Any)
            | EventKind::Modify(ModifyKind::Data(DataChange::Any))
            | EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                if let Ok(new_md) = event_path.metadata() {
                    let is_tailable = new_md.is_tailable();
                    let pd = files.get(event_path);
                    if let Some(old_md) = &pd.metadata {
                        if is_tailable {
                            // We resume tracking from the start of the file,
                            // assuming it has been truncated to 0. This mimics GNU's `tail`
                            // behavior and is the usual truncation operation for log files.
                            if !old_md.is_tailable() {
                                show_error!( "{} has become accessible", display_name.quote());
                                files.update_reader(event_path)?;
                            } else if pd.reader.is_none() {
                                show_error!( "{} has appeared;  following new file", display_name.quote());
                                files.update_reader(event_path)?;
                            } else if event.kind == EventKind::Modify(ModifyKind::Name(RenameMode::To))
                            || (settings.use_polling
                            && !old_md.file_id_eq(&new_md)) {
                                show_error!( "{} has been replaced;  following new file", display_name.quote());
                                files.update_reader(event_path)?;
                            } else if old_md.got_truncated(&new_md)? {
                                show_error!("{}: file truncated", display_name.display());
                                files.update_reader(event_path)?;
                            }
                            paths.push(event_path.to_owned());
                        } else if !is_tailable && old_md.is_tailable() {
                            if pd.reader.is_some() {
                                files.reset_reader(event_path);
                            } else {
                                show_error!(
                                    "{} has been replaced with an untailable file",
                                    display_name.quote()
                                );
                            }
                        }
                    } else if is_tailable {
                            show_error!( "{} has appeared;  following new file", display_name.quote());
                            files.update_reader(event_path)?;
                            paths.push(event_path.to_owned());
                        } else if settings.retry {
                            if settings.follow_descriptor() {
                                show_error!(
                                    "{} has been replaced with an untailable file; giving up on this name",
                                    display_name.quote()
                                );
                                let _ = watcher.unwatch(event_path);
                                files.remove(event_path);
                                if files.no_files_remaining(settings) {
                                    return Err(USimpleError::new(1, text::NO_FILES_REMAINING));
                                }
                            } else {
                                show_error!(
                                    "{} has been replaced with an untailable file",
                                    display_name.quote()
                                );
                            }
                        }
                    files.update_metadata(event_path, Some(new_md));
                }
            }
        EventKind::Remove(RemoveKind::File | RemoveKind::Any)
            // | EventKind::Modify(ModifyKind::Name(RenameMode::Any))
            | EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                if settings.follow_name() {
                    if settings.retry {
                        if let Some(old_md) = files.get_mut_metadata(event_path) {
                            if old_md.is_tailable() && files.get(event_path).reader.is_some() {
                                show_error!(
                                    "{} {}: {}",
                                    display_name.quote(),
                                    text::BECOME_INACCESSIBLE,
                                    text::NO_SUCH_FILE
                                );
                            }
                        }
                        if event_path.is_orphan() && !orphans.contains(event_path) {
                            show_error!("directory containing watched file was removed");
                            show_error!(
                                "{} cannot be used, reverting to polling",
                                text::BACKEND
                            );
                            orphans.push(event_path.to_owned());
                            let _ = watcher.unwatch(event_path);
                        }
                    } else {
                        show_error!("{}: {}", display_name.display(), text::NO_SUCH_FILE);
                        if !files.files_remaining() && settings.use_polling {
                            // NOTE: GNU's tail exits here for `---disable-inotify`
                            return Err(USimpleError::new(1, text::NO_FILES_REMAINING));
                        }
                    }
                    files.reset_reader(event_path);
                } else if settings.follow_descriptor_retry() {
                    // --retry only effective for the initial open
                    let _ = watcher.unwatch(event_path);
                    files.remove(event_path);
                } else if settings.use_polling && event.kind == EventKind::Remove(RemoveKind::Any) {
                    /*
                    BUG: The watched file was removed. Since we're using Polling, this
                    could be a rename. We can't tell because `notify::PollWatcher` doesn't
                    recognize renames properly.
                    Ideally we want to call seek to offset 0 on the file handle.
                    But because we only have access to `PathData::reader` as `BufRead`,
                    we cannot seek to 0 with `BufReader::seek_relative`.
                    Also because we don't have the new name, we cannot work around this
                    by simply reopening the file.
                    */
                }
            }
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
            /*
            NOTE: For `tail -f a`, keep tracking additions to b after `mv a b`
            (gnu/tests/tail-2/descriptor-vs-rename.sh)
            NOTE: The File/BufReader doesn't need to be updated.
            However, we need to update our `files.map`.
            This can only be done for inotify, because this EventKind does not
            trigger for the PollWatcher.
            BUG: As a result, there's a bug if polling is used:
            $ tail -f file_a ---disable-inotify
            $ mv file_a file_b
            $ echo A >> file_b
            $ echo A >> file_a
            The last append to file_a is printed, however this shouldn't be because
            after the "mv" tail should only follow "file_b".
            TODO: [2022-05; jhscheer] add test for this bug
            */

            if settings.follow_descriptor() {
                let new_path = event.paths.last().unwrap();
                paths.push(new_path.to_owned());
                // Open new file and seek to End:
                let mut file = File::open(&new_path)?;
                file.seek(SeekFrom::End(0))?;
                // Add new reader but keep old display name
                files.insert(
                    new_path,
                    PathData {
                        metadata: file.metadata().ok(),
                        reader: Some(Box::new(BufReader::new(file))),
                        display_name, // mimic GNU's tail and show old name in header
                    },
                    files.get_last().unwrap() == event_path
                );
                // Remove old reader
                files.remove(event_path);
                // Unwatch old path and watch new path
                let _ = watcher.unwatch(event_path);
                watcher.watch_with_parent(new_path)?;
            }
        }
        _ => {}
    }
    Ok(paths)
}

/// Data structure to keep a handle on the BufReader, Metadata
/// and the display_name (header_name) of files that are being followed.
pub struct PathData {
    reader: Option<Box<dyn BufRead>>,
    metadata: Option<Metadata>,
    display_name: PathBuf, // the path as provided by user input, used for headers
}

mod files {
    use super::*;
    use std::collections::hash_map::Keys;

    /// Data structure to keep a handle on files to follow.
    /// `last` always holds the path/key of the last file that was printed from.
    /// The keys of the HashMap can point to an existing file path (normal case),
    /// or stdin ("-"), or to a non existing path (--retry).
    /// For existing files, all keys in the HashMap are absolute Paths.
    pub struct FileHandling {
        map: HashMap<PathBuf, PathData>,
        last: Option<PathBuf>,
    }

    impl FileHandling {
        /// Creates an empty `FileHandling` with the specified capacity
        pub fn with_capacity(n: usize) -> Self {
            Self {
                map: HashMap::with_capacity(n),
                last: None,
            }
        }

        /// Wrapper for HashMap::insert using Path::canonicalize
        pub fn insert(&mut self, k: &Path, v: PathData, update_last: bool) {
            let k = Self::canonicalize_path(k);
            if update_last {
                self.last = Some(k.to_owned());
            }
            let _ = self.map.insert(k, v);
        }

        /// Wrapper for HashMap::remove using Path::canonicalize
        pub fn remove(&mut self, k: &Path) {
            self.map.remove(&Self::canonicalize_path(k)).unwrap();
        }

        /// Wrapper for HashMap::get using Path::canonicalize
        pub fn get(&self, k: &Path) -> &PathData {
            self.map.get(&Self::canonicalize_path(k)).unwrap()
        }

        /// Wrapper for HashMap::get_mut using Path::canonicalize
        pub fn get_mut(&mut self, k: &Path) -> &mut PathData {
            self.map.get_mut(&Self::canonicalize_path(k)).unwrap()
        }

        /// Canonicalize `path` if it is not already an absolute path
        fn canonicalize_path(path: &Path) -> PathBuf {
            if path.is_relative() && !path.is_stdin() {
                if let Ok(p) = path.canonicalize() {
                    return p;
                }
            }
            path.to_owned()
        }

        pub fn get_display_name(&self, path: &Path) -> PathBuf {
            self.get(path).display_name.to_owned()
        }

        pub fn get_mut_metadata(&mut self, path: &Path) -> Option<&Metadata> {
            self.get_mut(path).metadata.as_ref()
        }

        pub fn keys(&self) -> Keys<PathBuf, PathData> {
            self.map.keys()
        }

        pub fn contains_key(&self, k: &Path) -> bool {
            self.map.contains_key(k)
        }

        pub fn get_last(&self) -> Option<&PathBuf> {
            self.last.as_ref()
        }

        /// Return true if there is only stdin remaining
        pub fn only_stdin_remaining(&self) -> bool {
            self.map.len() == 1 && (self.map.contains_key(Path::new(text::DASH)))
        }

        /// Return true if there is at least one "tailable" path (or stdin) remaining
        pub fn files_remaining(&self) -> bool {
            for path in self.map.keys() {
                if path.is_tailable() || path.is_stdin() {
                    return true;
                }
            }
            false
        }

        /// Returns true if there are no files remaining
        pub fn no_files_remaining(&self, settings: &Settings) -> bool {
            self.map.is_empty() || !self.files_remaining() && !settings.retry
        }

        /// Set `reader` to None to indicate that `path` is not an existing file anymore.
        pub fn reset_reader(&mut self, path: &Path) {
            self.get_mut(path).reader = None;
        }

        /// Reopen the file at the monitored `path`
        pub fn update_reader(&mut self, path: &Path) -> UResult<()> {
            /*
            BUG: If it's not necessary to reopen a file, GNU's tail calls seek to offset 0.
            However we can't call seek here because `BufRead` does not implement `Seek`.
            As a workaround we always reopen the file even though this might not always
            be necessary.
            */
            self.get_mut(path)
                .reader
                .replace(Box::new(BufReader::new(File::open(&path)?)));
            Ok(())
        }

        /// Reload metadata from `path`, or `metadata`
        pub fn update_metadata(&mut self, path: &Path, metadata: Option<Metadata>) {
            self.get_mut(path).metadata = if metadata.is_some() {
                metadata
            } else {
                path.metadata().ok()
            };
        }

        /// Read `path` from the current seek position forward
        pub fn read_file(&mut self, path: &Path, buffer: &mut Vec<u8>) -> UResult<bool> {
            let mut read_some = false;
            let pd = self.get_mut(path).reader.as_mut();
            if let Some(reader) = pd {
                loop {
                    match reader.read_until(b'\n', buffer) {
                        Ok(0) => break,
                        Ok(_) => {
                            read_some = true;
                        }
                        Err(err) => return Err(USimpleError::new(1, err.to_string())),
                    }
                }
            }
            Ok(read_some)
        }

        /// Print `buffer` to stdout
        pub fn print_file(&self, buffer: &[u8]) -> UResult<()> {
            let mut stdout = stdout();
            stdout
                .write_all(buffer)
                .map_err_context(|| String::from("write error"))?;
            Ok(())
        }

        /// Read new data from `path` and print it to stdout
        pub fn tail_file(&mut self, path: &Path, verbose: bool) -> UResult<bool> {
            let mut buffer = vec![];
            let read_some = self.read_file(path, &mut buffer)?;
            if read_some {
                if self.needs_header(path, verbose) {
                    self.print_header(path, true);
                }
                self.print_file(&buffer)?;

                self.last.replace(path.to_owned());
                self.update_metadata(path, None);
            }
            Ok(read_some)
        }

        /// Decide if printing `path` needs a header based on when it was last printed
        pub fn needs_header(&self, path: &Path, verbose: bool) -> bool {
            if verbose {
                if let Some(ref last) = self.last {
                    return !last.eq(&path);
                } else {
                    return true;
                }
            }
            false
        }

        /// Print header for `path` to stdout
        pub fn print_header(&self, path: &Path, needs_newline: bool) {
            println!(
                "{}==> {} <==",
                if needs_newline { "\n" } else { "" },
                self.display_name(path)
            );
        }

        /// Wrapper for `PathData::display_name`
        pub fn display_name(&self, path: &Path) -> String {
            if let Some(path) = self.map.get(&Self::canonicalize_path(path)) {
                path.display_name.display().to_string()
            } else {
                path.display().to_string()
            }
        }
    }
}

/// Find the index after the given number of instances of a given byte.
///
/// This function reads through a given reader until `num_delimiters`
/// instances of `delimiter` have been seen, returning the index of
/// the byte immediately following that delimiter. If there are fewer
/// than `num_delimiters` instances of `delimiter`, this returns the
/// total number of bytes read from the `reader` until EOF.
///
/// # Errors
///
/// This function returns an error if there is an error during reading
/// from `reader`.
///
/// # Examples
///
/// Basic usage:
///
/// ```rust,ignore
/// use std::io::Cursor;
///
/// let mut reader = Cursor::new("a\nb\nc\nd\ne\n");
/// let i = forwards_thru_file(&mut reader, 2, b'\n').unwrap();
/// assert_eq!(i, 4);
/// ```
///
/// If `num_delimiters` is zero, then this function always returns
/// zero:
///
/// ```rust,ignore
/// use std::io::Cursor;
///
/// let mut reader = Cursor::new("a\n");
/// let i = forwards_thru_file(&mut reader, 0, b'\n').unwrap();
/// assert_eq!(i, 0);
/// ```
///
/// If there are fewer than `num_delimiters` instances of `delimiter`
/// in the reader, then this function returns the total number of
/// bytes read:
///
/// ```rust,ignore
/// use std::io::Cursor;
///
/// let mut reader = Cursor::new("a\n");
/// let i = forwards_thru_file(&mut reader, 2, b'\n').unwrap();
/// assert_eq!(i, 2);
/// ```
fn forwards_thru_file<R>(
    reader: &mut R,
    num_delimiters: u64,
    delimiter: u8,
) -> std::io::Result<usize>
where
    R: Read,
{
    let mut reader = BufReader::new(reader);

    let mut buf = vec![];
    let mut total = 0;
    for _ in 0..num_delimiters {
        match reader.read_until(delimiter, &mut buf) {
            Ok(0) => {
                return Ok(total);
            }
            Ok(n) => {
                total += n;
                buf.clear();
                continue;
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    Ok(total)
}

/// Iterate over bytes in the file, in reverse, until we find the
/// `num_delimiters` instance of `delimiter`. The `file` is left seek'd to the
/// position just after that delimiter.
fn backwards_thru_file(file: &mut File, num_delimiters: u64, delimiter: u8) {
    // This variable counts the number of delimiters found in the file
    // so far (reading from the end of the file toward the beginning).
    let mut counter = 0;

    for (block_idx, slice) in ReverseChunks::new(file).enumerate() {
        // Iterate over each byte in the slice in reverse order.
        let mut iter = slice.iter().enumerate().rev();

        // Ignore a trailing newline in the last block, if there is one.
        if block_idx == 0 {
            if let Some(c) = slice.last() {
                if *c == delimiter {
                    iter.next();
                }
            }
        }

        // For each byte, increment the count of the number of
        // delimiters found. If we have found more than the specified
        // number of delimiters, terminate the search and seek to the
        // appropriate location in the file.
        for (i, ch) in iter {
            if *ch == delimiter {
                counter += 1;
                if counter >= num_delimiters {
                    // After each iteration of the outer loop, the
                    // cursor in the file is at the *beginning* of the
                    // block, so seeking forward by `i + 1` bytes puts
                    // us right after the found delimiter.
                    file.seek(SeekFrom::Current((i + 1) as i64)).unwrap();
                    return;
                }
            }
        }
    }
}

/// When tail'ing a file, we do not need to read the whole file from start to
/// finish just to find the last n lines or bytes. Instead, we can seek to the
/// end of the file, and then read the file "backwards" in blocks of size
/// `BLOCK_SIZE` until we find the location of the first line/byte. This ends up
/// being a nice performance win for very large files.
fn bounded_tail(file: &mut File, settings: &Settings) {
    // Find the position in the file to start printing from.
    match (&settings.mode, settings.beginning) {
        (FilterMode::Lines(count, delimiter), false) => {
            backwards_thru_file(file, *count, *delimiter);
        }
        (FilterMode::Lines(count, delimiter), true) => {
            let i = forwards_thru_file(file, (*count).max(1) - 1, *delimiter).unwrap();
            file.seek(SeekFrom::Start(i as u64)).unwrap();
        }
        (FilterMode::Bytes(count), false) => {
            file.seek(SeekFrom::End(-(*count as i64))).unwrap();
        }
        (FilterMode::Bytes(count), true) => {
            // GNU `tail` seems to index bytes and lines starting at 1, not
            // at 0. It seems to treat `+0` and `+1` as the same thing.
            file.seek(SeekFrom::Start(((*count).max(1) - 1) as u64))
                .unwrap();
        }
    }

    // Print the target section of the file.
    let stdout = stdout();
    let mut stdout = stdout.lock();
    std::io::copy(file, &mut stdout).unwrap();
}

/// An alternative to [`Iterator::skip`] with u64 instead of usize. This is
/// necessary because the usize limit doesn't make sense when iterating over
/// something that's not in memory. For example, a very large file. This allows
/// us to skip data larger than 4 GiB even on 32-bit platforms.
fn skip_u64(iter: &mut impl Iterator, num: u64) {
    for _ in 0..num {
        if iter.next().is_none() {
            break;
        }
    }
}

/// Collect the last elements of an iterator into a `VecDeque`.
///
/// This function returns a [`VecDeque`] containing either the last
/// `count` elements of `iter`, an [`Iterator`] over [`Result`]
/// instances, or all but the first `count` elements of `iter`. If
/// `beginning` is `true`, then all but the first `count` elements are
/// returned.
///
/// # Panics
///
/// If any element of `iter` is an [`Err`], then this function panics.
fn unbounded_tail_collect<T, E>(
    mut iter: impl Iterator<Item = Result<T, E>>,
    count: u64,
    beginning: bool,
) -> UResult<VecDeque<T>>
where
    E: fmt::Debug,
{
    if beginning {
        // GNU `tail` seems to index bytes and lines starting at 1, not
        // at 0. It seems to treat `+0` and `+1` as the same thing.
        let i = count.max(1) - 1;
        skip_u64(&mut iter, i);
        Ok(iter.map(|r| r.unwrap()).collect())
    } else {
        let count: usize = count
            .try_into()
            .map_err(|_| USimpleError::new(1, "Insufficient addressable memory"))?;
        Ok(RingBuffer::from_iter(iter.map(|r| r.unwrap()), count).data)
    }
}

fn unbounded_tail<T: Read>(reader: &mut BufReader<T>, settings: &Settings) -> UResult<()> {
    // Read through each line/char and store them in a ringbuffer that always
    // contains count lines/chars. When reaching the end of file, output the
    // data in the ringbuf.
    match settings.mode {
        FilterMode::Lines(count, sep) => {
            let mut stdout = stdout();
            for line in unbounded_tail_collect(lines(reader, sep), count, settings.beginning)? {
                stdout
                    .write_all(&line)
                    .map_err_context(|| String::from("IO error"))?;
            }
        }
        FilterMode::Bytes(count) => {
            for byte in unbounded_tail_collect(reader.bytes(), count, settings.beginning)? {
                if let Err(err) = stdout().write(&[byte]) {
                    return Err(USimpleError::new(1, err.to_string()));
                }
            }
        }
    }
    Ok(())
}

fn parse_num(src: &str) -> Result<(u64, bool), ParseSizeError> {
    let mut size_string = src.trim();
    let mut starting_with = false;

    if let Some(c) = size_string.chars().next() {
        if c == '+' || c == '-' {
            // tail: '-' is not documented (8.32 man pages)
            size_string = &size_string[1..];
            if c == '+' {
                starting_with = true;
            }
        }
    } else {
        return Err(ParseSizeError::ParseFailure(src.to_string()));
    }

    parse_size(size_string).map(|n| (n, starting_with))
}

pub fn stdin_is_pipe_or_fifo() -> bool {
    #[cfg(unix)]
    {
        platform::stdin_is_pipe_or_fifo()
    }
    #[cfg(windows)]
    {
        winapi_util::file::typ(winapi_util::HandleRef::stdin())
            .map(|t| t.is_disk() || t.is_pipe())
            .unwrap_or(false)
    }
}

pub fn stdin_is_bad_fd() -> bool {
    #[cfg(unix)]
    {
        platform::stdin_is_bad_fd()
    }
    #[cfg(not(unix))]
    false
}

trait FileExtTail {
    #[allow(clippy::wrong_self_convention)]
    fn is_seekable(&mut self) -> bool;
}

impl FileExtTail for File {
    fn is_seekable(&mut self) -> bool {
        self.seek(SeekFrom::Current(0)).is_ok()
            && self.seek(SeekFrom::End(0)).is_ok()
            && self.seek(SeekFrom::Start(0)).is_ok()
    }
}

trait MetadataExtTail {
    fn is_tailable(&self) -> bool;
    fn got_truncated(
        &self,
        other: &Metadata,
    ) -> Result<bool, Box<(dyn uucore::error::UError + 'static)>>;
    fn get_block_size(&self) -> u64;
    fn file_id_eq(&self, other: &Metadata) -> bool;
}

impl MetadataExtTail for Metadata {
    fn is_tailable(&self) -> bool {
        let ft = self.file_type();
        #[cfg(unix)]
        {
            ft.is_file() || ft.is_char_device() || ft.is_fifo()
        }
        #[cfg(not(unix))]
        {
            ft.is_file()
        }
    }

    /// Return true if the file was modified and is now shorter
    fn got_truncated(
        &self,
        other: &Metadata,
    ) -> Result<bool, Box<(dyn uucore::error::UError + 'static)>> {
        Ok(other.len() < self.len() && other.modified()? != self.modified()?)
    }

    fn get_block_size(&self) -> u64 {
        #[cfg(unix)]
        {
            self.blocks()
        }
        #[cfg(not(unix))]
        {
            self.len()
        }
    }

    fn file_id_eq(&self, _other: &Metadata) -> bool {
        #[cfg(unix)]
        {
            self.ino().eq(&_other.ino())
        }
        #[cfg(windows)]
        {
            // TODO: `file_index` requires unstable library feature `windows_by_handle`
            // use std::os::windows::prelude::*;
            // if let Some(self_id) = self.file_index() {
            //     if let Some(other_id) = other.file_index() {
            //         // TODO: not sure this is the equivalent of comparing inode numbers
            //         return self_id.eq(&other_id);
            //     }
            // }
            false
        }
    }
}

trait PathExtTail {
    fn is_stdin(&self) -> bool;
    fn is_orphan(&self) -> bool;
    fn is_tailable(&self) -> bool;
}

impl PathExtTail for Path {
    fn is_stdin(&self) -> bool {
        self.eq(Self::new(text::DASH))
            || self.eq(Self::new(text::DEV_STDIN))
            || self.eq(Self::new(text::STDIN_HEADER))
    }

    /// Return true if `path` does not have an existing parent directory
    fn is_orphan(&self) -> bool {
        !matches!(self.parent(), Some(parent) if parent.is_dir())
    }

    /// Return true if `path` is is a file type that can be tailed
    fn is_tailable(&self) -> bool {
        self.is_file() || self.exists() && self.metadata().unwrap().is_tailable()
    }
}

trait WatcherExtTail {
    fn watch_with_parent(&mut self, path: &Path) -> UResult<()>;
}

impl WatcherExtTail for dyn Watcher {
    /// Wrapper for `notify::Watcher::watch` to also add the parent directory of `path` if necessary.
    fn watch_with_parent(&mut self, path: &Path) -> UResult<()> {
        let mut path = path.to_owned();
        #[cfg(target_os = "linux")]
        if path.is_file() {
            /*
            NOTE: Using the parent directory instead of the file is a workaround.
            This workaround follows the recommendation of the notify crate authors:
            > On some platforms, if the `path` is renamed or removed while being watched, behavior may
            > be unexpected. See discussions in [#165] and [#166]. If less surprising behavior is wanted
            > one may non-recursively watch the _parent_ directory as well and manage related events.
            NOTE: Adding both: file and parent results in duplicate/wrong events.
            Tested for notify::InotifyWatcher and for notify::PollWatcher.
            */
            if let Some(parent) = path.parent() {
                if parent.is_dir() {
                    path = parent.to_owned();
                } else {
                    path = PathBuf::from(".");
                }
            } else {
                // TODO: [2021-10; jhscheer] add test for this - "cannot watch parent directory"
                return Err(USimpleError::new(
                    1,
                    format!("cannot watch parent directory of {}", path.display()),
                ));
            };
        }
        if path.is_relative() {
            path = path.canonicalize()?;
        }
        // TODO: [2022-05; jhscheer] "gnu/tests/tail-2/inotify-rotate-resource.sh" is looking
        // for syscalls: 2x "inotify_add_watch" ("filename" and ".") and 1x "inotify_rm_watch"
        self.watch(&path, RecursiveMode::NonRecursive).unwrap();
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::forwards_thru_file;
    use std::io::Cursor;

    #[test]
    fn test_forwards_thru_file_zero() {
        let mut reader = Cursor::new("a\n");
        let i = forwards_thru_file(&mut reader, 0, b'\n').unwrap();
        assert_eq!(i, 0);
    }

    #[test]
    fn test_forwards_thru_file_basic() {
        //                   01 23 45 67 89
        let mut reader = Cursor::new("a\nb\nc\nd\ne\n");
        let i = forwards_thru_file(&mut reader, 2, b'\n').unwrap();
        assert_eq!(i, 4);
    }

    #[test]
    fn test_forwards_thru_file_past_end() {
        let mut reader = Cursor::new("x\n");
        let i = forwards_thru_file(&mut reader, 2, b'\n').unwrap();
        assert_eq!(i, 2);
    }
}
