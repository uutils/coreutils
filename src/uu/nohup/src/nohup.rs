// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) SIGHUP cproc vprocmgr homeout

use clap::{Arg, ArgAction, Command};
use std::env;
use std::fs::File;
use std::io::{Error, ErrorKind};
use std::process;
use std::sync::LazyLock;
use thiserror::Error;
use uucore::display::Quotable;
use uucore::error::{UError, UResult, set_exit_code, strip_errno};
use uucore::translate;
use uucore::{format_usage, show_error};

#[cfg(unix)]
#[path = "platform/unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "platform/windows.rs"]
mod platform;

static NOHUP_OUT: &str = "nohup.out";
// exit codes that match the GNU implementation
static EXIT_CANCELED: i32 = 125;
static EXIT_CANNOT_INVOKE: i32 = 126;
static EXIT_ENOENT: i32 = 127;
static POSIX_NOHUP_FAILURE: i32 = 127;

mod options {
    pub const CMD: &str = "cmd";
}

#[derive(Debug, Error)]
enum NohupError {
    #[error("{}", translate!("nohup-error-open-failed", "path" => NOHUP_OUT.quote(), "err" => _1))]
    OpenFailed(i32, #[source] Error),

    #[error("{}", translate!("nohup-error-open-failed-both", "first_path" => NOHUP_OUT.quote(), "first_err" => _1, "second_path" => _2.quote(), "second_err" => _3))]
    OpenFailed2(i32, #[source] Error, String, Error),
}

impl UError for NohupError {
    fn code(&self) -> i32 {
        match self {
            Self::OpenFailed(code, _) | Self::OpenFailed2(code, _, _, _) => *code,
        }
    }
}

static FAILURE_CODE: LazyLock<i32> = LazyLock::new(|| {
    if env::var_os("POSIXLY_CORRECT").is_some() {
        POSIX_NOHUP_FAILURE
    } else {
        EXIT_CANCELED
    }
});

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result_with_exit_code(
        uu_app(),
        args,
        *FAILURE_CODE,
    )?;

    platform::prepare()?;

    #[allow(clippy::unwrap_used, reason = "set as required by clap")]
    let mut cmd_iter = matches.get_many::<String>(options::CMD).unwrap();
    #[allow(clippy::unwrap_used, reason = "set as required by clap")]
    let cmd = cmd_iter.next().unwrap();
    let args: Vec<&String> = cmd_iter.collect();
    let mut command = process::Command::new(cmd);
    command.args(args);

    let Some(err) = platform::run(&mut command)? else {
        return Ok(());
    };

    show_error!(
        "{}",
        translate!("nohup-error-failed-to-run-command", "command" => cmd.quote(), "error" => strip_errno(&err))
    );

    match err.kind() {
        ErrorKind::NotFound => set_exit_code(EXIT_ENOENT),
        _ => set_exit_code(EXIT_CANNOT_INVOKE),
    }
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new("nohup")
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template("nohup"))
        .about(translate!("nohup-about"))
        .after_help(translate!("nohup-after-help"))
        .override_usage(format_usage(&translate!("nohup-usage")))
        .arg(
            Arg::new(options::CMD)
                .hide(true)
                .required(true)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::CommandName),
        )
        .trailing_var_arg(true)
        .infer_long_args(true)
}

/// Open the file the detached command's stdout is appended to: `nohup.out` in
/// the current directory, falling back to `$HOME/nohup.out`.
fn find_stdout() -> UResult<File> {
    try_open_nohup_file(NOHUP_OUT).or_else(|e1| {
        let Ok(home) = env::var("HOME") else {
            return Err(NohupError::OpenFailed(*FAILURE_CODE, e1).into());
        };

        let home_out = std::path::PathBuf::from(home).join(NOHUP_OUT);
        let home_out = home_out.to_str().unwrap();

        try_open_nohup_file(home_out).map_err(|e2| {
            NohupError::OpenFailed2(*FAILURE_CODE, e1, home_out.to_string(), e2).into()
        })
    })
}

fn try_open_nohup_file(path: &str) -> std::io::Result<File> {
    let mut opt = std::fs::OpenOptions::new();
    opt.create(true).append(true);
    platform::set_output_file_mode(&mut opt);
    let file = opt.open(path)?;

    show_error!(
        "{}",
        translate!("nohup-ignoring-input-appending-output", "path" => path.quote())
    );

    Ok(file)
}
