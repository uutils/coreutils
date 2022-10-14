//  * This file is part of the uutils coreutils package.
//  *
//  * (c) Jordi Boggiano <j.boggiano@seld.be>
//  *
//  * For the full copyright and license information, please view the LICENSE
//  * file that was distributed with this source code.

/* last synced with: yes (GNU coreutils) 8.13 */

use std::borrow::Cow;
use std::io::{self, Result, Write};

use clap::{Arg, ArgAction, Command};
use uucore::error::{UResult, USimpleError};
use uucore::format_usage;

#[cfg(any(target_os = "linux", target_os = "android"))]
mod splice;

const ABOUT: &str = "repeatedly display a line with STRING (or 'y')";
const USAGE: &str = "{} [STRING]...";

// it's possible that using a smaller or larger buffer might provide better performance on some
// systems, but honestly this is good enough
const BUF_SIZE: usize = 16 * 1024;

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let string = if let Some(values) = matches.get_many::<String>("STRING") {
        let mut result = values.fold(String::new(), |res, s| res + s + " ");
        result.pop();
        result.push('\n');
        Cow::from(result)
    } else {
        Cow::from("y\n")
    };

    let mut buffer = [0; BUF_SIZE];
    let bytes = prepare_buffer(&string, &mut buffer);

    match exec(bytes) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(err) => Err(USimpleError::new(1, format!("standard output: {}", err))),
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .arg(Arg::new("STRING").action(ArgAction::Append))
        .infer_long_args(true)
}

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

#[cfg(unix)]
fn enable_pipe_errors() -> Result<()> {
    let ret = unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL) };
    if ret == libc::SIG_ERR {
        return Err(io::Error::new(io::ErrorKind::Other, ""));
    }
    Ok(())
}

#[cfg(not(unix))]
fn enable_pipe_errors() -> Result<()> {
    // Do nothing.
    Ok(())
}

pub fn exec(bytes: &[u8]) -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    enable_pipe_errors()?;

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        match splice::splice_data(bytes, &stdout) {
            Ok(_) => return Ok(()),
            Err(splice::Error::Io(err)) => return Err(err),
            Err(splice::Error::Unsupported) => (),
        }
    }

    loop {
        stdout.write_all(bytes)?;
    }
}
