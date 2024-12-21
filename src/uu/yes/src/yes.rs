// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// cSpell:ignore strs

use clap::{builder::ValueParser, crate_version, Arg, ArgAction, Command};
use std::error::Error;
use std::ffi::OsString;
use std::io::{stdout, Write};
use uucore::error::UResult;
use uucore::{format_usage, help_about, help_usage};

#[cfg(any(target_os = "linux", target_os = "android"))]
use std::os::fd::AsFd;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::os::fd::AsRawFd;
#[cfg(any(target_os = "linux", target_os = "android"))]
use uucore::buf_copy::is_pipe;
#[cfg(any(target_os = "linux", target_os = "android"))]
use uucore::buf_copy::{splice_data_to_fd, splice_data_to_pipe};
#[cfg(any(target_os = "linux", target_os = "android"))]
use uucore::pipes::pipe;

#[cfg(unix)]
use uucore::signals::enable_pipe_errors;

const ABOUT: &str = help_about!("yes.md");
const USAGE: &str = help_usage!("yes.md");

// it's possible that using a smaller or larger buffer might provide better performance on some
// systems, but honestly this is good enough
const BUF_SIZE: usize = 16 * 1024;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let mut buffer = Vec::with_capacity(BUF_SIZE);
    args_into_buffer(&mut buffer, matches.get_many::<OsString>("STRING")).unwrap();
    prepare_buffer(&mut buffer);

    exec(&buffer)
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .arg(
            Arg::new("STRING")
                .value_parser(ValueParser::os_string())
                .action(ArgAction::Append),
        )
        .infer_long_args(true)
}

// Copies words from `i` into `buf`, separated by spaces.
fn args_into_buffer<'a>(
    buf: &mut Vec<u8>,
    i: Option<impl Iterator<Item = &'a OsString>>,
) -> Result<(), Box<dyn Error>> {
    let Some(i) = i else {
        buf.extend_from_slice(b"y\n");
        return Ok(());
    };

    // On Unix (and wasi), OsStrs are just &[u8]'s underneath...
    #[cfg(any(unix, target_os = "wasi"))]
    {
        #[cfg(unix)]
        use std::os::unix::ffi::OsStrExt;
        #[cfg(target_os = "wasi")]
        use std::os::wasi::ffi::OsStrExt;

        for part in itertools::intersperse(i.map(|a| a.as_bytes()), b" ") {
            buf.extend_from_slice(part);
        }
    }

    // But, on Windows, we must hop through a String.
    #[cfg(not(any(unix, target_os = "wasi")))]
    {
        for part in itertools::intersperse(i.map(|a| a.to_str()), Some(" ")) {
            let bytes = match part {
                Some(part) => part.as_bytes(),
                None => return Err("arguments contain invalid UTF-8".into()),
            };
            buf.extend_from_slice(bytes);
        }
    }

    buf.push(b'\n');

    Ok(())
}

// Assumes buf holds a single output line forged from the command line arguments, copies it
// repeatedly until the buffer holds as many copies as it can under BUF_SIZE.
fn prepare_buffer(buf: &mut Vec<u8>) {
    if buf.len() * 2 > BUF_SIZE {
        return;
    }

    assert!(!buf.is_empty());

    let line_len = buf.len();
    let target_size = line_len * (BUF_SIZE / line_len);

    while buf.len() < target_size {
        let to_copy = std::cmp::min(target_size - buf.len(), buf.len());
        debug_assert_eq!(to_copy % line_len, 0);
        buf.extend_from_within(..to_copy);
    }
}

/// On Linux and Android, repeatedly call the splice function to write our buffered string into
/// standard output using the `splice` system call.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn loop_splice_data<T>(bytes: &[u8], out: &T) -> UResult<()>
where
    T: AsRawFd + AsFd,
{
    let is_pipe = is_pipe(out)?;

    #[cfg(any(target_os = "linux", target_os = "android"))]

    // Under the if statements below, an Ok(()) is returned to indicate that the
    // splice call failed but is still recoverable, i.e by using the
    // stdout.write_all method.
    if is_pipe {

        loop {
            if splice_data_to_pipe(bytes, out)?.1 {
                return Ok(());
            }
        }
    } else {
        let (read, write) = pipe()?;
        loop {
            if splice_data_to_fd(bytes, &read, &write, out)?.1 {
                return Ok(());
            };
        }
    }
}

pub fn exec(bytes: &[u8]) -> UResult<()> {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    #[cfg(unix)]
    enable_pipe_errors()?;

    #[cfg(any(target_os = "linux", target_os = "android"))]
    loop_splice_data(bytes, &stdout)?;

    loop {
        stdout.write_all(bytes)?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepare_buffer() {
        let tests = [
            (150, 16350),
            (1000, 16000),
            (4093, 16372),
            (4099, 12297),
            (4111, 12333),
            (2, 16384),
            (3, 16383),
            (4, 16384),
            (5, 16380),
            (8192, 16384),
            (8191, 16382),
            (8193, 8193),
            (10000, 10000),
            (15000, 15000),
            (25000, 25000),
        ];

        for (line, final_len) in tests {
            let mut v = std::iter::repeat(b'a').take(line).collect::<Vec<_>>();
            prepare_buffer(&mut v);
            assert_eq!(v.len(), final_len);
        }
    }

    #[test]
    fn test_args_into_buf() {
        {
            let mut v = Vec::with_capacity(BUF_SIZE);
            args_into_buffer(&mut v, None::<std::slice::Iter<OsString>>).unwrap();
            assert_eq!(String::from_utf8(v).unwrap(), "y\n");
        }

        {
            let mut v = Vec::with_capacity(BUF_SIZE);
            args_into_buffer(&mut v, Some([OsString::from("foo")].iter())).unwrap();
            assert_eq!(String::from_utf8(v).unwrap(), "foo\n");
        }

        {
            let mut v = Vec::with_capacity(BUF_SIZE);
            args_into_buffer(
                &mut v,
                Some(
                    [
                        OsString::from("foo"),
                        OsString::from("bar    baz"),
                        OsString::from("qux"),
                    ]
                    .iter(),
                ),
            )
            .unwrap();
            assert_eq!(String::from_utf8(v).unwrap(), "foo bar    baz qux\n");
        }
    }
}
