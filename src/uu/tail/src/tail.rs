//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Morten Olsen Lysgaard <morten@lysgaard.no>
//  * (c) Alexander Batischev <eual.jp@gmail.com>
//  * (c) Thomas Queiroz <thomasqueirozb@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) seekable seek'd tail'ing ringbuffer ringbuf unwatch
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
mod platform;
use chunks::ReverseChunks;

use clap::{App, Arg};
use notify::{RecommendedWatcher, RecursiveMode, Watcher, WatcherKind};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fmt;
use std::fs::{File, Metadata};
use std::io::{stdin, stdout, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, channel};
use std::time::Duration;
use uucore::display::Quotable;
use uucore::parse_size::{parse_size, ParseSizeError};
use uucore::ringbuffer::RingBuffer;

#[cfg(unix)]
use crate::platform::stdin_is_pipe_or_fifo;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

const ABOUT: &str = "\
                     Print the last 10 lines of each FILE to standard output.\n\
                     With more than one FILE, precede each with a header giving the file name.\n\
                     With no FILE, or when FILE is -, read standard input.\n\
                     \n\
                     Mandatory arguments to long flags are mandatory for short flags too.\
                     ";
const USAGE: &str = "tail [FLAG]... [FILE]...";

pub mod text {
    pub static STDIN_STR: &str = "standard input";
    pub static NO_FILES_REMAINING: &str = "no files remaining";
    pub static NO_SUCH_FILE: &str = "No such file or directory";
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
    pub static DISABLE_INOTIFY_TERM: &str = "disable-inotify";
    pub static USE_POLLING: &str = "use-polling";
    pub static RETRY: &str = "retry";
    pub static FOLLOW_RETRY: &str = "F";
    pub static MAX_UNCHANGED_STATS: &str = "max-unchanged-stats";
    pub static ARG_FILES: &str = "files";
}

#[derive(Debug)]
enum FilterMode {
    Bytes(usize),
    Lines(usize, u8), // (number of lines, delimiter)
}

#[derive(Debug, PartialEq)]
enum FollowMode {
    Descriptor,
    Name,
}

#[derive(Debug)]
struct Settings {
    mode: FilterMode,
    sleep_sec: Duration,
    max_unchanged_stats: u32,
    beginning: bool,
    follow: Option<FollowMode>,
    use_polling: bool,
    verbose: bool,
    retry: bool,
    pid: platform::Pid,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            mode: FilterMode::Lines(10, b'\n'),
            sleep_sec: Duration::from_secs_f32(1.0),
            max_unchanged_stats: 5,
            beginning: false,
            follow: None,
            use_polling: false,
            verbose: false,
            retry: false,
            pid: 0,
        }
    }
}

#[allow(clippy::cognitive_complexity)]
pub fn uumain(args: impl uucore::Args) -> i32 {
    let app = uu_app();
    let matches = app.get_matches_from(args);

    let mut settings: Settings = Default::default();
    let mut return_code = 0;

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
            Err(_) => crash!(1, "invalid number of seconds: {}", s.quote()),
        }
    }

    if let Some(s) = matches.value_of(options::MAX_UNCHANGED_STATS) {
        settings.max_unchanged_stats = match s.parse::<u32>() {
            Ok(s) => s,
            Err(_) => {
                // TODO: add test for this
                crash!(
                    1,
                    "invalid maximum number of unchanged stats between opens: {}",
                    s.quote()
                )
            }
        }
    }

    if let Some(pid_str) = matches.value_of(options::PID) {
        if let Ok(pid) = pid_str.parse() {
            settings.pid = pid;
            if pid != 0 {
                if settings.follow.is_none() {
                    show_warning!("PID ignored; --pid=PID is useful only when following");
                }

                if !platform::supports_pid_checks(pid) {
                    show_warning!("--pid=PID is not supported on this system");
                    settings.pid = 0;
                }
            }
        }
    }

    let mode_and_beginning = if let Some(arg) = matches.value_of(options::BYTES) {
        match parse_num(arg) {
            Ok((n, beginning)) => (FilterMode::Bytes(n), beginning),
            Err(e) => crash!(1, "invalid number of bytes: {}", e.to_string()),
        }
    } else if let Some(arg) = matches.value_of(options::LINES) {
        match parse_num(arg) {
            Ok((n, beginning)) => (FilterMode::Lines(n, b'\n'), beginning),
            Err(e) => crash!(1, "invalid number of lines: {}", e.to_string()),
        }
    } else {
        (FilterMode::Lines(10, b'\n'), false)
    };
    settings.mode = mode_and_beginning.0;
    settings.beginning = mode_and_beginning.1;

    settings.use_polling = matches.is_present(options::USE_POLLING);
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

    let mut paths: Vec<PathBuf> = matches
        .values_of(options::ARG_FILES)
        .map(|v| v.map(PathBuf::from).collect())
        .unwrap_or_else(|| vec![PathBuf::from("-")]);

    // Filter out paths depending on `FollowMode`.
    paths.retain(|path| {
        if !path.is_stdin() {
            if !path.is_file() {
                return_code = 1;
                if settings.follow == Some(FollowMode::Descriptor) && settings.retry {
                    show_warning!("--retry only effective for the initial open");
                }
                if path.is_dir() {
                    show_error!("error reading {}: Is a directory", path.quote());
                    if settings.follow.is_some() {
                        let msg = if !settings.retry {
                            "; giving up on this name"
                        } else {
                            ""
                        };
                        show_error!(
                            "{}: cannot follow end of this type of file{}",
                            path.display(),
                            msg
                        );
                    }
                } else {
                    show_error!(
                        "cannot open {} for reading: {}",
                        path.quote(),
                        text::NO_SUCH_FILE
                    );
                }
            }
        } else if settings.follow == Some(FollowMode::Name) {
            // Mimic GNU's tail; Exit immediately even though there might be other valid files.
            crash!(1, "cannot follow '-' by name");
        }
        if settings.follow == Some(FollowMode::Name) && settings.retry {
            true
        } else {
            !path.is_dir() || path.is_stdin()
        }
    });

    settings.verbose = (matches.is_present(options::verbosity::VERBOSE) || paths.len() > 1)
        && !matches.is_present(options::verbosity::QUIET);

    let mut first_header = true;
    let mut files = FileHandling {
        map: HashMap::with_capacity(paths.len()),
        last: None,
    };

    // Do an initial tail print of each path's content.
    // Add `path` to `files` map if `--follow` is selected.
    for path in &paths {
        let md = path.metadata().ok();
        if path.is_stdin() {
            if settings.verbose {
                if !first_header {
                    println!();
                }
                Path::new(text::STDIN_STR).print_header();
            }
            let mut reader = BufReader::new(stdin());
            unbounded_tail(&mut reader, &settings);

            // Don't follow stdin since there are no checks for pipes/FIFOs
            //
            // FIXME windows has GetFileType which can determine if the file is a pipe/FIFO
            // so this check can also be performed

            #[cfg(unix)]
            {
                /*
                POSIX specification regarding tail -f

                If the input file is a regular file or if the file operand specifies a FIFO, do not
                terminate after the last line of the input file has been copied, but read and copy
                further bytes from the input file when they become available. If no file operand is
                specified and standard input is a pipe or FIFO, the -f option shall be ignored. If
                the input file is not a FIFO, pipe, or regular file, it is unspecified whether or
                not the -f option shall be ignored.
                */

                if settings.follow == Some(FollowMode::Descriptor) && !stdin_is_pipe_or_fifo() {
                    // Insert `stdin` into `files.map`.
                    files.map.insert(
                        PathBuf::from(text::STDIN_STR),
                        PathData {
                            reader: Some(Box::new(reader)),
                            metadata: None,
                            display_name: PathBuf::from(text::STDIN_STR),
                        },
                    );
                }
            }
        } else if path.is_file() {
            if settings.verbose {
                if !first_header {
                    println!();
                }
                path.print_header();
            }
            first_header = false;
            let mut file = File::open(&path).unwrap();
            let mut reader;

            if is_seekable(&mut file) && get_block_size(md.as_ref().unwrap()) > 0 {
                bounded_tail(&mut file, &settings);
                reader = BufReader::new(file);
            } else {
                reader = BufReader::new(file);
                unbounded_tail(&mut reader, &settings);
            }
            if settings.follow.is_some() {
                // Insert existing/file `path` into `files.map`.
                files.map.insert(
                    path.canonicalize().unwrap(),
                    PathData {
                        reader: Some(Box::new(reader)),
                        metadata: md,
                        display_name: path.to_owned(),
                    },
                );
                files.last = Some(path.canonicalize().unwrap());
            }
        } else if settings.retry && settings.follow.is_some() {
            // Insert non-is_file() paths into `files.map`.
            let key = if path.is_relative() {
                std::env::current_dir().unwrap().join(path)
            } else {
                path.to_path_buf()
            };
            files.map.insert(
                key.to_path_buf(),
                PathData {
                    reader: None,
                    metadata: md,
                    display_name: path.to_path_buf(),
                },
            );
            files.last = Some(key);
        }
    }

    if settings.follow.is_some() {
        if files.map.is_empty() || !files.files_remaining() && !settings.retry {
            show_error!("{}", text::NO_FILES_REMAINING);
        } else {
            follow(&mut files, &settings);
        }
    }

    return_code
}

pub fn uu_app() -> App<'static, 'static> {
    #[cfg(target_os = "linux")]
    pub static POLLING_HELP: &str = "Disable 'inotify' support and use polling instead";
    #[cfg(all(unix, not(target_os = "linux")))]
    pub static POLLING_HELP: &str = "Disable 'kqueue' support and use polling instead";
    #[cfg(target_os = "windows")]
    pub static POLLING_HELP: &str =
        "Disable 'ReadDirectoryChanges' support and use polling instead";

    App::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .usage(USAGE)
        .arg(
            Arg::with_name(options::BYTES)
                .short("c")
                .long(options::BYTES)
                .takes_value(true)
                .allow_hyphen_values(true)
                .overrides_with_all(&[options::BYTES, options::LINES])
                .help("Number of bytes to print"),
        )
        .arg(
            Arg::with_name(options::FOLLOW)
                .short("f")
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
            Arg::with_name(options::LINES)
                .short("n")
                .long(options::LINES)
                .takes_value(true)
                .allow_hyphen_values(true)
                .overrides_with_all(&[options::BYTES, options::LINES])
                .help("Number of lines to print"),
        )
        .arg(
            Arg::with_name(options::PID)
                .long(options::PID)
                .takes_value(true)
                .help("With -f, terminate after process ID, PID dies"),
        )
        .arg(
            Arg::with_name(options::verbosity::QUIET)
                .short("q")
                .long(options::verbosity::QUIET)
                .visible_alias("silent")
                .overrides_with_all(&[options::verbosity::QUIET, options::verbosity::VERBOSE])
                .help("Never output headers giving file names"),
        )
        .arg(
            Arg::with_name(options::SLEEP_INT)
                .short("s")
                .takes_value(true)
                .long(options::SLEEP_INT)
                .help("Number or seconds to sleep between polling the file when running with -f"),
        )
        .arg(
            Arg::with_name(options::MAX_UNCHANGED_STATS)
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
            Arg::with_name(options::verbosity::VERBOSE)
                .short("v")
                .long(options::verbosity::VERBOSE)
                .overrides_with_all(&[options::verbosity::QUIET, options::verbosity::VERBOSE])
                .help("Always output headers giving file names"),
        )
        .arg(
            Arg::with_name(options::ZERO_TERM)
                .short("z")
                .long(options::ZERO_TERM)
                .help("Line delimiter is NUL, not newline"),
        )
        .arg(
            Arg::with_name(options::USE_POLLING)
                .visible_alias(options::DISABLE_INOTIFY_TERM)
                .alias("dis") // Used by GNU's test suite
                .long(options::USE_POLLING)
                .help(POLLING_HELP),
        )
        .arg(
            Arg::with_name(options::RETRY)
                .long(options::RETRY)
                .help("Keep trying to open a file if it is inaccessible"),
        )
        .arg(
            Arg::with_name(options::FOLLOW_RETRY)
                .short(options::FOLLOW_RETRY)
                .help("Same as --follow=name --retry")
                .overrides_with_all(&[options::RETRY, options::FOLLOW]),
        )
        .arg(
            Arg::with_name(options::ARG_FILES)
                .multiple(true)
                .takes_value(true)
                .min_values(1),
        )
}

fn follow(files: &mut FileHandling, settings: &Settings) {
    let mut process = platform::ProcessChecker::new(settings.pid);

    let (tx, rx) = channel();

    // Watcher is implemented per platform using the best implementation available on that
    // platform. In addition to such event driven implementations, a polling implementation
    // is also provided that should work on any platform.
    // Linux / Android: inotify
    // macOS: FSEvents / kqueue
    // Windows: ReadDirectoryChangesWatcher
    // FreeBSD / NetBSD / OpenBSD / DragonflyBSD: kqueue
    // Fallback: polling (default delay is 30 seconds!)

    // NOTE:
    // We force the use of kqueue with: features=["macos_kqueue"],
    // because macOS only `kqueue` is suitable for our use case since `FSEvents` waits until
    // file close util it delivers a modify event. See:
    // https://github.com/notify-rs/notify/issues/240

    let mut watcher: Box<dyn Watcher> =
        if settings.use_polling || RecommendedWatcher::kind() == WatcherKind::PollWatcher {
            Box::new(notify::PollWatcher::with_delay(tx, settings.sleep_sec).unwrap())
        } else {
            Box::new(notify::RecommendedWatcher::new(tx).unwrap())
        };

    // Iterate user provided `paths`.
    // Add existing files to `Watcher` (InotifyWatcher).
    // If `path` is not an existing file, add its parent to `Watcher`.
    // If there is no parent, add `path` to `orphans`.
    let mut orphans = Vec::with_capacity(files.map.len());
    for path in files.map.keys() {
        if path.is_file() {
            let path = get_path(path, settings);
            watcher
                .watch(&path.canonicalize().unwrap(), RecursiveMode::NonRecursive)
                .unwrap();
        } else if settings.follow.is_some() && settings.retry {
            if path.is_orphan() {
                orphans.push(path.to_path_buf());
                // TODO: add test for this
            } else {
                let parent = path.parent().unwrap();
                watcher
                    .watch(&parent.canonicalize().unwrap(), RecursiveMode::NonRecursive)
                    .unwrap();
            }
        } else {
            unreachable!()
        }
    }

    let mut _event_counter = 0;
    let mut _timeout_counter = 0;

    loop {
        let mut read_some = false;

        // For `-F` we need to poll if an orphan path becomes available during runtime.
        // If a path becomes an orphan during runtime, it will be added to orphans.
        // To be able to differentiate between the cases of test_retry8 and test_retry9,
        // here paths will not be removed from orphans if the path becomes available.
        if settings.retry && settings.follow == Some(FollowMode::Name) {
            for new_path in orphans.iter() {
                if new_path.exists() {
                    let display_name = files.map.get(new_path).unwrap().display_name.to_path_buf();
                    if new_path.is_file() && files.map.get(new_path).unwrap().metadata.is_none() {
                        // TODO: add test for this
                        show_error!("{} has appeared;  following new file", display_name.quote());
                        if let Ok(new_path_canonical) = new_path.canonicalize() {
                            files.update_metadata(&new_path_canonical, None);
                            files.reopen_file(&new_path_canonical).unwrap();
                            read_some = files.print_file(&new_path_canonical, settings);
                            let new_path = get_path(&new_path_canonical, settings);
                            watcher
                                .watch(&new_path, RecursiveMode::NonRecursive)
                                .unwrap();
                        } else {
                            unreachable!()
                        }
                    } else if new_path.is_dir() {
                        // TODO: does is_dir() need handling?
                        todo!();
                    }
                }
            }
        }

        let rx_result = rx.recv_timeout(settings.sleep_sec);
        if rx_result.is_ok() {
            _event_counter += 1;
            _timeout_counter = 0;
        }
        match rx_result {
            Ok(Ok(event)) => {
                // eprintln!("=={:=>3}===========================", _event_counter);
                // dbg!(&event);
                // dbg!(files.map.keys());
                // eprintln!("=={:=>3}===========================", _event_counter);
                handle_event(event, files, settings, &mut watcher, &mut orphans);
            }
            Ok(Err(notify::Error {
                kind: notify::ErrorKind::Io(ref e),
                paths,
            })) if e.kind() == std::io::ErrorKind::NotFound => {
                // dbg!(e, &paths);
                // TODO: is this still needed ?
                if let Some(event_path) = paths.first() {
                    if files.map.contains_key(event_path) {
                        // TODO: handle this case for --follow=name --retry
                        let _ = watcher.unwatch(event_path);
                        // TODO: add test for this
                        show_error!(
                            "{}: {}",
                            files.map.get(event_path).unwrap().display_name.display(),
                            text::NO_SUCH_FILE
                        );
                        if !files.files_remaining() && !settings.retry {
                            // TODO: add test for this
                            crash!(1, "{}", text::NO_FILES_REMAINING);
                        }
                    }
                }
            }
            Ok(Err(notify::Error {
                kind: notify::ErrorKind::MaxFilesWatch,
                ..
            })) => crash!(1, "inotify resources exhausted"), // NOTE: Cannot test this in the CICD.
            Ok(Err(e)) => crash!(1, "{:?}", e),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                _timeout_counter += 1;
            }
            Err(e) => crash!(1, "RecvError: {:?}", e),
        }

        for path in files.map.keys().cloned().collect::<Vec<_>>() {
            read_some = files.print_file(&path, settings);
        }

        if !read_some && settings.pid != 0 && process.is_dead() {
            // pid is dead
            break;
        }

        if _timeout_counter == settings.max_unchanged_stats {
            // TODO: [2021-10; jhscheer] implement timeout_counter for each file.
            // ‘--max-unchanged-stats=n’
            // When tailing a file by name, if there have been n (default n=5) consecutive iterations
            // for which the file has not changed, then open/fstat the file to determine if that file
            // name is still associated with the same device/inode-number pair as before. When
            // following a log file that is rotated, this is approximately the number of seconds
            // between when tail prints the last pre-rotation lines and when it prints the lines that
            // have accumulated in the new log file. This option is meaningful only when polling
            // (i.e., without inotify) and when following by name.
            // TODO: [2021-10; jhscheer] `--sleep-interval=N`: implement: if `--pid=p`,
            // tail checks whether process p is alive at least every N seconds
        }
    }
}

fn handle_event(
    event: notify::Event,
    files: &mut FileHandling,
    settings: &Settings,
    watcher: &mut Box<dyn Watcher>,
    orphans: &mut Vec<PathBuf>,
) {
    use notify::event::*;

    if let Some(event_path) = event.paths.first() {
        if files.map.contains_key(event_path) {
            let display_name = files
                .map
                .get(event_path)
                .unwrap()
                .display_name
                .to_path_buf();
            match event.kind {
                // notify::EventKind::Any => {}
                EventKind::Access(AccessKind::Close(AccessMode::Write))
                | EventKind::Modify(ModifyKind::Metadata(MetadataKind::Any))
                | EventKind::Modify(ModifyKind::Metadata(MetadataKind::WriteTime))
                | EventKind::Modify(ModifyKind::Data(DataChange::Any)) => {
                    if let Ok(new_md) = event_path.metadata() {
                        if let Some(old_md) = &files.map.get(event_path).unwrap().metadata {
                            if new_md.is_file() && !old_md.is_file() {
                                show_error!(
                                    "{} has appeared;  following new file",
                                    display_name.quote()
                                );
                                files.update_metadata(event_path, None);
                                files.reopen_file(event_path).unwrap();
                            } else if !new_md.is_file() && old_md.is_file() {
                                show_error!(
                                    "{} has been replaced with an untailable file",
                                    display_name.quote()
                                );
                                files.map.insert(
                                    event_path.to_path_buf(),
                                    PathData {
                                        reader: None,
                                        metadata: None,
                                        display_name,
                                    },
                                );
                                files.update_metadata(event_path, None);
                            } else if new_md.len() <= old_md.len()
                                && new_md.modified().unwrap() != old_md.modified().unwrap()
                            {
                                // TODO: add test for this
                                show_error!("{}: file truncated", display_name.display());
                                files.update_metadata(event_path, None);
                                files.reopen_file(event_path).unwrap();
                            }
                        }
                    }
                }
                EventKind::Create(CreateKind::File)
                | EventKind::Create(CreateKind::Folder)
                | EventKind::Create(CreateKind::Any)
                | EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                    if event_path.is_file() {
                        if settings.follow.is_some() {
                            // TODO: add test for this
                            let msg = if settings.use_polling && !settings.retry {
                                format!("{} has been replaced", display_name.quote())
                            } else {
                                format!("{} has appeared", display_name.quote())
                            };
                            show_error!("{};  following new file", msg);
                        }

                        // Since Files are automatically closed when they go out of
                        // scope, we resume tracking from the start of the file,
                        // assuming it has been truncated to 0. This mimics GNU's `tail`
                        // behavior and is the usual truncation operation for log files.
                        files.reopen_file(event_path).unwrap();
                        if settings.follow == Some(FollowMode::Name) && settings.retry {
                            // TODO: add test for this
                            // Path has appeared, it's not an orphan any more.
                            orphans.retain(|path| path != event_path);
                        }
                    } else {
                        // If the path pointed to a file and now points to something else:
                        let md = &files.map.get(event_path).unwrap().metadata;
                        if md.is_none() || md.as_ref().unwrap().is_file() {
                            let msg = "has been replaced with an untailable file";
                            if settings.follow == Some(FollowMode::Descriptor) {
                                show_error!(
                                    "{} {}; giving up on this name",
                                    display_name.quote(),
                                    msg
                                );
                                let _ = watcher.unwatch(event_path);
                                files.map.remove(event_path).unwrap();
                                if files.map.is_empty() {
                                    crash!(1, "{}", text::NO_FILES_REMAINING);
                                }
                            } else if settings.follow == Some(FollowMode::Name) {
                                // TODO: add test for this
                                files.update_metadata(event_path, None);
                                show_error!("{} {}", display_name.quote(), msg);
                            }
                        }
                    }
                }
                // EventKind::Modify(ModifyKind::Metadata(_)) => {}
                // | EventKind::Remove(RemoveKind::Folder)
                EventKind::Remove(RemoveKind::File) | EventKind::Remove(RemoveKind::Any) => {
                    if settings.follow == Some(FollowMode::Name) {
                        if settings.retry {
                            if let Some(old_md) = &files.map.get(event_path).unwrap().metadata {
                                if old_md.is_file() {
                                    show_error!(
                                        "{} has become inaccessible: {}",
                                        display_name.quote(),
                                        text::NO_SUCH_FILE
                                    );
                                }
                            }
                            if event_path.is_orphan() {
                                if !orphans.contains(event_path) {
                                    show_error!("directory containing watched file was removed");
                                    show_error!(
                                        "{} cannot be used, reverting to polling",
                                        text::BACKEND
                                    );
                                    orphans.push(event_path.to_path_buf());
                                }
                                let _ = watcher.unwatch(event_path);
                            }
                            // Update `files.map` to indicate that `event_path`
                            // is not an existing file anymore.
                            files.map.insert(
                                event_path.to_path_buf(),
                                PathData {
                                    reader: None,
                                    metadata: None,
                                    display_name,
                                },
                            );
                        } else {
                            show_error!("{}: {}", display_name.display(), text::NO_SUCH_FILE);
                            if !files.files_remaining() {
                                crash!(1, "{}", text::NO_FILES_REMAINING);
                            }
                        }
                    } else if settings.follow == Some(FollowMode::Descriptor) && settings.retry {
                        // --retry only effective for the initial open
                        let _ = watcher.unwatch(event_path);
                        files.map.remove(event_path).unwrap();
                    }
                }
                EventKind::Modify(ModifyKind::Name(RenameMode::Any))
                | EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                    if settings.follow == Some(FollowMode::Name) {
                        show_error!("{}: {}", display_name.display(), text::NO_SUCH_FILE);
                    }
                }
                EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                    // NOTE: For `tail -f a`, keep tracking additions to b after `mv a b`
                    // (gnu/tests/tail-2/descriptor-vs-rename.sh)
                    // NOTE: The File/BufReader doesn't need to be updated.
                    // However, we need to update our `files.map`.
                    // This can only be done for inotify, because this EventKind does not
                    // trigger for the PollWatcher.
                    // BUG: As a result, there's a bug if polling is used:
                    // $ tail -f file_a ---disable-inotify
                    // $ mv file_a file_b
                    // $ echo A >> file_a
                    // The last append to file_a is printed, however this shouldn't be because
                    // after the "mv" tail should only follow "file_b".

                    if settings.follow == Some(FollowMode::Descriptor) {
                        let new_path = event.paths.last().unwrap().canonicalize().unwrap();
                        // Open new file and seek to End:
                        let mut file = File::open(&new_path).unwrap();
                        let _ = file.seek(SeekFrom::End(0));
                        // Add new reader and remove old reader:
                        files.map.insert(
                            new_path.to_owned(),
                            PathData {
                                metadata: file.metadata().ok(),
                                reader: Some(Box::new(BufReader::new(file))),
                                display_name, // mimic GNU's tail and show old name in header
                            },
                        );
                        files.map.remove(event_path).unwrap();
                        if files.last.as_ref().unwrap() == event_path {
                            files.last = Some(new_path.to_owned());
                        }
                        // Unwatch old path and watch new path:
                        let _ = watcher.unwatch(event_path);
                        let new_path = get_path(&new_path, settings);
                        watcher
                            .watch(
                                &new_path.canonicalize().unwrap(),
                                RecursiveMode::NonRecursive,
                            )
                            .unwrap();
                    }
                }
                // notify::EventKind::Other => {}
                _ => {} // println!("{:?}", event.kind),
            }
        }
    }
}

fn get_path(path: &Path, settings: &Settings) -> PathBuf {
    if cfg!(target_os = "linux") || settings.use_polling {
        // NOTE: Using the parent directory here instead of the file is a workaround.
        // On Linux the watcher can crash for rename/delete/move operations if a file is watched directly.
        // This workaround follows the recommendation of the notify crate authors:
        // > On some platforms, if the `path` is renamed or removed while being watched, behavior may
        // > be unexpected. See discussions in [#165] and [#166]. If less surprising behavior is wanted
        // > one may non-recursively watch the _parent_ directory as well and manage related events.
        let parent = path
            .parent()
            .unwrap_or_else(|| crash!(1, "cannot watch parent directory of {}", path.display()));
        // TODO: add test for this - "cannot watch parent directory"
        if parent.is_dir() {
            parent.to_path_buf()
        } else {
            PathBuf::from(".")
        }
    } else {
        path.to_path_buf()
    }
}

/// Data structure to keep a handle on the BufReader, Metadata
/// and the display_name (header_name) of files that are being followed.
struct PathData {
    reader: Option<Box<dyn BufRead>>,
    metadata: Option<Metadata>,
    display_name: PathBuf, // the path the user provided, used for headers
}

/// Data structure to keep a handle on files to follow.
/// `last` always holds the path/key of the last file that was printed from.
/// The keys of the HashMap can point to an existing file path (normal case),
/// or stdin ("-"), or to a non existing path (--retry).
/// With the exception of stdin, all keys in the HashMap are absolute Paths.
struct FileHandling {
    map: HashMap<PathBuf, PathData>,
    last: Option<PathBuf>,
}

impl FileHandling {
    fn files_remaining(&self) -> bool {
        for path in self.map.keys() {
            if path.is_file() {
                return true;
            }
        }
        false
    }

    // TODO: change to update_reader() without error return
    fn reopen_file(&mut self, path: &Path) -> Result<(), Error> {
        assert!(self.map.contains_key(path));
        if let Some(pd) = self.map.get_mut(path) {
            let new_reader = BufReader::new(File::open(&path)?);
            pd.reader = Some(Box::new(new_reader));
            return Ok(());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Entry should have been there, but wasn't!",
        ))
    }

    fn update_metadata(&mut self, path: &Path, md: Option<Metadata>) {
        assert!(self.map.contains_key(path));
        if let Some(pd) = self.map.get_mut(path) {
            if let Some(md) = md {
                pd.metadata = Some(md);
            } else {
                pd.metadata = path.metadata().ok();
            }
        }
    }

    // This prints from the current seek position forward.
    fn print_file(&mut self, path: &Path, settings: &Settings) -> bool {
        assert!(self.map.contains_key(path));
        let mut last_display_name = self
            .map
            .get(self.last.as_ref().unwrap())
            .unwrap()
            .display_name
            .to_path_buf();
        let mut read_some = false;
        let pd = self.map.get_mut(path).unwrap();
        if let Some(reader) = pd.reader.as_mut() {
            loop {
                let mut datum = String::new();
                match reader.read_line(&mut datum) {
                    Ok(0) => break,
                    Ok(_) => {
                        read_some = true;
                        if last_display_name != pd.display_name {
                            self.last = Some(path.to_path_buf());
                            last_display_name = pd.display_name.to_path_buf();
                            if settings.verbose {
                                println!();
                                pd.display_name.print_header();
                            }
                        }
                        print!("{}", datum);
                    }
                    Err(err) => panic!("{}", err),
                }
            }
        } else {
            return read_some;
        }
        if read_some {
            self.update_metadata(path, None);
            // TODO: add test for this
        }
        read_some
    }
}

/// Iterate over bytes in the file, in reverse, until we find the
/// `num_delimiters` instance of `delimiter`. The `file` is left seek'd to the
/// position just after that delimiter.
fn backwards_thru_file(file: &mut File, num_delimiters: usize, delimiter: u8) {
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
    match settings.mode {
        FilterMode::Lines(count, delimiter) => {
            backwards_thru_file(file, count as usize, delimiter);
        }
        FilterMode::Bytes(count) => {
            file.seek(SeekFrom::End(-(count as i64))).unwrap();
        }
    }

    // Print the target section of the file.
    let stdout = stdout();
    let mut stdout = stdout.lock();
    std::io::copy(file, &mut stdout).unwrap();
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
    iter: impl Iterator<Item = Result<T, E>>,
    count: usize,
    beginning: bool,
) -> VecDeque<T>
where
    E: fmt::Debug,
{
    if beginning {
        // GNU `tail` seems to index bytes and lines starting at 1, not
        // at 0. It seems to treat `+0` and `+1` as the same thing.
        let i = count.max(1) - 1;
        iter.skip(i as usize).map(|r| r.unwrap()).collect()
    } else {
        RingBuffer::from_iter(iter.map(|r| r.unwrap()), count as usize).data
    }
}

fn unbounded_tail<T: Read>(reader: &mut BufReader<T>, settings: &Settings) {
    // Read through each line/char and store them in a ringbuffer that always
    // contains count lines/chars. When reaching the end of file, output the
    // data in the ringbuf.
    match settings.mode {
        FilterMode::Lines(count, _) => {
            for line in unbounded_tail_collect(reader.lines(), count, settings.beginning) {
                println!("{}", line);
            }
        }
        FilterMode::Bytes(count) => {
            for byte in unbounded_tail_collect(reader.bytes(), count, settings.beginning) {
                let mut stdout = stdout();
                print_byte(&mut stdout, byte);
            }
        }
    }
}

fn is_seekable<T: Seek>(file: &mut T) -> bool {
    file.seek(SeekFrom::Current(0)).is_ok()
        && file.seek(SeekFrom::End(0)).is_ok()
        && file.seek(SeekFrom::Start(0)).is_ok()
}

#[inline]
fn print_byte<T: Write>(stdout: &mut T, ch: u8) {
    if let Err(err) = stdout.write(&[ch]) {
        crash!(1, "{}", err);
    }
}

fn parse_num(src: &str) -> Result<(usize, bool), ParseSizeError> {
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

fn get_block_size(md: &Metadata) -> u64 {
    #[cfg(unix)]
    {
        md.blocks()
    }
    #[cfg(not(unix))]
    {
        md.len()
    }
}

trait PathExt {
    fn is_stdin(&self) -> bool;
    fn print_header(&self);
    fn is_orphan(&self) -> bool;
}

impl PathExt for Path {
    fn is_stdin(&self) -> bool {
        self.to_str() == Some("-")
    }
    fn print_header(&self) {
        println!("==> {} <==", self.display());
    }
    fn is_orphan(&self) -> bool {
        !matches!(self.parent(), Some(parent) if parent.is_dir())
    }
}
