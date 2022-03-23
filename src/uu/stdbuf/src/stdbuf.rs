// * This file is part of the uutils coreutils package.
// *
// * (c) Dorota Kapturkiewicz <dokaptur@gmail.com>
// *
// * For the full copyright and license information, please view the LICENSE
// * file that was distributed with this source code.

// spell-checker:ignore (ToDO) tempdir dyld dylib dragonflybsd optgrps libstdbuf

#[macro_use]
extern crate uucore;

use clap::{crate_version, Arg, ArgMatches, Command};
use std::convert::{TryFrom, TryInto};
use std::fs::File;
use std::io::{self, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process;
use tempfile::tempdir;
use tempfile::TempDir;
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};
use uucore::parse_size::parse_size;
use uucore::{format_usage, InvalidEncodingHandling};

static ABOUT: &str =
    "Run COMMAND, with modified buffering operations for its standard streams.\n\n\
     Mandatory arguments to long options are mandatory for short options too.";
const USAGE: &str = "{} OPTION... COMMAND";
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
    pub const INPUT_SHORT: char = 'i';
    pub const OUTPUT: &str = "output";
    pub const OUTPUT_SHORT: char = 'o';
    pub const ERROR: &str = "error";
    pub const ERROR_SHORT: char = 'e';
    pub const COMMAND: &str = "command";
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

impl<'a> TryFrom<&ArgMatches> for ProgramOptions {
    type Error = ProgramOptionsError;

    fn try_from(matches: &ArgMatches) -> Result<Self, Self::Error> {
        Ok(Self {
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
                |m| {
                    Ok(BufferType::Size(m.try_into().map_err(|_| {
                        ProgramOptionsError(format!(
                            "invalid mode '{}': Value too large for defined data type",
                            x
                        ))
                    })?))
                },
            ),
        },
        None => Ok(BufferType::Default),
    }
}

fn set_command_env(command: &mut process::Command, buffer_name: &str, buffer_type: &BufferType) {
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

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args
        .collect_str(InvalidEncodingHandling::Ignore)
        .accept_any();

    let matches = uu_app().get_matches_from(args);

    let options = ProgramOptions::try_from(&matches).map_err(|e| UUsageError::new(125, e.0))?;

    let mut command_values = matches.values_of::<&str>(options::COMMAND).unwrap();
    let mut command = process::Command::new(command_values.next().unwrap());
    let command_params: Vec<&str> = command_values.collect();

    let mut tmp_dir = tempdir().unwrap();
    let (preload_env, libstdbuf) = get_preload_env(&mut tmp_dir).map_err_context(String::new)?;
    command.env(preload_env, libstdbuf);
    set_command_env(&mut command, "_STDBUF_I", &options.stdin);
    set_command_env(&mut command, "_STDBUF_O", &options.stdout);
    set_command_env(&mut command, "_STDBUF_E", &options.stderr);
    command.args(command_params);

    let mut process = command
        .spawn()
        .map_err_context(|| "failed to execute process".to_string())?;
    let status = process.wait().map_err_context(String::new)?;
    match status.code() {
        Some(i) => {
            if i == 0 {
                Ok(())
            } else {
                Err(i.into())
            }
        }
        None => Err(USimpleError::new(
            1,
            format!("process killed by signal {}", status.signal().unwrap()),
        )),
    }
}

pub fn uu_app<'a>() -> Command<'a> {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .after_help(LONG_HELP)
        .override_usage(format_usage(USAGE))
        .trailing_var_arg(true)
        .infer_long_args(true)
        .arg(
            Arg::new(options::INPUT)
                .long(options::INPUT)
                .short(options::INPUT_SHORT)
                .help("adjust standard input stream buffering")
                .value_name("MODE")
                .required_unless_present_any(&[options::OUTPUT, options::ERROR]),
        )
        .arg(
            Arg::new(options::OUTPUT)
                .long(options::OUTPUT)
                .short(options::OUTPUT_SHORT)
                .help("adjust standard output stream buffering")
                .value_name("MODE")
                .required_unless_present_any(&[options::INPUT, options::ERROR]),
        )
        .arg(
            Arg::new(options::ERROR)
                .long(options::ERROR)
                .short(options::ERROR_SHORT)
                .help("adjust standard error stream buffering")
                .value_name("MODE")
                .required_unless_present_any(&[options::INPUT, options::OUTPUT]),
        )
        .arg(
            Arg::new(options::COMMAND)
                .multiple_occurrences(true)
                .takes_value(true)
                .hide(true)
                .required(true),
        )
}
