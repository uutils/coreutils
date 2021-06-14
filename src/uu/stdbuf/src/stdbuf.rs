// * This file is part of the uutils coreutils package.
// *
// * (c) Dorota Kapturkiewicz <dokaptur@gmail.com>
// *
// * For the full copyright and license information, please view the LICENSE
// * file that was distributed with this source code.

// spell-checker:ignore (ToDO) tempdir dyld dylib dragonflybsd optgrps libstdbuf

#[macro_use]
extern crate uucore;

use clap::ArgMatches;
use std::convert::TryFrom;
use std::fs::File;
use std::io::{self, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;
use tempfile::TempDir;
use uucore::parse_size::parse_size;
use uucore::InvalidEncodingHandling;

use crate::app::{get_app, options};

mod app;

fn get_usage() -> String {
    format!("{0} OPTION... COMMAND", executable!())
}

const STDBUF_INJECT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/libstdbuf.so"));

enum BufferType {
    Default,
    Line,
    Size(usize),
}

struct ProgramOptions {
    stdin: BufferType,
    stdout: BufferType,
    stderr: BufferType,
}

impl<'a> TryFrom<&ArgMatches<'a>> for ProgramOptions {
    type Error = ProgramOptionsError;

    fn try_from(matches: &ArgMatches) -> Result<Self, Self::Error> {
        Ok(ProgramOptions {
            stdin: check_option(matches, options::INPUT)?,
            stdout: check_option(matches, options::OUTPUT)?,
            stderr: check_option(matches, options::ERROR)?,
        })
    }
}

struct ProgramOptionsError(String);

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "dragonflybsd"
))]
fn preload_strings() -> (&'static str, &'static str) {
    ("LD_PRELOAD", "so")
}

#[cfg(target_vendor = "apple")]
fn preload_strings() -> (&'static str, &'static str) {
    ("DYLD_LIBRARY_PATH", "dylib")
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "dragonflybsd",
    target_vendor = "apple"
)))]
fn preload_strings() -> (&'static str, &'static str) {
    crash!(1, "Command not supported for this operating system!")
}

fn check_option(matches: &ArgMatches, name: &str) -> Result<BufferType, ProgramOptionsError> {
    match matches.value_of(name) {
        Some(value) => match value {
            "L" => {
                if name == options::INPUT {
                    Err(ProgramOptionsError(
                        "line buffering stdin is meaningless".to_string(),
                    ))
                } else {
                    Ok(BufferType::Line)
                }
            }
            x => parse_size(x).map_or_else(
                |e| crash!(125, "invalid mode {}", e),
                |m| Ok(BufferType::Size(m)),
            ),
        },
        None => Ok(BufferType::Default),
    }
}

fn set_command_env(command: &mut Command, buffer_name: &str, buffer_type: BufferType) {
    match buffer_type {
        BufferType::Size(m) => {
            command.env(buffer_name, m.to_string());
        }
        BufferType::Line => {
            command.env(buffer_name, "L");
        }
        BufferType::Default => {}
    }
}

fn get_preload_env(tmp_dir: &mut TempDir) -> io::Result<(String, PathBuf)> {
    let (preload, extension) = preload_strings();
    let inject_path = tmp_dir.path().join("libstdbuf").with_extension(extension);

    let mut file = File::create(&inject_path)?;
    file.write_all(STDBUF_INJECT)?;

    Ok((preload.to_owned(), inject_path))
}

pub fn uumain(args: impl uucore::Args) -> i32 {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();
    let usage = get_usage();

    let matches = get_app(executable!())
        .usage(&usage[..])
        .get_matches_from(args);

    let options = ProgramOptions::try_from(&matches)
        .unwrap_or_else(|e| crash!(125, "{}\nTry 'stdbuf --help' for more information.", e.0));

    let mut command_values = matches.values_of::<&str>(options::COMMAND).unwrap();
    let mut command = Command::new(command_values.next().unwrap());
    let command_params: Vec<&str> = command_values.collect();

    let mut tmp_dir = tempdir().unwrap();
    let (preload_env, libstdbuf) = return_if_err!(1, get_preload_env(&mut tmp_dir));
    command.env(preload_env, libstdbuf);
    set_command_env(&mut command, "_STDBUF_I", options.stdin);
    set_command_env(&mut command, "_STDBUF_O", options.stdout);
    set_command_env(&mut command, "_STDBUF_E", options.stderr);
    command.args(command_params);

    let mut process = match command.spawn() {
        Ok(p) => p,
        Err(e) => crash!(1, "failed to execute process: {}", e),
    };
    match process.wait() {
        Ok(status) => match status.code() {
            Some(i) => i,
            None => crash!(1, "process killed by signal {}", status.signal().unwrap()),
        },
        Err(e) => crash!(1, "{}", e),
    }
}
