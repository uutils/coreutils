#[crate_id(name="tee", vers="1.0.0", author="Aleksander Bielawski")];
#[license="MIT"];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Aleksander Bielawski <pabzdzdzwiagief@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern mod extra;

use std::io::{println, stdin, stdout, Append, File, Truncate, Write};
use std::io::{io_error, EndOfFile};
use std::io::signal::{Interrupt, Listener};
use std::io::util::{copy, NullWriter, MultiWriter};
use std::os::{args, set_exit_status};
use extra::getopts::groups::{getopts, optflag, usage};

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
    files: ~[Path]
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
        let names = m.free + ~[~"-"];
        let to_print = if m.opt_present("help") { Some(help) }
                       else if m.opt_present("version") { Some(version) }
                       else { None };
        Ok(Options {
            program: program,
            append: m.opt_present("append"),
            ignore_interrupts: m.opt_present("ignore-interrupts"),
            print_and_exit: to_print,
            files: names.map(|name| Path::new(name.clone()))
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
    let mut handler = Listener::new();
    if options.ignore_interrupts {
        handler.register(Interrupt);
    }
    let mut ok = true;
    io_error::cond.trap(|_| ok = false).inside(|| {
        let writers = options.files.map(|path| open(path, options.append));
        let output = &mut MultiWriter::new(writers);
        let input = &mut NamedReader { inner: ~stdin() as ~Reader };
        copy(input, output);
        output.flush();
    });
    if ok { Ok(()) } else { Err(()) }
}

fn open(path: &Path, append: bool) -> ~Writer {
    let inner = with_path(path, || if *path == Path::new("-") {
        ~stdout() as ~Writer
    } else {
        let mode = if append { Append } else { Truncate };
        match File::open_mode(path, mode, Write) {
            Some(file) => ~file as ~Writer,
            None => ~NullWriter as ~Writer
        }
    });
    ~NamedWriter { inner: inner, path: ~path.clone() } as ~Writer
}

struct NamedWriter {
    priv inner: ~Writer,
    priv path: ~Path
}

impl Writer for NamedWriter {
    fn write(&mut self, buf: &[u8]) {
        with_path(self.path, || io_error::cond.trap(|e| {
            self.inner = ~NullWriter as ~Writer;
            io_error::cond.raise(e);
        }).inside(|| self.inner.write(buf)))
    }

    fn flush(&mut self) {
        with_path(self.path, || io_error::cond.trap(|e| {
            self.inner = ~NullWriter as ~Writer;
            io_error::cond.raise(e);
        }).inside(|| self.inner.flush()))
    }
}

struct NamedReader {
    priv inner: ~Reader
}

impl Reader for NamedReader {
    fn read(&mut self, buf: &mut [u8]) -> Option<uint> {
        with_path(&Path::new("stdin"), || io_error::cond.trap(|e| {
            if e.kind != EndOfFile {
                io_error::cond.raise(e)
            }
        }).inside(|| self.inner.read(buf)))

    }
}

fn with_path<T>(path: &Path, cb: || -> T) -> T {
    io_error::cond.trap(|e| {
        warn(format!("{}: {}", path.display(), e.desc));
        io_error::cond.raise(e);
    }).inside(cb)
}

fn warn(message: &str) {
    error!("{}: {}", args()[0], message);
}
