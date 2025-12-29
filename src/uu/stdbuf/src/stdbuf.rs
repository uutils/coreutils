// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) tempdir dyld dylib optgrps libstdbuf

use clap::{Arg, ArgAction, ArgMatches, Command};
use std::ffi::OsString;
use std::path::PathBuf;
use std::process;
use tempfile::TempDir;
use tempfile::tempdir;
use thiserror::Error;
use uucore::error::{FromIo, UResult, USimpleError, UUsageError};
use uucore::format_usage;
use uucore::parser::parse_size::parse_size_u64;
use uucore::translate;

mod options {
    pub const INPUT: &str = "input";
    pub const INPUT_SHORT: char = 'i';
    pub const OUTPUT: &str = "output";
    pub const OUTPUT_SHORT: char = 'o';
    pub const ERROR: &str = "error";
    pub const ERROR_SHORT: char = 'e';
    pub const COMMAND: &str = "command";
}

#[cfg(all(
    not(feature = "feat_external_libstdbuf"),
    any(
        target_os = "linux",
        target_os = "android",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly"
    )
))]
const STDBUF_INJECT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/libstdbuf.so"));

#[cfg(all(not(feature = "feat_external_libstdbuf"), target_vendor = "apple"))]
const STDBUF_INJECT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/libstdbuf.dylib"));

#[cfg(all(not(feature = "feat_external_libstdbuf"), target_os = "cygwin"))]
const STDBUF_INJECT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/libstdbuf.dll"));

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

#[derive(Debug, Error)]
enum ProgramOptionsError {
    #[error("{}", translate!("stdbuf-error-line-buffering-stdin-meaningless"))]
    LineBufferingStdinMeaningless,
    #[error("{}", translate!("stdbuf-error-invalid-mode", "error" => _0.clone()))]
    InvalidMode(String),
    #[error("{}", translate!("stdbuf-error-value-too-large", "value" => _0.clone()))]
    ValueTooLarge(String),
}

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
fn preload_strings() -> UResult<(&'static str, &'static str)> {
    Ok(("LD_PRELOAD", "so"))
}

#[cfg(target_vendor = "apple")]
fn preload_strings() -> UResult<(&'static str, &'static str)> {
    Ok(("DYLD_LIBRARY_PATH", "dylib"))
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly",
    target_vendor = "apple"
)))]
fn preload_strings() -> UResult<(&'static str, &'static str)> {
    Err(USimpleError::new(
        1,
        translate!("stdbuf-error-command-not-supported"),
    ))
}

fn check_option(matches: &ArgMatches, name: &str) -> Result<BufferType, ProgramOptionsError> {
    match matches.get_one::<String>(name) {
        Some(value) => match value.as_str() {
            "L" => {
                if name == options::INPUT {
                    Err(ProgramOptionsError::LineBufferingStdinMeaningless)
                } else {
                    Ok(BufferType::Line)
                }
            }
            x => parse_size_u64(x).map_or_else(
                |e| Err(ProgramOptionsError::InvalidMode(e.to_string())),
                |m| {
                    Ok(BufferType::Size(m.try_into().map_err(|_| {
                        ProgramOptionsError::ValueTooLarge(x.to_string())
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

#[cfg(not(feature = "feat_external_libstdbuf"))]
fn get_preload_env(tmp_dir: &TempDir) -> UResult<(String, PathBuf)> {
    use std::fs::File;
    use std::io::Write;

    let (preload, extension) = preload_strings()?;
    let inject_path = tmp_dir.path().join("libstdbuf").with_extension(extension);

    let mut file = File::create(&inject_path)?;
    file.write_all(STDBUF_INJECT)?;

    Ok((preload.to_owned(), inject_path))
}

#[cfg(feature = "feat_external_libstdbuf")]
fn get_preload_env(_tmp_dir: &TempDir) -> UResult<(String, PathBuf)> {
    let (preload, extension) = preload_strings()?;

    // Use the directory provided at compile time via LIBSTDBUF_DIR environment variable
    // This will fail to compile if LIBSTDBUF_DIR is not set, which is the desired behavior
    const LIBSTDBUF_DIR: &str = env!("LIBSTDBUF_DIR");
    let path_buf = PathBuf::from(LIBSTDBUF_DIR)
        .join("libstdbuf")
        .with_extension(extension);
    if path_buf.exists() {
        return Ok((preload.to_owned(), path_buf));
    }

    Err(USimpleError::new(
        1,
        translate!("stdbuf-error-external-libstdbuf-not-found", "path" => path_buf.display()),
    ))
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches =
        uucore::clap_localization::handle_clap_result_with_exit_code(uu_app(), args, 125)?;

    let options =
        ProgramOptions::try_from(&matches).map_err(|e| UUsageError::new(125, e.to_string()))?;

    let mut command_values = matches
        .get_many::<OsString>(options::COMMAND)
        .ok_or_else(|| UUsageError::new(125, "no command specified".to_string()))?;
    let Some(first_command) = command_values.next() else {
        return Err(UUsageError::new(125, "no command specified".to_string()));
    };
    let mut command = process::Command::new(first_command);
    let command_params: Vec<&OsString> = command_values.collect();

    let tmp_dir = tempdir()
        .map_err(|e| UUsageError::new(125, format!("failed to create temp directory: {e}")))?;
    let (preload_env, libstdbuf) = get_preload_env(&tmp_dir)?;
    command.env(preload_env, libstdbuf);
    set_command_env(&mut command, "_STDBUF_I", &options.stdin);
    set_command_env(&mut command, "_STDBUF_O", &options.stdout);
    set_command_env(&mut command, "_STDBUF_E", &options.stderr);
    command.args(command_params);

    let mut process = match command.spawn() {
        Ok(p) => p,
        Err(e) => {
            return match e.kind() {
                std::io::ErrorKind::PermissionDenied => Err(USimpleError::new(
                    126,
                    translate!("stdbuf-error-permission-denied"),
                )),
                std::io::ErrorKind::NotFound => Err(USimpleError::new(
                    127,
                    translate!("stdbuf-error-no-such-file"),
                )),
                _ => Err(USimpleError::new(
                    1,
                    translate!("stdbuf-error-failed-to-execute", "error" => e),
                )),
            };
        }
    };

    let status = process.wait().map_err_context(String::new)?;
    match status.code() {
        Some(i) => {
            if i == 0 {
                Ok(())
            } else {
                Err(i.into())
            }
        }
        None => {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                let signal_msg = status
                    .signal()
                    .map_or_else(|| "unknown".to_string(), |s| s.to_string());
                Err(USimpleError::new(
                    1,
                    translate!("stdbuf-error-killed-by-signal", "signal" => signal_msg),
                ))
            }
            #[cfg(not(unix))]
            {
                Err(USimpleError::new(
                    1,
                    "process terminated abnormally".to_string(),
                ))
            }
        }
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("stdbuf-about"))
        .after_help(translate!("stdbuf-after-help"))
        .override_usage(format_usage(&translate!("stdbuf-usage")))
        .trailing_var_arg(true)
        .infer_long_args(true)
        .arg(
            Arg::new(options::INPUT)
                .long(options::INPUT)
                .short(options::INPUT_SHORT)
                .help(translate!("stdbuf-help-input"))
                .value_name("MODE")
                .required_unless_present_any([options::OUTPUT, options::ERROR]),
        )
        .arg(
            Arg::new(options::OUTPUT)
                .long(options::OUTPUT)
                .short(options::OUTPUT_SHORT)
                .help(translate!("stdbuf-help-output"))
                .value_name("MODE")
                .required_unless_present_any([options::INPUT, options::ERROR]),
        )
        .arg(
            Arg::new(options::ERROR)
                .long(options::ERROR)
                .short(options::ERROR_SHORT)
                .help(translate!("stdbuf-help-error"))
                .value_name("MODE")
                .required_unless_present_any([options::INPUT, options::OUTPUT]),
        )
        .arg(
            Arg::new(options::COMMAND)
                .action(ArgAction::Append)
                .hide(true)
                .required(true)
                .value_hint(clap::ValueHint::CommandName)
                .value_parser(clap::value_parser!(OsString)),
        )
}
