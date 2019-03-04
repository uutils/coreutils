#![crate_name = "uu_yes"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Jordi Boggiano <j.boggiano@seld.be>
 * (c) Árni Dagur <arni@dagur.eu>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

/* last synced with: yes (GNU coreutils) 8.13 */

#[macro_use]
extern crate clap;
#[macro_use]
extern crate uucore;
#[cfg(any(target_os = "linux", target_os = "android"))]
extern crate libc;
#[cfg(any(target_os = "linux", target_os = "android"))]
extern crate nix;

use clap::Arg;
use std::borrow::Cow;
use std::io::{self, Write};

// force a re-build whenever Cargo.toml changes
const _CARGO_TOML: &str = include_str!("Cargo.toml");

// it's possible that using a smaller or larger buffer might provide better performance on some
// systems, but honestly this is good enough
const BUF_SIZE: usize = 16 * 1024;

pub fn uumain(args: Vec<String>) -> i32 {
    let app = app_from_crate!().arg(Arg::with_name("STRING").index(1).multiple(true));

    let matches = match app.get_matches_from_safe(args) {
        Ok(m) => m,
        Err(ref e)
            if e.kind == clap::ErrorKind::HelpDisplayed
                || e.kind == clap::ErrorKind::VersionDisplayed =>
        {
            println!("{}", e);
            return 0;
        }
        Err(f) => {
            show_error!("{}", f);
            return 1;
        }
    };

    let string = if let Some(values) = matches.values_of("STRING") {
        let mut result = values.fold(String::new(), |res, s| res + s + " ");
        result.pop();
        result.push('\n');
        Cow::from(result)
    } else {
        Cow::from("y\n")
    };

    let mut buffer = [0; BUF_SIZE];
    let bytes = prepare_buffer(&string, &mut buffer);

    exec(bytes);

    0
}

#[cfg(not(feature = "latency"))]
fn prepare_buffer<'a>(input: &'a str, buffer: &'a mut [u8; BUF_SIZE]) -> &'a [u8] {
    if input.len() < BUF_SIZE / 2 {
        let mut size = 0;
        while size < BUF_SIZE - input.len() {
            let (_, right) = buffer.split_at_mut(size);
            right[..input.len()].copy_from_slice(input.as_bytes());
            size += input.len();
        }
        &buffer[..size]
    } else {
        input.as_bytes()
    }
}

#[cfg(feature = "latency")]
fn prepare_buffer<'a>(input: &'a str, _buffer: &'a mut [u8; BUF_SIZE]) -> &'a [u8] {
    input.as_bytes()
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
pub fn exec(bytes: &[u8]) {
    let stdout_raw = io::stdout();
    let mut stdout = stdout_raw.lock();
    loop {
        stdout.write_all(bytes).unwrap();
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn exec(bytes: &[u8]) {
    use libc::{O_APPEND, S_IFIFO};
    use nix::errno::EPIPE;
    use nix::fcntl::{fcntl, splice, vmsplice, FcntlArg, SpliceFFlags};
    use nix::sys::stat::fstat;
    use nix::sys::uio::IoVec;
    use nix::unistd::pipe;
    use nix::Error::Sys;
    use std::os::unix::io::AsRawFd;

    let stdout = io::stdout();
    let stdout_stat = fstat(stdout.as_raw_fd()).unwrap();
    let stdout_is_pipe = (stdout_stat.st_mode & S_IFIFO) != 0;
    let stdout_access_mode = fcntl(stdout.as_raw_fd(), FcntlArg::F_GETFL).unwrap();
    let stdout_is_append = (stdout_access_mode & O_APPEND) != 0;

    if stdout_is_append {
        // If the splice system calls are given a file descriptor opened in
        // append mode, they return Sys(EINVAL) error. Thus we fall back to
        // slow writing.
        let mut stdout = stdout.lock();
        loop {
            stdout.write_all(bytes).unwrap();
        }
    } else {
        let bytes = &[IoVec::from_slice(&bytes[..])];
        if stdout_is_pipe {
            loop {
                let res = vmsplice(stdout.as_raw_fd(), bytes, SpliceFFlags::empty());
                match res {
                    Err(err) => {
                        if err == Sys(EPIPE) {
                            // This means that our pipe was interrupted, e.g.
                            // `yes | head` is run. Like GNU yes, we make a
                            // graceful exit.
                            break;
                        } else {
                            panic!("{:?}", err);
                        }
                    }
                    _ => {}
                }
            }
        } else {
            // If stdout is not a pipe, e.g. if you make `yes` print right to
            // the terminal, we have to use an intermediate pipe; vmsplice
            // only works on pipes.
            let (pipe_rd, pipe_wr) = pipe().unwrap();
            loop {
                vmsplice(pipe_wr, bytes, SpliceFFlags::empty()).unwrap();

                splice(
                    pipe_rd,
                    None,
                    stdout.as_raw_fd(),
                    None,
                    BUF_SIZE,
                    SpliceFFlags::empty(),
                )
                .unwrap();
            }
        }
    }
}
