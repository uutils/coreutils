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
#[phase(plugin, link)] extern crate log;

use std::io::{println, stdin, stdout, Append, File, Truncate, Write};
use std::io::{IoResult};
use std::io::util::{copy, NullWriter, MultiWriter};
use std::os::{args, set_exit_status};
use getopts::{getopts, optflag, usage};

static NAME: &'static str = "tee";
static VERSION: &'static str = "1.0.0";

#[allow(dead_code)]
fn main() { uumain(args()); }

pub fn uumain(args: Vec<String>) {
    match options(args.as_slice()).and_then(exec) {
        Ok(_) => set_exit_status(0),
        Err(_) => set_exit_status(1)
    }
}

struct Options {
    #[allow(dead_code)]
    program: String,
    append: bool,
    #[allow(dead_code)]
    ignore_interrupts: bool,
    print_and_exit: Option<String>,
    files: Box<Vec<Path>>
}

fn options(args: &[String]) -> Result<Options, ()> {
    let opts = [
        optflag("a", "append", "append to the given FILEs, do not overwrite"),
        optflag("i", "ignore-interrupts", "ignore interrupt signals"),
        optflag("h", "help", "display this help and exit"),
        optflag("V", "version", "output version information and exit"),
    ];

    let args: Vec<String> = args.iter().map(|x| x.to_string()).collect();

    getopts(args.tail(), opts).map_err(|e| e.to_err_msg()).and_then(|m| {
        let version = format!("{} {}", NAME, VERSION);
        let program = args.get(0).as_slice();
        let arguments = "[OPTION]... [FILE]...";
        let brief = "Copy standard input to each FILE, and also to standard output.";
        let comment = "If a FILE is -, copy again to standard output.";
        let help = format!("{}\n\nUsage:\n  {} {}\n\n{}\n{}",
                           version, program, arguments, usage(brief, opts),
                           comment);
        let names = m.free.clone().move_iter().collect::<Vec<String>>().append_one("-".to_string());
        let to_print = if m.opt_present("help") { Some(help) }
                       else if m.opt_present("version") { Some(version) }
                       else { None };
        Ok(Options {
            program: program.to_string(),
            append: m.opt_present("append"),
            ignore_interrupts: m.opt_present("ignore-interrupts"),
            print_and_exit: to_print,
            files: box names.iter().map(|name| Path::new(name.clone())).collect()
        })
    }).map_err(|message| warn(message.as_slice()))
}

fn exec(options: Options) -> Result<(), ()> {
    match options.print_and_exit {
        Some(text) => Ok(println(text.as_slice())),
        None => tee(options)
    }
}

fn tee(options: Options) -> Result<(), ()> {
    let writers = options.files.iter().map(|path| open(path, options.append)).collect();
    let output = &mut MultiWriter::new(writers);
    let input = &mut NamedReader { inner: box stdin() as Box<Reader> };
    if copy(input, output).is_err() || output.flush().is_err() {
        Err(())
    } else {
        Ok(())
    }
}

fn open(path: &Path, append: bool) -> Box<Writer> {
    let inner = if *path == Path::new("-") {
        box stdout() as Box<Writer>
    } else {
        let mode = if append { Append } else { Truncate };
        match File::open_mode(path, mode, Write) {
            Ok(file) => box file as Box<Writer>,
            Err(_) => box NullWriter as Box<Writer>
        }
    };
    box NamedWriter { inner: inner, path: box path.clone() } as Box<Writer>
}

struct NamedWriter {
    inner: Box<Writer>,
    path: Box<Path>
}

impl Writer for NamedWriter {
    fn write(&mut self, buf: &[u8]) -> IoResult<()> {
        with_path(self.path.clone(), || {
            let val = self.inner.write(buf);
            if val.is_err() {
                self.inner = box NullWriter as Box<Writer>;
            }
            val
        })
    }

    fn flush(&mut self) -> IoResult<()> {
        with_path(self.path.clone(), || {
            let val = self.inner.flush();
            if val.is_err() {
                self.inner = box NullWriter as Box<Writer>;
            }
            val
        })
    }
}

struct NamedReader {
    inner: Box<Reader>
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
        Err(f) => { warn(format!("{}: {}", path.display(), f.to_str()).as_slice()); Err(f) }
        okay => okay
    }
}

fn warn(message: &str) {
    error!("{}: {}", args().get(0), message);
}
