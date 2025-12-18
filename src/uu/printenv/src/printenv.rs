// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::{Arg, ArgAction, Command};
use std::env;
#[cfg(unix)]
use std::io::{self, Write};
use uucore::translate;
use uucore::{error::UResult, format_usage};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

static OPT_NULL: &str = "null";

static ARG_VARIABLES: &str = "variables";

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result_with_exit_code(uu_app(), args, 2)?;

    let variables: Vec<String> = matches
        .get_many::<String>(ARG_VARIABLES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let separator: &[u8] = if matches.get_flag(OPT_NULL) {
        b"\x00"
    } else {
        b"\n"
    };

    #[cfg(unix)]
    let mut stdout = io::stdout().lock();

    if variables.is_empty() {
        for (env_var, value) in env::vars_os() {
            #[cfg(unix)]
            {
                stdout.write_all(env_var.as_bytes())?;
                stdout.write_all(b"=")?;
                stdout.write_all(value.as_bytes())?;
                stdout.write_all(separator)?;
            }
            #[cfg(not(unix))]
            {
                // On non-Unix, use lossy conversion as OsStrExt is not available
                print!(
                    "{}={}{}",
                    env_var.to_string_lossy(),
                    value.to_string_lossy(),
                    String::from_utf8_lossy(separator)
                );
            }
        }
        return Ok(());
    }

    let mut error_found = false;
    for env_var in variables {
        // we silently ignore a=b as variable but we trigger an error
        if env_var.contains('=') {
            error_found = true;
            continue;
        }
        if let Some(var) = env::var_os(&env_var) {
            #[cfg(unix)]
            {
                stdout.write_all(var.as_bytes())?;
                stdout.write_all(separator)?;
            }
            #[cfg(not(unix))]
            {
                print!(
                    "{}{}",
                    var.to_string_lossy(),
                    String::from_utf8_lossy(separator)
                );
            }
        } else {
            error_found = true;
        }
    }

    if error_found { Err(1.into()) } else { Ok(()) }
}

pub fn uu_app() -> Command {
    let cmd = Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(translate!("printenv-about"))
        .override_usage(format_usage(&translate!("printenv-usage")))
        .infer_long_args(true);
    uucore::clap_localization::configure_localized_command(cmd)
        .arg(
            Arg::new(OPT_NULL)
                .short('0')
                .long(OPT_NULL)
                .help(translate!("printenv-help-null"))
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(ARG_VARIABLES)
                .action(ArgAction::Append)
                .num_args(1..),
        )
}
