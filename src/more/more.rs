#![crate_name = "uu_more"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Martin Kysel <code@martinkysel.com>
 *
 * For the full copyright and license information, please view the LICENSE file
 * that was distributed with this source code.
 */

extern crate getopts;

#[macro_use]
extern crate uucore;
extern crate term_size;

use getopts::Options;
use std::io::{stdout, Read, Write};
use std::fs::File;

#[cfg(all(unix, not(target_os = "fuchsia")))]
extern crate nix;
#[cfg(all(unix, not(target_os = "fuchsia")))]
use nix::sys::termios;

#[cfg(target_os = "redox")]
extern crate redox_termios;
#[cfg(target_os = "redox")]
extern crate syscall;

#[derive(Clone, Eq, PartialEq)]
pub enum Mode {
    More,
    Help,
    Version,
}

static NAME: &str = "more";
static VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = Options::new();

    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("v", "version", "output version information and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => {
            show_error!("{}", e);
            panic!()
        }
    };
    let usage = opts.usage("more TARGET.");
    let mode = if matches.opt_present("version") {
        Mode::Version
    } else if matches.opt_present("help") {
        Mode::Help
    } else {
        Mode::More
    };

    match mode {
        Mode::More => more(matches),
        Mode::Help => help(&usage),
        Mode::Version => version(),
    }

    0
}

fn version() {
    println!("{} {}", NAME, VERSION);
}

fn help(usage: &str) {
    let msg = format!(
        "{0} {1}\n\n\
         Usage: {0} TARGET\n  \
         \n\
         {2}",
        NAME, VERSION, usage
    );
    println!("{}", msg);
}

#[cfg(all(unix, not(target_os = "fuchsia")))]
fn setup_term() -> termios::Termios {
    let mut term = termios::tcgetattr(0).unwrap();
    // Unset canonical mode, so we get characters immediately
    term.c_lflag.remove(termios::ICANON);
    // Disable local echo
    term.c_lflag.remove(termios::ECHO);
    termios::tcsetattr(0, termios::TCSADRAIN, &term).unwrap();
    term
}

#[cfg(any(windows, target_os = "fuchsia"))]
#[inline(always)]
fn setup_term() -> usize {
    0
}

#[cfg(target_os = "redox")]
fn setup_term() -> redox_termios::Termios {
    let mut term = redox_termios::Termios::default();
    let fd = syscall::dup(0, b"termios").unwrap();
    syscall::read(fd, &mut term).unwrap();
    term.c_lflag &= !redox_termios::ICANON;
    term.c_lflag &= !redox_termios::ECHO;
    syscall::write(fd, &term).unwrap();
    let _ = syscall::close(fd);
    term
}

#[cfg(all(unix, not(target_os = "fuchsia")))]
fn reset_term(term: &mut termios::Termios) {
    term.c_lflag.insert(termios::ICANON);
    term.c_lflag.insert(termios::ECHO);
    termios::tcsetattr(0, termios::TCSADRAIN, &term).unwrap();
}

#[cfg(any(windows, target_os = "fuchsia"))]
#[inline(always)]
fn reset_term(_: &mut usize) {}

#[cfg(any(target_os = "redox"))]
fn reset_term(term: &mut redox_termios::Termios) {
    let fd = syscall::dup(0, b"termios").unwrap();
    syscall::read(fd, term).unwrap();
    term.c_lflag |= redox_termios::ICANON;
    term.c_lflag |= redox_termios::ECHO;
    syscall::write(fd, &term).unwrap();
    let _ = syscall::close(fd);
}

fn more(matches: getopts::Matches) {
    let files = matches.free;
    let mut f = File::open(files.first().unwrap()).unwrap();

    let mut buffer = [0; 1024];
    let mut term = setup_term();
    // TODO get size of actual terminal
    let term_columns = 80;
    let term_lines = 30;

    let mut want_lines = term_lines; // start with a full page; count down
    let mut columns = term_columns;   // for consistency, count down

    'chunks: while let Ok(size) = f.read(&mut buffer) {
        if size == 0 {
            break;
        }
        let mut write_start = 0;    // start of next write
        let mut point = 0;          // next char when counting lines

        loop {
            // find a subrange with the right number of lines
            while want_lines > 0 {
                let c = buffer[point];
                if c == b'\n' {
                    want_lines -= 1;
                    columns = term_columns;
                    point += 1;
                }
                else if columns == 0 {
                    // visual line, wrapped by terminal
                    want_lines -= 1;
                    columns = term_columns;
                    // don't increment point; this char needs to start the next line
                } else {
                    point += 1;
                    columns -= 1;
                }
                if point == size {
                    stdout().write(&buffer[write_start..point]).unwrap();
                    continue 'chunks
                }
            }

            stdout().write(&buffer[write_start..point]).unwrap();
            stdout().flush().unwrap();
            write_start = point;

            for byte in std::io::stdin().bytes() {
                match byte.unwrap() {
                    b' ' => {
                        want_lines = term_lines;
                        break
                    },
                    b'\n' => {
                        want_lines = 1;
                        break
                    }
                    b'q' | 27 => {
                        break 'chunks;
                    }
                    _ => (),
                }
            }
        }

    }

    reset_term(&mut term);
    println!("");
}
