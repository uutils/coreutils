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
#[cfg(any(target_os = "linux", target_os = "android"))]
use nix::Error::Sys;
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
    use nix::errno::Errno::{ENOSPC, EPIPE};
    use std::process::exit;

    match try_splice(bytes) {
        Err(Sys(err)) => match err {
            EPIPE => {
                // Our pipe was interrupted, this happens, for example, when
                // the shell command `yes | head` is run.
                exit(0);
            }
            ENOSPC => {
                eprintln!("No space left on disk.");
                exit(1);
            }
            _ => {}
        },
        _ => {}
    }

    // If we reach this point, we should fall back to slow writing.
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    loop {
        stdout.write_all(bytes).unwrap();
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn try_splice(bytes: &[u8]) -> nix::Result<nix::Error> {
    use libc::{O_APPEND, S_IFIFO, S_IFREG};
    use nix::errno::Errno::UnknownErrno;
    use nix::fcntl::{fcntl, splice, vmsplice, FcntlArg, SpliceFFlags};
    use nix::sys::stat::fstat;
    use nix::sys::uio::IoVec;
    use nix::unistd::pipe;
    use std::os::unix::io::AsRawFd;

    let stdout = io::stdout();
    let byte_iovec = &[IoVec::from_slice(&bytes[..])];
    let (pipe_rd, pipe_wr) = pipe()?;

    let stdout_stat = fstat(stdout.as_raw_fd())?;
    let stdout_is_regular = (stdout_stat.st_mode & S_IFREG) != 0;

    let stdout_access_mode = fcntl(stdout.as_raw_fd(), FcntlArg::F_GETFL)?;
    let stdout_is_append = (stdout_access_mode & O_APPEND) != 0;
    let stdout_is_fifo = (stdout_stat.st_mode & S_IFIFO) != 0;

    if stdout_is_regular && !stdout_is_append {
        loop {
            vmsplice(pipe_wr, byte_iovec, SpliceFFlags::empty())?;
            splice(
                pipe_rd,
                None,
                stdout.as_raw_fd(),
                None,
                BUF_SIZE,
                SpliceFFlags::empty(),
            )?;
        }
    } else if stdout_is_fifo {
        // Stdout is already a pipe; we do not have to use an intermediate
        // pipe.
        loop {
            vmsplice(stdout.as_raw_fd(), byte_iovec, SpliceFFlags::empty())?;
        }
    }

    Err(Sys(UnknownErrno))
}
