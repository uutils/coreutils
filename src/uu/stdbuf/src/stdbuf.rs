// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) tempdir dyld dylib dragonflybsd optgrps libstdbuf

use clap::{crate_version, Arg, ArgAction, ArgMatches, Command};
use std::fs::File;
use std::io::{self, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process;
use tempfile::tempdir;
use tempfile::TempDir;
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};
use uucore::parse_size::parse_size_u64;
use uucore::{crash, format_usage, help_about, help_section, help_usage};

const ABOUT: &str = help_about!("stdbuf.md");
const USAGE: &str = help_usage!("stdbuf.md");
const LONG_HELP: &str = help_section!("after help", "stdbuf.md");

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

impl TryFrom<&ArgMatches> for ProgramOptions {
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
    target_os = "android",
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
    target_os = "android",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "dragonflybsd",
    target_vendor = "apple"
)))]
fn preload_strings() -> (&'static str, &'static str) {
    use uucore::crash;

    crash!(1, "Command not supported for this operating system!")
}

fn check_option(matches: &ArgMatches, name: &str) -> Result<BufferType, ProgramOptionsError> {
    match matches.get_one::<String>(name) {
        Some(value) => match value.as_str() {
            "L" => {
                if name == options::INPUT {
                    Err(ProgramOptionsError(
                        "line buffering stdin is meaningless".to_string(),
                    ))
                } else {
                    Ok(BufferType::Line)
                }
            }
            x => parse_size_u64(x).map_or_else(
                |e| crash!(125, "invalid mode {}", e),
                |m| {
                    Ok(BufferType::Size(m.try_into().map_err(|_| {
                        ProgramOptionsError(format!(
                            "invalid mode '{x}': Value too large for defined data type"
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

fn get_preload_env(tmp_dir: &TempDir) -> io::Result<(String, PathBuf)> {
    let (preload, extension) = preload_strings();
    let inject_path = tmp_dir.path().join("libstdbuf").with_extension(extension);

    let mut file = File::create(&inject_path)?;
    file.write_all(STDBUF_INJECT)?;

    Ok((preload.to_owned(), inject_path))
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args.collect_ignore();

    let matches = uu_app().try_get_matches_from(args)?;

    let options = ProgramOptions::try_from(&matches).map_err(|e| UUsageError::new(125, e.0))?;

    let mut command_values = matches.get_many::<String>(options::COMMAND).unwrap();
    let mut command = process::Command::new(command_values.next().unwrap());
    let command_params: Vec<&str> = command_values.map(|s| s.as_ref()).collect();

    let tmp_dir = tempdir().unwrap();
    let (preload_env, libstdbuf) = get_preload_env(&tmp_dir).map_err_context(String::new)?;
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

pub fn uu_app() -> Command {
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
                .required_unless_present_any([options::OUTPUT, options::ERROR]),
        )
        .arg(
            Arg::new(options::OUTPUT)
                .long(options::OUTPUT)
                .short(options::OUTPUT_SHORT)
                .help("adjust standard output stream buffering")
                .value_name("MODE")
                .required_unless_present_any([options::INPUT, options::ERROR]),
        )
        .arg(
            Arg::new(options::ERROR)
                .long(options::ERROR)
                .short(options::ERROR_SHORT)
                .help("adjust standard error stream buffering")
                .value_name("MODE")
                .required_unless_present_any([options::INPUT, options::OUTPUT]),
        )
        .arg(
            Arg::new(options::COMMAND)
                .action(ArgAction::Append)
                .hide(true)
                .required(true)
                .value_hint(clap::ValueHint::CommandName),
        )
}
