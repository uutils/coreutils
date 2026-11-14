// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command, builder::ValueParser};
use std::ffi::OsString;
use uucore::error::{UResult, USimpleError, strip_errno};
use uucore::{format_usage, translate};

#[cfg(any(target_os = "linux", target_os = "android"))]
#[path = "platform/linux.rs"]
mod platform;
#[cfg(not(any(target_os = "linux", target_os = "android")))]
#[path = "platform/portable.rs"]
mod platform;

use platform::BUF_SIZE;
pub use platform::exec;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;

    #[allow(clippy::unwrap_used, reason = "clap provides 'y' by default")]
    let mut buffer = args_into_buffer(matches.get_many::<OsString>("STRING").unwrap());
    // On the platform OsStr is not &[u8], reject invalid utf8
    // todo: accept invalid utf8 on safe output type
    #[cfg(not(any(unix, target_os = "wasi")))]
    std::str::from_utf8(&buffer).map_err(|e| USimpleError::new(1, format!("{e}")))?;

    repeat_content_to_capacity(&mut buffer);
    match exec(&buffer) {
        Ok(()) => Ok(()),
        // On Windows and WASI, silently handle broken pipe since there's no SIGPIPE
        #[cfg(any(windows, target_os = "wasi"))]
        Err(err) if err.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
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
fn args_into_buffer<'a>(i: impl Iterator<Item = &'a OsString>) -> Vec<u8> {
    let mut buf = Vec::with_capacity(BUF_SIZE);
    for part in itertools::intersperse(i.map(|a| a.as_encoded_bytes()), b" ") {
        buf.extend_from_slice(part);
    }
    buf.push(b'\n');
    buf
}

/// Assumes buf holds a single output line forged from the command line arguments, copies it
/// repeatedly until the buffer holds as many copies as it can
fn repeat_content_to_capacity(buf: &mut Vec<u8>) {
    let line_len = buf.len();
    debug_assert!(line_len > 0, "buffer is not empty since we have newline");
    let target_size = line_len * (buf.capacity() / line_len); // 0 if line_len is already large enough

    while buf.len() < target_size {
        let to_copy = std::cmp::min(target_size - buf.len(), buf.len());
        debug_assert_eq!(to_copy % line_len, 0);
        buf.extend_from_within(..to_copy);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(not(any(target_os = "linux", target_os = "android")))] // Linux uses different buffer size
    fn test_prepare_buffer() {
        let tests = [
            (150, 32700),
            (1000, 32000),
            (4093, 32744),
            (4099, 28693),
            (4111, 28777),
            (2, 32768),
            (3, 32766),
            (4, 32768),
            (5, 32765),
            (8192, 32768),
            (8191, 32764),
            (8193, 24579),
            (10000, 30000),
            (15000, 30000),
            (25000, 25000),
        ];

        for (line, final_len) in tests {
            let mut v = Vec::with_capacity(BUF_SIZE);
            v.extend(std::iter::repeat_n(b'a', line));
            repeat_content_to_capacity(&mut v);
            assert_eq!(v.len(), final_len);
        }
    }

    #[test]
    fn test_args_into_buf() {
        {
            let default_args = ["y".into()];
            let v = args_into_buffer(default_args.iter());
            assert_eq!(String::from_utf8(v).unwrap(), "y\n");
        }

        {
            let args = ["foo".into()];
            let v = args_into_buffer(args.iter());
            assert_eq!(String::from_utf8(v).unwrap(), "foo\n");
        }

        {
            let args = ["foo".into(), "bar    baz".into(), "qux".into()];
            let v = args_into_buffer(args.iter());
            assert_eq!(String::from_utf8(v).unwrap(), "foo bar    baz qux\n");
        }
    }
}
