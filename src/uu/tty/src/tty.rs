// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) ttyname filedesc

use clap::{Arg, ArgAction, Command};
use std::io::{IsTerminal, Write};
use uucore::display::OsWrite;
use uucore::error::{UResult, set_exit_code};
use uucore::format_usage;

use uucore::translate;

mod options {
    pub const SILENT: &str = "silent";
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uucore::clap_localization::handle_clap_result_with_exit_code(uu_app(), args, 2)?;

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
        Ok(name) => {
            if let Err(e) = stdout.write_all_os(name.as_os_str()) {
                Err(e)
            } else if let Err(e) = writeln!(stdout) {
                Err(e)
            } else {
                stdout.flush()
            }
        }
        Err(_) => {
            set_exit_code(1);
            writeln!(stdout, "{}", translate!("tty-not-a-tty"))
        }
    };

    if let Err(e) = write_result {
        eprintln!("tty: write error: {}", e);
        std::process::exit(3);
    }

    if let Err(e) = stdout.flush() {
        eprintln!("tty: write error: {}", e);
        std::process::exit(3);
    }

    Ok(())
}

pub fn uu_app() -> Command {
    let cmd = Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .about(translate!("tty-about"))
        .override_usage(format_usage(&translate!("tty-usage")))
        .infer_long_args(true);
    uucore::clap_localization::configure_localized_command(cmd).arg(
        Arg::new(options::SILENT)
            .long(options::SILENT)
            .visible_alias("quiet")
            .short('s')
            .help(translate!("tty-help-silent"))
            .action(ArgAction::SetTrue),
    )
}
