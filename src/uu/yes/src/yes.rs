// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// cSpell:ignore strs setpipe

use aligned_vec::{AVec, Alignment, ConstAlign};
use clap::{Arg, ArgAction, Command, builder::ValueParser};
use std::error::Error;
use std::ffi::OsString;
use std::io::{self, Write};
use uucore::error::{UResult, USimpleError};
use uucore::format_usage;
use uucore::translate;

#[cfg(any(target_os = "linux", target_os = "android"))]
use rustix::{
    fd::AsFd,
    param::page_size,
    pipe::{IoSliceRaw, SpliceFlags, fcntl_setpipe_size, vmsplice},
};

// Should be multiple of page size
const ROOTLESS_MAX_PIPE_SIZE: usize = 1024 * 1024;
// it's possible that using a smaller or larger buffer might provide better performance on some
// systems, but honestly this is good enough
const BUF_SIZE: usize = 16 * 1024;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;
    // use larger pipe if zero-copy is possible
    // todo: deduplicate logic
    // increase pipe size if stdout is pipe for zero-copy performance
    #[cfg(any(target_os = "linux", target_os = "android"))]
    let buf_size = fcntl_setpipe_size(io::stdout(), ROOTLESS_MAX_PIPE_SIZE).unwrap_or(BUF_SIZE);
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    let buf_size = BUF_SIZE;

    let mut buffer: AVec<u8, ConstAlign<ROOTLESS_MAX_PIPE_SIZE>> =
        AVec::with_capacity(ROOTLESS_MAX_PIPE_SIZE, buf_size);
    #[allow(clippy::unwrap_used, reason = "clap provides 'y' by default")]
    let _ = args_into_buffer(&mut buffer, matches.get_many::<OsString>("STRING").unwrap());
    prepare_buffer(&mut buffer, buf_size);

    match exec(&buffer) {
        Ok(()) => Ok(()),
        // On Windows, silently handle broken pipe since there's no SIGPIPE
        #[cfg(windows)]
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(err) => Err(USimpleError::new(
            1,
            translate!("yes-error-standard-output", "error" => err),
        )),
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("yes-about"))
        .override_usage(format_usage(&translate!("yes-usage")))
        .arg(
            Arg::new("STRING")
                .default_value("y")
                .value_parser(ValueParser::os_string())
                .action(ArgAction::Append),
        )
        .infer_long_args(true)
}

/// Copies words from `i` into `buf`, separated by spaces.
#[allow(clippy::unnecessary_wraps, reason = "needed on some platforms")]
fn args_into_buffer<'a, A: Alignment>(
    buf: &mut AVec<u8, A>,
    i: impl Iterator<Item = &'a OsString>,
) -> Result<(), Box<dyn Error>> {
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
                None => return Err(translate!("yes-error-invalid-utf8").into()),
            };
            buf.extend_from_slice(bytes);
        }
    }

    buf.push(b'\n');

    Ok(())
}

/// Assumes buf holds a single output line forged from the command line arguments, copies it
/// repeatedly until the buffer holds as many copies as it can under [`BUF_SIZE`].
fn prepare_buffer<A: Alignment>(buf: &mut AVec<u8, A>, buf_size: usize) {
    if buf.len() * 2 > buf_size {
        return;
    }

    assert!(!buf.is_empty());

    let line_len = buf.len();
    let target_size = line_len * (buf_size / line_len);

    while buf.len() < target_size {
        let current_len = buf.len();
        let to_copy = std::cmp::min(target_size - current_len, current_len);
        debug_assert_eq!(to_copy % line_len, 0);
        #[allow(
            clippy::unnecessary_to_owned,
            reason = "needs useless copy without unsafe"
        )]
        buf.extend_from_slice(&buf[..to_copy].to_vec());
    }
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
pub fn exec(bytes: &[u8]) -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    loop {
        stdout.write_all(bytes)?;
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn exec(bytes: &[u8]) -> io::Result<()> {
    let stdout = io::stdout();
    //zero copy fast-path
    //large args might not be aligned
    //todo: align instead of giving up fast-path
    let aligned = bytes.len().is_multiple_of(page_size());
    if aligned {
        let fd = stdout.as_fd();
        let iovec = [IoSliceRaw::from_slice(bytes)];
        // we no longer edit bytes until vmsplice
        // bytes should not be dropped until vmsplice finishes since we have write fallback
        while unsafe { vmsplice(fd, &iovec, SpliceFlags::GIFT) }.is_ok() {}
    }
    let mut stdout = stdout.lock();
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
            let mut v: AVec<u8, ConstAlign<ROOTLESS_MAX_PIPE_SIZE>> =
                AVec::from_iter(ROOTLESS_MAX_PIPE_SIZE, std::iter::repeat_n(b'a', line));
            prepare_buffer(&mut v, BUF_SIZE);
            assert_eq!(v.len(), final_len);
        }
    }

    #[test]
    fn test_args_into_buf() {
        {
            let mut v: AVec<u8, ConstAlign<ROOTLESS_MAX_PIPE_SIZE>> =
                AVec::with_capacity(ROOTLESS_MAX_PIPE_SIZE, BUF_SIZE);
            let default_args = ["y".into()];
            args_into_buffer(&mut v, default_args.iter()).unwrap();
            assert_eq!(String::from_utf8(v.to_vec()).unwrap(), "y\n");
        }

        {
            let mut v: AVec<u8, ConstAlign<ROOTLESS_MAX_PIPE_SIZE>> =
                AVec::with_capacity(ROOTLESS_MAX_PIPE_SIZE, BUF_SIZE);
            let args = ["foo".into()];
            args_into_buffer(&mut v, args.iter()).unwrap();
            assert_eq!(String::from_utf8(v.to_vec()).unwrap(), "foo\n");
        }

        {
            let mut v: AVec<u8, ConstAlign<ROOTLESS_MAX_PIPE_SIZE>> =
                AVec::with_capacity(ROOTLESS_MAX_PIPE_SIZE, BUF_SIZE);
            let args = ["foo".into(), "bar    baz".into(), "qux".into()];
            args_into_buffer(&mut v, args.iter()).unwrap();
            assert_eq!(
                String::from_utf8(v.to_vec()).unwrap(),
                "foo bar    baz qux\n"
            );
        }
    }
}
