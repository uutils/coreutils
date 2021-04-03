//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Martin Kysel <code@martinkysel.com>
//  *
//  * For the full copyright and license information, please view the LICENSE file
//  * that was distributed with this source code.

// spell-checker:ignore (ToDO) lflag ICANON tcgetattr tcsetattr TCSADRAIN

#[macro_use]
extern crate uucore;

use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, Read, Write};

#[cfg(all(unix, not(target_os = "fuchsia")))]
extern crate nix;
#[cfg(all(unix, not(target_os = "fuchsia")))]
use nix::sys::termios::{self, LocalFlags, SetArg};
use uucore::InvalidEncodingHandling;

#[cfg(target_os = "redox")]
extern crate redox_termios;
#[cfg(target_os = "redox")]
extern crate syscall;

use clap::{App, Arg, ArgMatches};

static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str = "A file perusal filter for CRT viewing.";

mod options {
    pub const FILE: &str = "file";
}

fn get_usage() -> String {
    format!("{} [options] <file>...", executable!())
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let usage = get_usage();
    let args = args
        .collect_str(InvalidEncodingHandling::ConvertLossy)
        .accept_any();

    let matches = App::new(executable!())
        .version(VERSION)
        .usage(usage.as_str())
        .about(ABOUT)
        .arg(
            Arg::with_name(options::FILE)
                .number_of_values(1)
                .multiple(true),
        )
        .get_matches_from(args);

    // FixME: fail without panic for now; but `more` should work with no arguments (ie, for piped input)
    if let None | Some("-") = matches.value_of(options::FILE) {
        show_usage_error!("Reading from stdin isn't supported yet.");
        return 1;
    }

    if let Some(x) = matches.value_of(options::FILE) {
        let path = std::path::Path::new(x);
        if path.is_dir() {
            show_usage_error!("'{}' is a directory.", x);
            return 1;
        }
    }

    more(matches);

    0
}

#[cfg(all(unix, not(target_os = "fuchsia")))]
fn setup_term() -> termios::Termios {
    let mut term = termios::tcgetattr(0).unwrap();
    // Unset canonical mode, so we get characters immediately
    term.local_flags.remove(LocalFlags::ICANON);
    // Disable local echo
    term.local_flags.remove(LocalFlags::ECHO);
    termios::tcsetattr(0, SetArg::TCSADRAIN, &term).unwrap();
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
    term.local_flags &= !redox_termios::ICANON;
    term.local_flags &= !redox_termios::ECHO;
    syscall::write(fd, &term).unwrap();
    let _ = syscall::close(fd);
    term
}

#[cfg(all(unix, not(target_os = "fuchsia")))]
fn reset_term(term: &mut termios::Termios) {
    term.local_flags.insert(LocalFlags::ICANON);
    term.local_flags.insert(LocalFlags::ECHO);
    termios::tcsetattr(0, SetArg::TCSADRAIN, &term).unwrap();
}

#[cfg(any(windows, target_os = "fuchsia"))]
#[inline(always)]
fn reset_term(_: &mut usize) {}

#[cfg(any(target_os = "redox"))]
fn reset_term(term: &mut redox_termios::Termios) {
    let fd = syscall::dup(0, b"termios").unwrap();
    syscall::read(fd, term).unwrap();
    term.local_flags |= redox_termios::ICANON;
    term.local_flags |= redox_termios::ECHO;
    syscall::write(fd, &term).unwrap();
    let _ = syscall::close(fd);
}

fn more(matches: ArgMatches) {
    let mut f: Box<dyn BufRead> = match matches.value_of(options::FILE) {
        None | Some("-") => Box::new(BufReader::new(stdin())),
        Some(filename) => Box::new(BufReader::new(File::open(filename).unwrap())),
    };
    let mut buffer = [0; 1024];

    let mut term = setup_term();

    let mut end = false;
    while let Ok(sz) = f.read(&mut buffer) {
        if sz == 0 {
            break;
        }
        stdout().write_all(&buffer[0..sz]).unwrap();
        for byte in std::io::stdin().bytes() {
            match byte.unwrap() {
                b' ' => break,
                b'q' | 27 => {
                    end = true;
                    break;
                }
                _ => (),
            }
        }

        if end {
            break;
        }
    }

    reset_term(&mut term);
    println!();
}
