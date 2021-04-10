// * This file is part of the uutils coreutils package.
// *
// * (c) Dorota Kapturkiewicz <dokaptur@gmail.com>
// *
// * For the full copyright and license information, please view the LICENSE
// * file that was distributed with this source code.

// spell-checker:ignore (ToDO) tempdir dyld dylib dragonflybsd optgrps libstdbuf

#[macro_use]
extern crate uucore;

use clap::{App, AppSettings, Arg, ArgMatches};
use std::convert::TryFrom;
use std::fs::File;
use std::io::{self, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;
use tempfile::TempDir;

static VERSION: &str = env!("CARGO_PKG_VERSION");
static ABOUT: &str =
    "Run COMMAND, with modified buffering operations for its standard streams.\n\n\
                      Mandatory arguments to long options are mandatory for short options too.";
static LONG_HELP: &str = "If MODE is 'L' the corresponding stream will be line buffered.\n\
                          This option is invalid with standard input.\n\n\
                          If MODE is '0' the corresponding stream will be unbuffered.\n\n\
                          Otherwise MODE is a number which may be followed by one of the following:\n\n\
                          KB 1000, K 1024, MB 1000*1000, M 1024*1024, and so on for G, T, P, E, Z, Y.\n\
                          In this case the corresponding stream will be fully buffered with the buffer size set to \
                          MODE bytes.\n\n\
                          NOTE: If COMMAND adjusts the buffering of its standard streams ('tee' does for e.g.) then \
                          that will override corresponding settings changed by 'stdbuf'.\n\
                          Also some filters (like 'dd' and 'cat' etc.) don't use streams for I/O, \
                          and are thus unaffected by 'stdbuf' settings.\n";

mod options {
    pub const INPUT: &str = "input";
    pub const INPUT_SHORT: &str = "i";
    pub const OUTPUT: &str = "output";
    pub const OUTPUT_SHORT: &str = "o";
    pub const ERROR: &str = "error";
    pub const ERROR_SHORT: &str = "e";
    pub const COMMAND: &str = "command";
}

fn get_usage() -> String {
    format!("{0} OPTION... COMMAND", executable!())
}

const STDBUF_INJECT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/libstdbuf.so"));

enum BufferType {
    Default,
    Line,
    Size(u64),
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
            stdin: check_option(&matches, options::INPUT)?,
            stdout: check_option(&matches, options::OUTPUT)?,
            stderr: check_option(&matches, options::ERROR)?,
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

fn parse_size(size: &str) -> Option<u64> {
    let ext = size.trim_start_matches(|c: char| c.is_digit(10));
    let num = size.trim_end_matches(char::is_alphabetic);
    let mut recovered = num.to_owned();
    recovered.push_str(ext);
    if recovered != size {
        return None;
    }
    let buf_size: u64 = match num.parse().ok() {
        Some(m) => m,
        None => return None,
    };
    let (power, base): (u32, u64) = match ext {
        "" => (0, 0),
        "KB" => (1, 1024),
        "K" => (1, 1000),
        "MB" => (2, 1024),
        "M" => (2, 1000),
        "GB" => (3, 1024),
        "G" => (3, 1000),
        "TB" => (4, 1024),
        "T" => (4, 1000),
        "PB" => (5, 1024),
        "P" => (5, 1000),
        "EB" => (6, 1024),
        "E" => (6, 1000),
        "ZB" => (7, 1024),
        "Z" => (7, 1000),
        "YB" => (8, 1024),
        "Y" => (8, 1000),
        _ => return None,
    };
    Some(buf_size * base.pow(power))
}

fn check_option(matches: &ArgMatches, name: &str) -> Result<BufferType, ProgramOptionsError> {
    match matches.value_of(name) {
        Some(value) => match &value[..] {
            "L" => {
                if name == options::INPUT {
                    Err(ProgramOptionsError(format!(
                        "line buffering stdin is meaningless"
                    )))
                } else {
                    Ok(BufferType::Line)
                }
            }
            x => {
                let size = match parse_size(x) {
                    Some(m) => m,
                    None => return Err(ProgramOptionsError(format!("invalid mode {}", x))),
                };
                Ok(BufferType::Size(size))
            }
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
    let usage = get_usage();

    let matches = App::new(executable!())
        .version(VERSION)
        .about(ABOUT)
        .usage(&usage[..])
        .after_help(LONG_HELP)
        .setting(AppSettings::TrailingVarArg)
        .arg(
            Arg::with_name(options::INPUT)
                .long(options::INPUT)
                .short(options::INPUT_SHORT)
                .help("adjust standard input stream buffering")
                .value_name("MODE")
                .required_unless_one(&[options::OUTPUT, options::ERROR]),
        )
        .arg(
            Arg::with_name(options::OUTPUT)
                .long(options::OUTPUT)
                .short(options::OUTPUT_SHORT)
                .help("adjust standard output stream buffering")
                .value_name("MODE")
                .required_unless_one(&[options::INPUT, options::ERROR]),
        )
        .arg(
            Arg::with_name(options::ERROR)
                .long(options::ERROR)
                .short(options::ERROR_SHORT)
                .help("adjust standard error stream buffering")
                .value_name("MODE")
                .required_unless_one(&[options::INPUT, options::OUTPUT]),
        )
        .arg(
            Arg::with_name(options::COMMAND)
                .multiple(true)
                .takes_value(true)
                .hidden(true)
                .required(true),
        )
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
