// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::env;
use std::io::{IsTerminal, Write};

use clap::{Arg, ArgAction, Command};

use uucore::display::OsWrite;
use uucore::error::UResult;
use uucore::line_ending::LineEnding;
use uucore::quoting_style::{QuotingStyle, locale_aware_escape_name};
use uucore::{format_usage, translate};

static OPT_NULL: &str = "null";

static ARG_VARIABLES: &str = "variables";

#[uucore::main(no_signals)]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result_with_exit_code(uu_app(), args, 2)?;

    let variables: Vec<String> = matches
        .get_many::<String>(ARG_VARIABLES)
        .map(|v| v.map(ToString::to_string).collect())
        .unwrap_or_default();

    let separator = LineEnding::from_zero_flag(matches.get_flag(OPT_NULL));
    let quoting_style = quoting_style();

    if variables.is_empty() {
        print_all_env_vars(separator, quoting_style)?;
        return Ok(());
    }

    let mut error_found = false;
    for env_var in variables {
        // we silently ignore a=b as variable but we trigger an error
        if env_var.contains('=') {
            error_found = true;
            continue;
        }
        if let Some(var) = env::var_os(env_var) {
            let mut stdout = std::io::stdout().lock();
            if stdout.is_terminal() {
                stdout.write_all_os(&locale_aware_escape_name(&var, quoting_style))?;
            } else {
                stdout.write_all_os(&var)?;
            }
            write!(stdout, "{separator}")?;
        } else {
            error_found = true;
        }
    }

    if error_found { Err(1.into()) } else { Ok(()) }
}

fn quoting_style() -> QuotingStyle {
    match env::var("QUOTING_STYLE").as_deref() {
        Ok("literal") => QuotingStyle::Literal { show_control: true },
        Ok("shell") => QuotingStyle::SHELL,
        Ok("shell-always") => QuotingStyle::SHELL_QUOTE,
        Ok("shell-escape-always") | Ok("locale") => QuotingStyle::SHELL_ESCAPE_QUOTE,
        Ok("c") | Ok("clocale") => QuotingStyle::C_DOUBLE,
        Ok("c-maybe") | Ok("escape") => QuotingStyle::C_NO_QUOTES,
        _ => QuotingStyle::SHELL_ESCAPE,
    }
}

fn print_all_env_vars<T: std::fmt::Display>(
    line_ending: T,
    style: QuotingStyle,
) -> std::io::Result<()> {
    let mut stdout = std::io::stdout().lock();
    for (name, value) in env::vars_os() {
        stdout.write_all_os(&locale_aware_escape_name(&name, style))?;
        stdout.write_all(b"=")?;
        stdout.write_all_os(&locale_aware_escape_name(&value, style))?;
        write!(stdout, "{line_ending}")?;
    }
    Ok(())
}

pub fn uu_app() -> Command {
    let cmd = Command::new("printenv")
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
