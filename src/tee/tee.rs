#![crate_name = "tee"]
#![feature(rustc_private)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Aleksander Bielawski <pabzdzdzwiagief@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
#[macro_use] extern crate log;

use std::fs::OpenOptions;
use std::io::{copy, Error, ErrorKind, Read, Result, sink, stdin, stdout, Write};
use std::path::{Path, PathBuf};
use getopts::{getopts, optflag, usage};

static NAME: &'static str = "tee";
static VERSION: &'static str = "1.0.0";

pub fn uumain(args: Vec<String>) -> i32 {
    match options(&args).and_then(exec) {
        Ok(_) => 0,
        Err(_) => 1
    }
}

#[allow(dead_code)]
struct Options {
    program: String,
    append: bool,
    ignore_interrupts: bool,
    print_and_exit: Option<String>,
    files: Vec<String>
}

fn options(args: &Vec<String>) -> Result<Options> {
    let opts = [
        optflag("a", "append", "append to the given FILEs, do not overwrite"),
        optflag("i", "ignore-interrupts", "ignore interrupt signals"),
        optflag("h", "help", "display this help and exit"),
        optflag("V", "version", "output version information and exit"),
    ];

    getopts(&args[1..], &opts).map_err(|e| Error::new(ErrorKind::Other, format!("{}", e))).and_then(|m| {
        let version = format!("{} {}", NAME, VERSION);
        let program = &args[0][..];
        let arguments = "[OPTION]... [FILE]...";
        let brief = "Copy standard input to each FILE, and also to standard output.";
        let comment = "If a FILE is -, copy again to standard output.";
        let help = format!("{}\n\nUsage:\n  {} {}\n\n{}\n{}",
                           version, program, arguments, usage(brief, &opts),
                           comment);
        let mut names: Vec<String> = m.free.clone().into_iter().collect();
        names.push("-".to_string());
        let to_print = if m.opt_present("help") { Some(help) }
                       else if m.opt_present("version") { Some(version) }
                       else { None };
        Ok(Options {
            program: program.to_string(),
            append: m.opt_present("append"),
            ignore_interrupts: m.opt_present("ignore-interrupts"),
            print_and_exit: to_print,
            files: names
        })
    }).map_err(|message| warn(format!("{}", message).as_ref()))
}

fn exec(options: Options) -> Result<()> {
    match options.print_and_exit {
        Some(text) => Ok(println!("{}", text)),
        None => tee(options)
    }
}

fn tee(options: Options) -> Result<()> {
    let writers: Vec<Box<Write>> = options.files.clone().into_iter().map(|file| open(file, options.append)).collect();
    let output = &mut MultiWriter { writers: writers };
    let input = &mut NamedReader { inner: Box::new(stdin()) as Box<Read> };
    if copy(input, output).is_err() || output.flush().is_err() {
        Err(Error::new(ErrorKind::Other, ""))
    } else {
        Ok(())
    }
}

fn open(name: String, append: bool) -> Box<Write> {
    let is_stdout = name == "-";
    let path = PathBuf::from(name);
    let inner: Box<Write> = if is_stdout {
        Box::new(stdout())
    } else {
        let mut options = OpenOptions::new();
        let mode = if append { options.append(true) } else { options.truncate(true) };
        match mode.write(true).create(true).open(path.as_path()) {
            Ok(file) => Box::new(file),
            Err(_) => Box::new(sink())
        }
    };
    Box::new(NamedWriter { inner: inner, path: path }) as Box<Write>
}

struct MultiWriter {
    writers: Vec<Box<Write>>
}

impl Write for MultiWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        for writer in self.writers.iter_mut() {
            try!(writer.write_all(buf));
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        for writer in self.writers.iter_mut() {
            try!(writer.flush());
        }
        Ok(())
    }
}

struct NamedWriter {
    inner: Box<Write>,
    path: PathBuf
}

impl Write for NamedWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        match self.inner.write(buf) {
            Err(f) => {
                self.inner = Box::new(sink()) as Box<Write>;
                warn(format!("{}: {}", self.path.display(), f.to_string()).as_ref());
                Err(f)
            }
            okay => okay
        }
    }

    fn flush(&mut self) -> Result<()> {
        match self.inner.flush() {
            Err(f) => {
                self.inner = Box::new(sink()) as Box<Write>;
                warn(format!("{}: {}", self.path.display(), f.to_string()).as_ref());
                Err(f)
            }
            okay => okay
        }
    }
}

struct NamedReader {
    inner: Box<Read>
}

impl Read for NamedReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self.inner.read(buf) {
            Err(f) => {
                warn(format!("{}: {}", Path::new("stdin").display(), f.to_string()).as_ref());
                Err(f)
            }
            okay => okay
        }
    }
}

fn warn(message: &str) -> Error {
    error!("tee: {}", message);
    Error::new(ErrorKind::Other, format!("tee: {}", message))
}
