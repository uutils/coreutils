// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use clap::{Arg, ArgAction, Command};
use std::io::Write;
use uucore::translate;

// uucore::main does not support no-result
pub fn uumain(mut args: impl uucore::Args) -> i32 {
    // skip binary name
    let (Some(flag), None) = (args.nth(1), args.next()) else {
        return 1;
    };

    let error = if flag == "--help" {
        uu_app().print_help()
    } else if flag == "--version" {
        write!(std::io::stdout(), "{}", uu_app().render_version())
    } else {
        return 1;
    };

    if let Err(print_fail) = error
        && print_fail.kind() != std::io::ErrorKind::BrokenPipe
    {
        let _ = writeln!(std::io::stderr(), "{}: {print_fail}", uucore::util_name());
    }
    1
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .about(translate!("false-about"))
        // We provide our own help and version options, to ensure maximum compatibility with GNU.
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new("help")
                .long("help")
                .help(translate!("false-help-text"))
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new("version")
                .long("version")
                .help(translate!("false-version-text"))
                .action(ArgAction::Version),
        )
}
