// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use clap::{Arg, ArgAction, Command};
use std::{
    ffi::OsString,
    io::{IsTerminal, Write},
};
use uucore::error::{UResult, set_exit_code};

use uucore::translate;

/// Create a localized help template for true/false commands which starts with Usage:
/// This ensures the format matches GNU coreutils where Usage: appears first
pub fn true_false_help_template(util_name: &str) -> clap::builder::StyledStr {
    // Determine if colors should be enabled - same logic as configure_localized_command
    let colors_enabled = if std::env::var("NO_COLOR").is_ok() {
        false
    } else if std::env::var("CLICOLOR_FORCE").is_ok() || std::env::var("FORCE_COLOR").is_ok() {
        true
    } else {
        IsTerminal::is_terminal(&std::io::stdout())
            && std::env::var("TERM").unwrap_or_default() != "dumb"
    };

    true_false_help_template_with_colors(util_name, colors_enabled)
}

/// Create a localized help template for true/false commands with explicit color control
pub fn true_false_help_template_with_colors(
    util_name: &str,
    colors_enabled: bool,
) -> clap::builder::StyledStr {
    use std::fmt::Write;

    // Ensure localization is initialized for this utility
    let _ = uucore::locale::setup_localization(util_name);

    // Get the localized "Usage" label
    let usage_label = uucore::locale::translate!("common-usage");

    // Create a styled template
    let mut template = clap::builder::StyledStr::new();

    // Add the basic template parts
    write!(template, "{{before-help}}").unwrap();

    // Add styled usage header (bold + underline like clap's default)
    if colors_enabled {
        write!(
            template,
            "\x1b[1m\x1b[4m{usage_label}:\x1b[0m {{usage}}\n\n"
        )
        .unwrap();
    } else {
        write!(template, "{usage_label}: {{usage}}\n\n").unwrap();
    }

    writeln!(template, "{{about-with-newline}}").unwrap();

    // Add the rest
    write!(template, "{{all-args}}{{after-help}}").unwrap();

    template
}

#[uucore::main]
// TODO: modify proc macro to allow no-result uumain
#[expect(clippy::unnecessary_wraps, reason = "proc macro requires UResult")]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args: Vec<OsString> = args.collect();
    if args.len() != 2 {
        return Ok(());
    }

    // args[0] is the name of the binary.
    let error = if args[1] == "--help" {
        uu_app().print_help()
    } else if args[1] == "--version" {
        write!(std::io::stdout(), "{}", uu_app().render_version())
    } else {
        Ok(())
    };

    if let Err(print_fail) = error {
        // Try to display this error.
        let _ = writeln!(std::io::stderr(), "{}: {print_fail}", uucore::util_name());
        // Mirror GNU options. When failing to print warnings or version flags, then we exit
        // with FAIL. This avoids allocation some error information which may result in yet
        // other types of failure.
        set_exit_code(1);
    }

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .override_usage(uucore::util_name().to_string())
        .help_template(true_false_help_template(uucore::util_name()))
        .about(translate!("true-about"))
        // We provide our own help and version options, to ensure maximum compatibility with GNU.
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new("help")
                .long("help")
                .help(translate!("true-help-text"))
                .action(ArgAction::Help),
        )
        .arg(
            Arg::new("version")
                .long("version")
                .help(translate!("true-version-text"))
                .action(ArgAction::Version),
        )
}
