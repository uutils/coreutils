//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Aleksander Bielawski <pabzdzdzwiagief@gmail.com>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

extern crate getopts;

extern crate uucore;

use std::fs::OpenOptions;
use std::io::{copy, sink, stdin, stdout, Error, ErrorKind, Read, Result, Write};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use uucore::libc;

static NAME: &str = "tee";
static VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args.collect_str();

    match options(&args).and_then(exec) {
        Ok(_) => 0,
        Err(_) => 1,
    }
}

#[allow(dead_code)]
struct Options {
    program: String,
    append: bool,
    ignore_interrupts: bool,
    print_and_exit: Option<String>,
    files: Vec<String>,
}

fn options(args: &[String]) -> Result<Options> {
    let mut opts = getopts::Options::new();

    opts.optflag("a", "append", "append to the given FILEs, do not overwrite");
    opts.optflag(
        "i",
        "ignore-interrupts",
        "ignore interrupt signals (ignored on non-Unix platforms)",
    );
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("V", "version", "output version information and exit");

    opts.parse(&args[1..])
        .map_err(|e| Error::new(ErrorKind::Other, format!("{}", e)))
        .map(|m| {
            let version = format!("{} {}", NAME, VERSION);
            let arguments = "[OPTION]... [FILE]...";
            let brief = "Copy standard input to each FILE, and also to standard output.";
            let comment = "If a FILE is -, it refers to a file named - .";
            let help = format!(
                "{}\n\nUsage:\n  {} {}\n\n{}\n{}",
                version,
                NAME,
                arguments,
                opts.usage(brief),
                comment
            );
            let names: Vec<String> = m.free.clone().into_iter().collect();
            let to_print = if m.opt_present("help") {
                Some(help)
            } else if m.opt_present("version") {
                Some(version)
            } else {
                None
            };
            Options {
                program: NAME.to_owned(),
                append: m.opt_present("append"),
                ignore_interrupts: m.opt_present("ignore-interrupts"),
                print_and_exit: to_print,
                files: names,
            }
        })
        .map_err(|message| warn(format!("{}", message).as_ref()))
}

fn exec(options: Options) -> Result<()> {
    match options.print_and_exit {
        Some(text) => {
            println!("{}", text);
            Ok(())
        }
        None => tee(options),
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
    eprintln!("{}: {}", NAME, message);
    Error::new(ErrorKind::Other, format!("{}: {}", NAME, message))
}
