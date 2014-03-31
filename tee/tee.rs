#![crate_id(name="tee", vers="1.0.0", author="Aleksander Bielawski")]
#![license="MIT"]
#![feature(phase)]
#![feature(macro_rules)]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Aleksander Bielawski <pabzdzdzwiagief@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate getopts;
#[phase(syntax, link)] extern crate log;

use std::io::{println, stdin, stdout, Append, File, Truncate, Write};
use std::io::{IoResult};
use std::io::util::{copy, NullWriter, MultiWriter};
use std::os::{args, set_exit_status};
use getopts::{getopts, optflag, usage};

static NAME: &'static str = "tee";
static VERSION: &'static str = "1.0.0";

fn main() {
    match options(args()).and_then(exec) {
        Ok(_) => set_exit_status(0),
        Err(_) => set_exit_status(1)
    }
}

struct Options {
    program: ~str,
    append: bool,
    ignore_interrupts: bool,
    print_and_exit: Option<~str>,
    files: ~Vec<Path>
}

fn options(args: &[~str]) -> Result<Options, ()> {
    let opts = ~[
        optflag("a", "append", "append to the given FILEs, do not overwrite"),
        optflag("i", "ignore-interrupts", "ignore interrupt signals"),
        optflag("h", "help", "display this help and exit"),
        optflag("V", "version", "output version information and exit"),
    ];

    getopts(args.tail(), opts).map_err(|e| e.to_err_msg()).and_then(|m| {
        let version = format!("{} {}", NAME, VERSION);
        let program = args[0].clone();
        let arguments = "[OPTION]... [FILE]...";
        let brief = "Copy standard input to each FILE, and also to standard " +
                    "output.";
        let comment = "If a FILE is -, copy again to standard output.";
        let help = format!("{}\n\nUsage:\n  {} {}\n\n{}\n{}",
                           version, program, arguments, usage(brief, opts),
                           comment);
        let names = std::vec::append_one(m.free.clone(), ~"-");
        let to_print = if m.opt_present("help") { Some(help) }
                       else if m.opt_present("version") { Some(version) }
                       else { None };
        Ok(Options {
            program: program,
            append: m.opt_present("append"),
            ignore_interrupts: m.opt_present("ignore-interrupts"),
            print_and_exit: to_print,
            files: ~names.iter().map(|name| Path::new(name.clone())).collect()
        })
    }).map_err(|message| warn(message))
}

fn exec(options: Options) -> Result<(), ()> {
    match options.print_and_exit {
        Some(text) => Ok(println(text)),
        None => tee(options)
    }
}

fn tee(options: Options) -> Result<(), ()> {
    let writers = options.files.iter().map(|path| open(path, options.append)).collect();
    let output = &mut MultiWriter::new(writers);
    let input = &mut NamedReader { inner: ~stdin() as ~Reader };
    if copy(input, output).is_err() || output.flush().is_err() {
        Err(())
    } else {
        Ok(())
    }
}

fn open(path: &Path, append: bool) -> ~Writer {
    let inner = if *path == Path::new("-") {
        ~stdout() as ~Writer
    } else {
        let mode = if append { Append } else { Truncate };
        match File::open_mode(path, mode, Write) {
            Ok(file) => ~file as ~Writer,
            Err(_) => ~NullWriter as ~Writer
        }
    };
    ~NamedWriter { inner: inner, path: ~path.clone() } as ~Writer
}

struct NamedWriter {
    inner: ~Writer,
    path: ~Path
}

impl Writer for NamedWriter {
    fn write(&mut self, buf: &[u8]) -> IoResult<()> {
        with_path(self.path.clone(), || {
            let val = self.inner.write(buf);
            if val.is_err() {
                self.inner = ~NullWriter as ~Writer;
            }
            val
        })
    }

    fn flush(&mut self) -> IoResult<()> {
        with_path(self.path.clone(), || {
            let val = self.inner.flush();
            if val.is_err() {
                self.inner = ~NullWriter as ~Writer;
            }
            val
        })
    }
}

struct NamedReader {
    inner: ~Reader
}

impl Reader for NamedReader {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<uint> {
        with_path(&Path::new("stdin"), || {
            self.inner.read(buf)
        })
    }
}

fn with_path<T>(path: &Path, cb: || -> IoResult<T>) -> IoResult<T> {
    match cb() {
        Err(f) => { warn(format!("{}: {}", path.display(), f.to_str())); Err(f) }
        okay => okay
    }
}

fn warn(message: &str) {
    error!("{}: {}", args()[0], message);
}
