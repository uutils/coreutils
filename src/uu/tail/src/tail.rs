//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Morten Olsen Lysgaard <morten@lysgaard.no>
//  * (c) Alexander Batischev <eual.jp@gmail.com>
//  * (c) Thomas Queiroz <thomasqueirozb@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

// spell-checker:ignore (ToDO) seekable seek'd tail'ing ringbuffer ringbuf

#[macro_use]
extern crate clap;

#[macro_use]
extern crate uucore;

mod chunks;
mod platform;
use chunks::ReverseChunks;

use clap::{App, Arg};
use std::collections::VecDeque;
use std::fmt;
use std::fs::{File, Metadata};
use std::io::{stdin, stdout, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::Duration;
use uucore::display::Quotable;
use uucore::parse_size::{parse_size, ParseSizeError};
use uucore::ringbuffer::RingBuffer;

#[cfg(unix)]
use crate::platform::stdin_is_pipe_or_fifo;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[cfg(target_os = "linux")]
pub static BACKEND: &str = "Disable 'inotify' support and use polling instead";
#[cfg(target_os = "macos")]
pub static BACKEND: &str = "Disable 'FSEvents' support and use polling instead";
#[cfg(any(
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "dragonflybsd",
    target_os = "netbsd",
))]
pub static BACKEND: &str = "Disable 'kqueue' support and use polling instead";
#[cfg(target_os = "windows")]
pub static BACKEND: &str = "Disable 'ReadDirectoryChanges' support and use polling instead";

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
    pub static MAX_UNCHANGED_STATS: &str = "max-unchanged-stats";
    pub static ARG_FILES: &str = "files";
}

enum FilterMode {
    Bytes(usize),
    Lines(usize, u8), // (number of lines, delimiter)
}

#[derive(Debug, PartialEq)]
enum FollowMode {
    Descriptor,
    Name,
}

struct Settings {
    mode: FilterMode,
    sleep_sec: Duration,
    max_unchanged_stats: usize,
    beginning: bool,
    follow: Option<FollowMode>,
    force_polling: bool,
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
            force_polling: false,
            pid: 0,
        }
    }
}

#[allow(clippy::cognitive_complexity)]
pub fn uumain(args: impl uucore::Args) -> i32 {
    let mut settings: Settings = Default::default();
    let mut return_code = 0;
    let app = uu_app();

    let matches = app.get_matches_from(args);

    settings.follow = if matches.occurrences_of(options::FOLLOW) == 0 {
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
        settings.max_unchanged_stats = match s.parse::<usize>() {
            Ok(s) => s,
            Err(_) => crash!(
                1,
                "invalid maximum number of unchanged stats between opens: {}",
                s.quote()
            ),
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

    settings.force_polling = matches.is_present(options::DISABLE_INOTIFY_TERM);

    if matches.is_present(options::ZERO_TERM) {
        if let FilterMode::Lines(count, _) = settings.mode {
            settings.mode = FilterMode::Lines(count, 0);
        }
    }

    let verbose = matches.is_present(options::verbosity::VERBOSE);
    let quiet = matches.is_present(options::verbosity::QUIET);

    let paths: Vec<PathBuf> = matches
        .values_of(options::ARG_FILES)
        .map(|v| v.map(PathBuf::from).collect())
        .unwrap_or_else(|| vec![PathBuf::from("-")]);

    let mut files_count = paths.len();
    let mut first_header = true;
    let mut readers: Vec<(Box<dyn BufRead>, &PathBuf, Option<Metadata>)> = Vec::new();

    #[cfg(unix)]
    let stdin_string = PathBuf::from("standard input");

    for filename in &paths {
        let use_stdin = filename.to_str() == Some("-");

        if use_stdin {
            if verbose && !quiet {
                println!("==> standard input <==");
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

                if settings.follow.is_some() && !stdin_is_pipe_or_fifo() {
                    readers.push((Box::new(reader), &stdin_string, None));
                }
            }
        } else {
            let path = Path::new(filename);
            if path.is_dir() {
                continue;
            }
            if !path.exists() {
                show_error!("cannot open {}: No such file or directory", path.quote());
                files_count -= 1;
                return_code = 1;
                continue;
            }
            if (files_count > 1 || verbose) && !quiet {
                if !first_header {
                    println!();
                }
                println!("==> {} <==", filename.display());
            }
            first_header = false;
            let mut file = File::open(&path).unwrap();
            let md = file.metadata().ok();
            if is_seekable(&mut file) && get_block_size(md.as_ref().unwrap()) > 0 {
                bounded_tail(&mut file, &settings);
                if settings.follow.is_some() {
                    let reader = BufReader::new(file);
                    readers.push((Box::new(reader), filename, md));
                }
            } else {
                let mut reader = BufReader::new(file);
                unbounded_tail(&mut reader, &settings);
                if settings.follow.is_some() {
                    readers.push((Box::new(reader), filename, md));
                }
            }
        }
    }

    if settings.follow.is_some() {
        follow(&mut readers, &settings);
    }

    return_code
}

pub fn uu_app() -> App<'static, 'static> {
    App::new(uucore::util_name())
        .version(crate_version!())
        .about("output the last part of files")
        // TODO: add usage
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
                .help("with -f, terminate after process ID, PID dies"),
        )
        .arg(
            Arg::with_name(options::verbosity::QUIET)
                .short("q")
                .long(options::verbosity::QUIET)
                .visible_alias("silent")
                .overrides_with_all(&[options::verbosity::QUIET, options::verbosity::VERBOSE])
                .help("never output headers giving file names"),
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
                    "Reopen a FILE which has not changed size after N (default 5) iterations to \
                    see if it has been unlinked or renamed (this is the usual case of rotated log \
                        files); This option is meaningful only when polling \
                    (i.e., with --disable-inotify) and when --follow=name.",
                ),
        )
        .arg(
            Arg::with_name(options::verbosity::VERBOSE)
                .short("v")
                .long(options::verbosity::VERBOSE)
                .overrides_with_all(&[options::verbosity::QUIET, options::verbosity::VERBOSE])
                .help("always output headers giving file names"),
        )
        .arg(
            Arg::with_name(options::ZERO_TERM)
                .short("z")
                .long(options::ZERO_TERM)
                .help("Line delimiter is NUL, not newline"),
        )
        .arg(
            Arg::with_name(options::DISABLE_INOTIFY_TERM)
                .long(options::DISABLE_INOTIFY_TERM)
                .help(BACKEND),
        )
        .arg(
            Arg::with_name(options::ARG_FILES)
                .multiple(true)
                .takes_value(true)
                .min_values(1),
        )
}

fn follow(readers: &mut Vec<(Box<dyn BufRead>, &PathBuf, Option<Metadata>)>, settings: &Settings) {
    assert!(settings.follow.is_some());
    if readers.is_empty() {
        return;
    }

    let mut process = platform::ProcessChecker::new(settings.pid);

    use notify::{RecursiveMode, Watcher};
    use std::sync::{Arc, Mutex};
    let (tx, rx) = channel();

    let mut watcher: Box<dyn Watcher>;
    if settings.force_polling {
        // Polling based Watcher implementation
        watcher = Box::new(
            notify::PollWatcher::with_delay(Arc::new(Mutex::new(tx)), settings.sleep_sec).unwrap(),
        );
    } else {
        // Watcher is implemented per platform using the best implementation available on that
        // platform. In addition to such event driven implementations, a polling implementation
        // is also provided that should work on any platform.
        // Linux / Android: inotify
        // macOS: FSEvents
        // Windows: ReadDirectoryChangesW
        // FreeBSD / NetBSD / OpenBSD / DragonflyBSD: kqueue
        // Fallback: polling (default delay is 30 seconds!)
        watcher = Box::new(notify::RecommendedWatcher::new(tx).unwrap());
    };

    for (_, path, _) in readers.iter() {
        // NOTE: Using the parent directory here instead of the file is a workaround.
        // On Linux (other OSs not tested yet) the watcher can crash for rename/delete/move
        // operations if a file is watched directly.
        // This is the recommendation of the notify crate authors:
        // > On some platforms, if the `path` is renamed or removed while being watched, behaviour may
        // > be unexpected. See discussions in [#165] and [#166]. If less surprising behaviour is wanted
        // > one may non-recursively watch the _parent_ directory as well and manage related events.
        let parent = path.parent().unwrap(); // This should never be `None` if `path.is_file()`
        let path = if parent.is_dir() {
            parent
        } else {
            Path::new(".")
        };
        watcher.watch(path, RecursiveMode::NonRecursive).unwrap();
    }

    let mut read_some;
    let last = readers.len() - 1;

    loop {
        read_some = false;
        match rx.recv() {
            Ok(Ok(event)) => {
                // println!("\n{:?}\n", event);
                if settings.follow == Some(FollowMode::Name) {
                    handle_event(event, readers, settings, last);
                }
            }
            Err(e) => eprintln!("{:?}", e),
            _ => eprintln!("UnknownError"),
        }

        for reader_i in readers.iter_mut().enumerate() {
            read_some = print_file(reader_i, last);
        }

        if !read_some && settings.pid != 0 && process.is_dead() {
            // pid is dead
            break;
        }

        // TODO:
        // Implement `--max-unchanged-stats`, however right now we use the `PollWatcher` from the
        // notify crate if `--disable-inotify` is selected. This means we cannot do any thing
        // useful with `--max-unchanged-stats` here.
    }
}

fn handle_event(
    event: notify::Event,
    readers: &mut Vec<(Box<dyn BufRead>, &PathBuf, Option<Metadata>)>,
    settings: &Settings,
    last: usize,
) -> bool {
    let mut read_some = false;
    use notify::event::*;
    for (i, (reader, path, metadata)) in readers.iter_mut().enumerate() {
        if let Some(event_path) = event.paths.first() {
            if path.ends_with(
                event_path
                    .file_name()
                    .unwrap_or_else(|| std::ffi::OsStr::new("")),
            ) {
                match event.kind {
                    // notify::EventKind::Any => {}
                    EventKind::Access(AccessKind::Close(AccessMode::Write))
                    | EventKind::Modify(ModifyKind::Data(DataChange::Any)) => {
                        // This triggers for e.g.:
                        // head log.dat > log.dat
                        if let Ok(new_md) = path.metadata() {
                            if let Some(old_md) = metadata {
                                if new_md.len() < old_md.len() {
                                    show_error!("{}: file truncated", path.display());
                                    // Update Metadata, open file again and print from beginning.
                                    let _ = std::mem::replace(metadata, Some(new_md));
                                    let new_reader = BufReader::new(File::open(&path).unwrap());
                                    // let _ = new_reader.seek(SeekFrom::End(0));
                                    let _ = std::mem::replace(reader, Box::new(new_reader));
                                    read_some = print_file((i, &mut (reader, path, None)), last);
                                }
                            }
                        }
                    }
                    EventKind::Create(CreateKind::File)
                    | EventKind::Create(CreateKind::Any)
                    | EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                        // This triggers for e.g.:
                        // Create: cp log.bak log.dat
                        // Rename: mv log.bak log.dat

                        let msg = if settings.force_polling {
                            format!("{} has been replaced", path.quote())
                        } else {
                            format!("{} has appeared", path.quote())
                        };
                        show_error!("{};  following new file", msg);
                        // Since Files are automatically closed when they go out of
                        // scope, we resume tracking from the start of the file,
                        // assuming it has been truncated to 0, which is the usual
                        // truncation operation for log files.

                        // Open file again and then print it from the beginning.
                        let new_reader = BufReader::new(File::open(&path).unwrap());
                        let _ = std::mem::replace(reader, Box::new(new_reader));
                        read_some = print_file((i, &mut (reader, path, None)), last);
                    }
                    // EventKind::Modify(ModifyKind::Metadata(_)) => {}
                    // EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {}
                    // EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {}
                    EventKind::Remove(RemoveKind::File)
                    | EventKind::Remove(RemoveKind::Any)
                    | EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                        // This triggers for e.g.:
                        // Create: cp log.dat log.bak
                        // Rename: mv log.dat log.bak
                        if !settings.force_polling {
                            show_error!("{}: No such file or directory", path.display());
                        }
                    }
                    // notify::EventKind::Other => {}
                    _ => {} // println!("{:?}", event.kind),
                }
            }
        }
    }
    read_some
}

// Print all new content since the last pass.
// This prints from the current seek position forward.
// `last` determines if a header needs to be printed.
fn print_file<T: BufRead>(
    reader_i: (usize, &mut (T, &PathBuf, Option<Metadata>)),
    mut last: usize,
) -> bool {
    let mut read_some = false;
    let (i, (reader, filename, _)) = reader_i;
    loop {
        let mut datum = String::new();
        match reader.read_line(&mut datum) {
            Ok(0) => break,
            Ok(_) => {
                read_some = true;
                if i != last {
                    println!("\n==> {} <==", filename.display());
                    last = i;
                }
                print!("{}", datum);
            }
            Err(err) => panic!("{}", err),
        }
    }
    read_some
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
