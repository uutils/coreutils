// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// cSpell:ignore strs

use clap::{Arg, ArgAction, Command, builder::ValueParser};
use std::ffi::OsString;
use std::io::{self, Write};
use uucore::error::{UResult, USimpleError, strip_errno};
use uucore::format_usage;
use uucore::translate;

// it's possible that using a smaller or larger buffer might provide better performance
const BUF_SIZE: usize = 16 * 1024;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    #[allow(clippy::unwrap_used, reason = "clap provides 'y' by default")]
    let mut buffer = args_into_buffer(matches.get_many::<OsString>("STRING").unwrap())?;
    prepare_buffer(&mut buffer);

    match exec(&buffer) {
        Ok(()) => Ok(()),
        // On Windows, silently handle broken pipe since there's no SIGPIPE
        #[cfg(windows)]
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(err) => Err(USimpleError::new(
            1,
            translate!("yes-error-standard-output", "error" => strip_errno(&err)),
        )),
    }
}

pub fn uu_app() -> Command {
    Command::new("yes")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("yes"))
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

/// create a buffer filled by words `i` separated by spaces.
#[allow(clippy::unnecessary_wraps, reason = "needed on some platforms")]
fn args_into_buffer<'a>(i: impl Iterator<Item = &'a OsString>) -> UResult<Vec<u8>> {
    let mut buf = Vec::with_capacity(BUF_SIZE);
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
                None => {
                    return Err(USimpleError::new(1, translate!("yes-error-invalid-utf8")));
                }
            };
            buf.extend_from_slice(bytes);
        }
    }

    buf.push(b'\n');

    Ok(buf)
}

/// Assumes buf holds a single output line forged from the command line arguments, copies it
/// repeatedly until the buffer holds as many copies as it can
fn prepare_buffer(buf: &mut Vec<u8>) {
    let line_len = buf.len();
    debug_assert!(line_len > 0, "buffer is not empty since we have newline");
    let target_size = line_len * (buf.capacity() / line_len); // 0 if line_len is already large enough

    while buf.len() < target_size {
        let to_copy = std::cmp::min(target_size - buf.len(), buf.len());
        debug_assert_eq!(to_copy % line_len, 0);
        buf.extend_from_within(..to_copy);
    }
}

pub fn exec(bytes: &[u8]) -> io::Result<()> {
    let stdout = io::stdout();
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
            let mut v = Vec::with_capacity(BUF_SIZE);
            v.extend(std::iter::repeat_n(b'a', line));
            prepare_buffer(&mut v);
            assert_eq!(v.len(), final_len);
        }
    }

    #[test]
    fn test_args_into_buf() {
        {
            let default_args = ["y".into()];
            let v = args_into_buffer(default_args.iter()).unwrap();
            assert_eq!(String::from_utf8(v).unwrap(), "y\n");
        }

        {
            let args = ["foo".into()];
            let v = args_into_buffer(args.iter()).unwrap();
            assert_eq!(String::from_utf8(v).unwrap(), "foo\n");
        }

        {
            let args = ["foo".into(), "bar    baz".into(), "qux".into()];
            let v = args_into_buffer(args.iter()).unwrap();
            assert_eq!(String::from_utf8(v).unwrap(), "foo bar    baz qux\n");
        }
    }
}
