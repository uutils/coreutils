#[link(name="tee", vers="1.0.0", author="Aleksander Bielawski")];

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Aleksander Bielawski <pabzdzdzwiagief@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern mod extra;

use std::io::{stdin, stdout, Append, File, Truncate, Write};
use std::io::{io_error, result, EndOfFile};
use std::io::signal::{Interrupt, Listener};
use std::io::util::{copy, MultiWriter};
use std::os::{args, set_exit_status};
use extra::getopts::groups::{getopts, optflag, usage};

static NAME: &'static str = "tee";
static VERSION: &'static str = "1.0.0";

fn main() {
    match options(args()).and_then(exec) {
        Err(message) => {
            error!("{}: {}", args()[0], message);
            set_exit_status(1)
        },
        Ok(status) => set_exit_status(status)
    }
}

struct Options {
    program: ~str,
    append: bool,
    ignore_interrupts: bool,
    print_and_exit: Option<~str>,
    files: ~[Path]
}

fn options(args: &[~str]) -> Result<Options, ~str> {
    let opts = ~[
        optflag("a", "append", "append to the given FILEs, do not overwrite"),
        optflag("i", "ignore-interrupts", "ignore interrupt signals"),
        optflag("", "help", "display this help and exit"),
        optflag("", "version", "output version information and exit")];
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
    })
}

fn open(path: &Path, append: bool) -> ~Writer {
    if *path == Path::new("-") {
        ~stdout() as ~Writer
    } else {
        let mode = if append { Append } else { Truncate };
        ~File::open_mode(path, mode, Write) as ~Writer
    }
}

fn exec(options: Options) -> Result<int, ~str> {
    match options.print_and_exit {
        Some(text) => {
            println(text);
            Ok(0)
        },
        None => tee(options)
    }
}

fn tee(options: Options) -> Result<int, ~str> {
    let mut handler = Listener::new();
    if options.ignore_interrupts {
        handler.register(Interrupt);
    }
    result(|| io_error::cond.trap(|e| {
        if e.kind != EndOfFile {
            io_error::cond.raise(e);
        }
    }).inside(|| {
        let writers = options.files.map(|path| open(path, options.append));
        let output = &mut MultiWriter::new(writers);
        let input = &mut stdin();
        copy(input, output);
        output.flush();
        0
    })).map_err(|err| err.desc.to_owned())
}
