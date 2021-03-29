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
use retain_mut::RetainMut;

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
    let mut writers: Vec<NamedWriter> = options
        .files
        .clone()
        .into_iter()
        .map(|file|
            NamedWriter {
                name: file.clone(),
                inner: open(file, options.append),
            }
        )
        .collect();


    writers.insert(0, NamedWriter {
        name: "'standard output'".to_owned(),
        inner: Box::new(stdout()),
    });

    let mut output = MultiWriter::new(writers);
    let input = &mut NamedReader {
        inner: Box::new(stdin()) as Box<dyn Read>,
    };

    // TODO: replaced generic 'copy' call to be able to stop copying
    // if all outputs are closed (due to errors)
    if copy(input, &mut output).is_err() || output.flush().is_err() ||
            output.error_occured() {
        Err(Error::new(ErrorKind::Other, ""))
    } else {
        Ok(())
    }
}

fn open(name: String, append: bool) -> Box<dyn Write> {
    let path = PathBuf::from(name.clone());
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
    Box::new(NamedWriter { inner, name }) as Box<dyn Write>
}

struct MultiWriter {
    writers: Vec<NamedWriter>,
    initial_len: usize,
}

impl MultiWriter {
    fn new (writers: Vec<NamedWriter>) -> Self {
        Self {
            initial_len: writers.len(),
            writers,
        }
    }
    fn error_occured(&self) -> bool {
        self.writers.len() != self.initial_len
    }
}

impl Write for MultiWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.writers.retain_mut(|writer| {
            let result = writer.write_all(buf);
            match result {
                Err(f) => {
                    show_info!("{}: {}", writer.name, f.to_string());
                    false
                }
                _ => true
            }
        });
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        self.writers.retain_mut(|writer| {
            let result = writer.flush();
            match result {
                Err(f) => {
                    show_info!("{}: {}", writer.name, f.to_string());
                    false
                }
                _ => true
            }
        });
        Ok(())
    }
}

struct NamedWriter {
    inner: Box<dyn Write>,
    pub name: String,
}

impl Write for NamedWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}

struct NamedReader {
    inner: Box<dyn Read>,
}

impl Read for NamedReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self.inner.read(buf) {
            Err(f) => {
                show_info!("{}: {}", Path::new("stdin").display(), f.to_string());
                Err(f)
            }
            okay => okay,
        }
    }
}
