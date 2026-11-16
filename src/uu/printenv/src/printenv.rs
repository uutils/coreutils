// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::env;
use std::io::{IsTerminal, Write};

use clap::{Arg, ArgAction, Command};

use uucore::display::{OsWrite, print_all_env_vars, print_all_env_vars_quoted};
use uucore::error::UResult;
use uucore::i18n::get_ctype_encoding;
use uucore::line_ending::LineEnding;
use uucore::quoting_style::{
    QuotingStyle, escape_name, parser as qs_parser, quoting_style_from_env,
};
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

    let maybe_quoting_style = matches
        .get_one::<String>(qs_parser::QUOTING_STYLE)
        .and_then(QuotingStyle::parse)
        .or_else(|| {
            std::io::stdout()
                .is_terminal() // quoting style from env var is only used in TTY
                .then(quoting_style_from_env)
                .flatten()
        });

    let separator = LineEnding::from_zero_flag(matches.get_flag(OPT_NULL));

    if variables.is_empty() {
        if let Some(quoting_style) = maybe_quoting_style {
            print_all_env_vars_quoted(
                std::io::stdout(),
                separator,
                quoting_style,
                get_ctype_encoding(),
            )?;
        } else {
            print_all_env_vars(std::io::stdout(), separator)?;
        }
        return Ok(());
    }

    let mut error_found = false;
    let ctype = get_ctype_encoding();
    for env_var in variables {
        // we silently ignore a=b as variable but we trigger an error
        if env_var.contains('=') {
            error_found = true;
            continue;
        }
        if let Some(var) = env::var_os(env_var) {
            let mut stdout = std::io::stdout().lock();

            let var = if let Some(qs) = maybe_quoting_style {
                escape_name(&var, qs, ctype)
            } else {
                var
            };

            stdout.write_all_os(&var)?;
            write!(stdout, "{separator}")?;
        } else {
            error_found = true;
        }
    }

    if error_found { Err(1.into()) } else { Ok(()) }
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
        .arg(qs_parser::clap_quoting_argument())
        .arg(
            Arg::new(ARG_VARIABLES)
                .action(ArgAction::Append)
                .num_args(1..),
        )
}
