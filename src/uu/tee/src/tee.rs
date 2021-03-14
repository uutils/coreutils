//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Aleksander Bielawski <pabzdzdzwiagief@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

#[macro_use]
extern crate uucore;

use clap::{App, Arg};
use std::fs::OpenOptions;
use std::io::{copy, sink, stdin, stdout, Error, ErrorKind, Read, Result, Write};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use uucore::libc;

static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str = "Copy standard input to each FILE, and also to standard output.";

mod options {
    pub const APPEND: &str = "append";
    pub const IGNORE_INTERRUPTS: &str = "ignore-interrupts";
    pub const FILE: &str = "file";
}

#[allow(dead_code)]
struct Options {
    append: bool,
    ignore_interrupts: bool,
    files: Vec<String>,
}

fn get_usage() -> String {
    format!("{0} [OPTION]... [FILE]...", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .after_help("If a FILE is -, it refers to a file named - .")
        .arg(
            Arg::with_name(options::APPEND)
                .long(options::APPEND)
                .short("a")
                .help("append to the given FILEs, do not overwrite"),
        )
        .arg(
            Arg::with_name(options::IGNORE_INTERRUPTS)
                .long(options::IGNORE_INTERRUPTS)
                .short("i")
                .help("ignore interrupt signals (ignored on non-Unix platforms)"),
        )
        .arg(Arg::with_name(options::FILE).multiple(true))
        .get_matches_from(args);

    let options = Options {
        append: matches.is_present(options::APPEND),
        ignore_interrupts: matches.is_present(options::IGNORE_INTERRUPTS),
        files: matches
            .values_of(options::FILE)
            .map(|v| v.map(ToString::to_string).collect())
            .unwrap_or_default(),
    };

    match tee(options) {
        Ok(_) => 0,
        Err(_) => 1,
    }
}

#[cfg(unix)]
fn ignore_interrupts() -> Result<()> {
    let ret = unsafe { libc::signal(libc::SIGINT, libc::SIG_IGN) };
    if ret == libc::SIG_ERR {
        return Err(Error::new(ErrorKind::Other, ""));
    }
    Ok(())
}

#[cfg(not(unix))]
fn ignore_interrupts() -> Result<()> {
    // Do nothing.
    Ok(())
}

fn tee(options: Options) -> Result<()> {
    if options.ignore_interrupts {
        ignore_interrupts()?
    }
    let mut writers: Vec<Box<dyn Write>> = options
        .files
        .clone()
        .into_iter()
        .map(|file| open(file, options.append))
        .collect();
    writers.push(Box::new(stdout()));
    let output = &mut MultiWriter { writers };
    let input = &mut NamedReader {
        inner: Box::new(stdin()) as Box<dyn Read>,
    };
    if copy(input, output).is_err() || output.flush().is_err() {
        Err(Error::new(ErrorKind::Other, ""))
    } else {
        Ok(())
    }
}

fn open(name: String, append: bool) -> Box<dyn Write> {
    let path = PathBuf::from(name);
    let inner: Box<dyn Write> = {
        let mut options = OpenOptions::new();
        let mode = if append {
            options.append(true)
        } else {
            options.truncate(true)
        };
        match mode.write(true).create(true).open(path.as_path()) {
            Ok(file) => Box::new(file),
            Err(_) => Box::new(sink()),
        }
    };
    Box::new(NamedWriter { inner, path }) as Box<dyn Write>
}

struct MultiWriter {
    writers: Vec<Box<dyn Write>>,
}

impl Write for MultiWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        for writer in &mut self.writers {
            writer.write_all(buf)?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        for writer in &mut self.writers {
            writer.flush()?;
        }
        Ok(())
    }
}

struct NamedWriter {
    inner: Box<dyn Write>,
    path: PathBuf,
}

impl Write for NamedWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        match self.inner.write(buf) {
            Err(f) => {
                self.inner = Box::new(sink()) as Box<dyn Write>;
                warn(format!("{}: {}", self.path.display(), f.to_string()).as_ref());
                Err(f)
            }
            okay => okay,
        }
    }

    fn flush(&mut self) -> Result<()> {
        match self.inner.flush() {
            Err(f) => {
                self.inner = Box::new(sink()) as Box<dyn Write>;
                warn(format!("{}: {}", self.path.display(), f.to_string()).as_ref());
                Err(f)
            }
            okay => okay,
        }
    }
}

struct NamedReader {
    inner: Box<dyn Read>,
}

impl Read for NamedReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self.inner.read(buf) {
            Err(f) => {
                warn(format!("{}: {}", Path::new("stdin").display(), f.to_string()).as_ref());
                Err(f)
            }
            okay => okay,
        }
    }
}

fn warn(message: &str) -> Error {
    show_warning!("{}", message);
    Error::new(ErrorKind::Other, format!("{}: {}", executable!(), message))
}
