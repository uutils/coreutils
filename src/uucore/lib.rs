#[cfg(feature = "libc")]
pub extern crate libc;
#[cfg(feature = "winapi")]
pub extern crate winapi;

pub extern crate failure;
#[macro_use]
pub extern crate failure_derive;

use std::borrow::Cow;
use std::io::{self, Read, Write};
use failure::Fail;

#[macro_use]
mod macros;

#[macro_use]
pub mod coreopts;

pub mod panic;

#[cfg(feature = "fs")]
pub mod fs;
#[cfg(feature = "utf8")]
pub mod utf8;
#[cfg(feature = "encoding")]
pub mod encoding;
#[cfg(feature = "parse_time")]
pub mod parse_time;

#[cfg(all(not(windows), feature = "mode"))]
pub mod mode;
#[cfg(all(unix, not(target_os = "fuchsia"), feature = "utmpx"))]
pub mod utmpx;
#[cfg(feature = "utsname")]
pub mod utsname;
#[cfg(all(unix, feature = "entries"))]
pub mod entries;
#[cfg(all(unix, feature = "process"))]
pub mod process;
#[cfg(all(unix, not(target_os = "fuchsia"), feature = "signals"))]
pub mod signals;

#[cfg(all(windows, feature = "wide"))]
pub mod wide;

#[cfg(unix)]
pub const PIPE_EXITCODE: i32 = (libc::SIGPIPE + 128) as i32;
#[cfg(not(unix))]
pub const PIPE_EXITCODE: i32 = 141;

pub struct ProgramInfo<'a, I: Read, O: Write, E: Write> {
    pub stdin: I,
    pub stdout: O,
    pub stderr: E,
    pub posix: bool,
    pub exitcode: i32,
    pub name: Cow<'a, str>
}

impl<'a, I: Read, O: Write, E: Write> ProgramInfo<'a, I, O, E> {
    pub fn new(stdin: I, stdout: O, stderr: E, posix: bool, name: Cow<'a, str>) -> ProgramInfo<'a, I, O, E> {
        ProgramInfo {
            stdin: stdin,
            stdout: stdout,
            stderr: stderr,
            posix: posix,
            exitcode: 0,
            name: name
        }
    }

    pub fn set_exitcode(&mut self, code: i32) {
        self.exitcode = code;
    }
}

impl<'a, I: Read, O: Write, E: Write> Write for ProgramInfo<'a, I, O, E> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stdout.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stdout.flush()
    }
}

impl<'a, I: Read, O: Write, E: Write> Read for ProgramInfo<'a, I, O, E> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stdin.read(buf)
    }
}

pub trait Util<'a, I: Read, O: Write, E: Write, S: UStatus> {
    fn uumain(args: Vec<String>, pio: &mut ProgramInfo<'a, I, O, E>) -> Result<i32, S>;

    fn entry(&self, args: Vec<String>, pio: &mut ProgramInfo<'a, I, O, E>) -> i32 {
        Self::uumain(args, pio).handle(pio)
    }
}

pub trait ResultExt {
    fn handle<'a, I: Read, O: Write, E: Write>(self, pio: &mut ProgramInfo<'a, I, O, E>) -> i32;
}

impl<T: UStatus> ResultExt for Result<i32, T> {
    fn handle<'a, I: Read, O: Write, E: Write>(self, pio: &mut ProgramInfo<'a, I, O, E>) -> i32 {
        match self {
            Ok(code) => code,
            Err(ref err) if err.is_pipe_err() => PIPE_EXITCODE,
            Err(err) => {
                let _ = show_error!(pio, "{}", err);
                err.code()
            }
        }
    }
}

pub trait UStatus: Fail + Sized {
    /// The exit code associated with this error (pipe errors are always PIPE_EXITCODE, no matter
    /// the contents of this method).
    fn code(&self) -> i32 { 1 }

    fn is_pipe_err(&self) -> bool {
        if let Some(err) = self.root_cause().downcast_ref::<io::Error>() {
            err.kind() == io::ErrorKind::BrokenPipe
        } else {
            false
        }
    }
}
