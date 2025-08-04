// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (path) osrelease

//! Helper clap functions to localize error handling and options
//!

use crate::locale::translate;
use clap::error::{ContextKind, ErrorKind};
use clap::{ArgMatches, Command, Error};
use std::ffi::OsString;

/// Apply color to text using ANSI escape codes
pub fn colorize(text: &str, color_code: &str) -> String {
    format!("\x1b[{color_code}m{text}\x1b[0m")
}

/// Color constants for consistent styling
pub mod colors {
    pub const RED: &str = "31";
    pub const YELLOW: &str = "33";
    pub const GREEN: &str = "32";
}

pub fn handle_clap_error_with_exit_code(err: Error, util_name: &str, exit_code: i32) -> ! {
    // Try to ensure localization is initialized for this utility
    // If it's already initialized, that's fine - we'll use the existing one
    let _ = crate::locale::setup_localization_with_common(util_name);

    match err.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
            // For help and version, use clap's built-in formatting and exit with 0
            // Output to stdout as expected by tests
            print!("{}", err.render());
            std::process::exit(0);
        }
        ErrorKind::UnknownArgument => {
            // Use clap's rendering system but capture the output to check if colors are used
            let rendered = err.render();
            let rendered_str = rendered.to_string();

            // Simple check - if the rendered output contains ANSI escape codes, colors are enabled
            let colors_enabled = rendered_str.contains("\x1b[");

            if let Some(invalid_arg) = err.get(ContextKind::InvalidArg) {
                let arg_str = invalid_arg.to_string();

                // Get the uncolored words from common strings
                let error_word = translate!("common-error");
                let tip_word = translate!("common-tip");

                // Apply colors only if they're enabled in the original error
                let (colored_arg, colored_error_word, colored_tip_word) = if colors_enabled {
                    (
                        colorize(&arg_str, colors::YELLOW),
                        colorize(&error_word, colors::RED),
                        colorize(&tip_word, colors::GREEN),
                    )
                } else {
                    (arg_str.clone(), error_word.clone(), tip_word.clone())
                };

                // Print main error message
                let error_msg = translate!(
                    "clap-error-unexpected-argument",
                    "arg" => colored_arg.clone(),
                    "error_word" => colored_error_word
                );
                eprintln!("{error_msg}");
                eprintln!();

                // Show suggestion or generic tip
                let suggestion = err.get(ContextKind::SuggestedArg);
                if let Some(suggested_arg) = suggestion {
                    let colored_suggestion = if colors_enabled {
                        colorize(&suggested_arg.to_string(), colors::GREEN)
                    } else {
                        suggested_arg.to_string()
                    };
                    let suggestion_msg = translate!(
                        "clap-error-similar-argument",
                        "tip_word" => colored_tip_word,
                        "suggestion" => colored_suggestion
                    );
                    eprintln!("  {suggestion_msg}");
                } else {
                    let colored_tip_command = if colors_enabled {
                        colorize(&format!("-- {arg_str}"), colors::GREEN)
                    } else {
                        format!("-- {arg_str}")
                    };
                    let tip_msg = translate!(
                        "clap-error-pass-as-value",
                        "arg" => colored_arg,
                        "tip_word" => colored_tip_word,
                        "tip_command" => colored_tip_command
                    );
                    eprintln!("  {tip_msg}");
                }

                // Show usage and help
                eprintln!();
                let usage_label = translate!("common-usage");
                let usage_pattern = translate!(&format!("{util_name}-usage"));
                eprintln!("{usage_label}: {usage_pattern}");
                eprintln!();

                let help_msg = translate!("clap-error-help-suggestion", "command" => util_name);
                eprintln!("{help_msg}");

                std::process::exit(exit_code);
            } else {
                // Generic fallback case
                let rendered = err.render();
                let rendered_str = rendered.to_string();
                let colors_enabled = rendered_str.contains("\x1b[");

                let colored_error_word = if colors_enabled {
                    colorize(&translate!("common-error"), colors::RED)
                } else {
                    translate!("common-error")
                };
                eprintln!("{colored_error_word}: unexpected argument");
                std::process::exit(exit_code);
            }
        }
        _ => {
            // For other errors, print using clap's formatter but exit with code 1
            eprint!("{}", err.render());
            std::process::exit(1);
        }
    }
}

/// Trait extension to provide localized clap error handling
/// This provides a cleaner API than wrapping with macros
pub trait LocalizedCommand {
    /// Get matches with localized error handling
    fn get_matches_localized(self) -> ArgMatches
    where
        Self: Sized;

    /// Try to get matches from args with localized error handling
    fn try_get_matches_from_localized<I, T>(self, itr: I) -> ArgMatches
    where
        Self: Sized,
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone;

    /// Try to get matches from mutable args with localized error handling
    fn try_get_matches_from_mut_localized<I, T>(self, itr: I) -> ArgMatches
    where
        Self: Sized,
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone;
}

impl LocalizedCommand for Command {
    fn get_matches_localized(self) -> ArgMatches {
        self.try_get_matches()
            .unwrap_or_else(|err| handle_clap_error_with_exit_code(err, crate::util_name(), 1))
    }

    fn try_get_matches_from_localized<I, T>(self, itr: I) -> ArgMatches
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        self.try_get_matches_from(itr)
            .unwrap_or_else(|err| handle_clap_error_with_exit_code(err, crate::util_name(), 1))
    }

    fn try_get_matches_from_mut_localized<I, T>(mut self, itr: I) -> ArgMatches
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        self.try_get_matches_from_mut(itr)
            .unwrap_or_else(|err| handle_clap_error_with_exit_code(err, crate::util_name(), 1))
    }
}
