// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) ttyname filedesc

use clap::{Arg, ArgAction, Command};
use std::io::{IsTerminal, Write};
use uucore::error::{UResult, set_exit_code};
use uucore::format_usage;

use uucore::translate;

mod options {
    pub const SILENT: &str = "silent";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args).unwrap_or_else(|e| {
        use uucore::clap_localization::handle_clap_error_with_exit_code;
        handle_clap_error_with_exit_code(e, uucore::util_name(), 2)
    });

    let silent = matches.get_flag(options::SILENT);

    // If silent, we don't need the name, only whether or not stdin is a tty.
    if silent {
        return if std::io::stdin().is_terminal() {
            Ok(())
        } else {
            Err(1.into())
        };
    }

    let mut stdout = std::io::stdout();

    let name = nix::unistd::ttyname(std::io::stdin());

    let write_result = match name {
        Ok(name) => writeln!(stdout, "{}", name.display()),
        Err(_) => {
            set_exit_code(1);
            writeln!(stdout, "{}", translate!("tty-not-a-tty"))
        }
    };

    if write_result.is_err() || stdout.flush().is_err() {
        // Don't return to prevent a panic later when another flush is attempted
        // because the `uucore_procs::main` macro inserts a flush after execution for every utility.
        std::process::exit(3);
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(translate!("tty-about"))
        .override_usage(format_usage(&translate!("tty-usage")))
        .infer_long_args(true)
        .arg(
            Arg::new(options::SILENT)
                .long(options::SILENT)
                .visible_alias("quiet")
                .short('s')
                .help(translate!("tty-help-silent"))
                .action(ArgAction::SetTrue),
        )
}
