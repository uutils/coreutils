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

/// Determines if a clap error should show simple help instead of full usage
/// Based on clap's own design patterns and error categorization
fn should_show_simple_help_for_clap_error(kind: ErrorKind) -> bool {
    match kind {
        // Most validation errors should show simple help
        ErrorKind::InvalidValue
        | ErrorKind::InvalidSubcommand
        | ErrorKind::ValueValidation
        | ErrorKind::InvalidUtf8
        | ErrorKind::ArgumentConflict
        | ErrorKind::NoEquals => true,

        // Argument count and structural errors need special formatting
        ErrorKind::TooFewValues
        | ErrorKind::TooManyValues
        | ErrorKind::WrongNumberOfValues
        | ErrorKind::MissingSubcommand => false,

        // MissingRequiredArgument needs different handling
        ErrorKind::MissingRequiredArgument => false,

        // Special cases - handle their own display
        ErrorKind::DisplayHelp
        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        | ErrorKind::DisplayVersion => false,

        // UnknownArgument gets special handling elsewhere, so mark as false here
        ErrorKind::UnknownArgument => false,

        // System errors - keep simple
        ErrorKind::Io | ErrorKind::Format => true,

        // Default for any new ErrorKind variants - be conservative and show simple help
        _ => true,
    }
}

/// Color enum for consistent styling
#[derive(Debug, Clone, Copy)]
pub enum Color {
    Red,
    Yellow,
    Green,
}

impl Color {
    fn code(self) -> &'static str {
        match self {
            Color::Red => "31",
            Color::Yellow => "33",
            Color::Green => "32",
        }
    }
}

/// Apply color to text using ANSI escape codes
fn colorize(text: &str, color: Color) -> String {
    format!("\x1b[{}m{text}\x1b[0m", color.code())
}

/// Display usage information and help suggestion for errors that require it
/// This consolidates the shared logic between clap errors and UUsageError
pub fn display_usage_and_help(util_name: &str) {
    eprintln!();
    // Try to get usage information from localization
    let usage_key = format!("{}-usage", util_name);
    let usage_text = translate!(&usage_key);
    let formatted_usage = crate::format_usage(&usage_text);
    let usage_label = translate!("common-usage");
    eprintln!("{}: {}", usage_label, formatted_usage);
    eprintln!();
    let help_msg = translate!("clap-error-help-suggestion", "command" => crate::execution_phrase());
    eprintln!("{help_msg}");
}

pub fn handle_clap_error_with_exit_code(err: Error, util_name: &str, exit_code: i32) -> ! {
    // Ensure localization is initialized for this utility (always with common strings)
    let _ = crate::locale::setup_localization(util_name);

    // Check if colors are enabled by examining clap's rendered output
    let rendered_str = err.render().to_string();
    let colors_enabled = rendered_str.contains("\x1b[");

    // Helper function to conditionally colorize text
    let maybe_colorize = |text: &str, color: Color| -> String {
        if colors_enabled {
            colorize(text, color)
        } else {
            text.to_string()
        }
    };

    match err.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
            // For help and version, use clap's built-in formatting and exit with 0
            // Output to stdout as expected by tests
            print!("{}", err.render());
            std::process::exit(0);
        }
        ErrorKind::UnknownArgument => {
            // Force localization initialization - ignore any previous failures
            crate::locale::setup_localization(util_name).ok();

            // Choose exit code based on utility name
            let exit_code = match util_name {
                // These utilities expect exit code 2 for invalid options
                "ls" | "dir" | "vdir" | "sort" | "tty" | "printenv" => 2,
                // Most utilities expect exit code 1
                _ => 1,
            };

            // For UnknownArgument, we need to preserve clap's built-in tips (like using -- for values)
            // while still allowing localization of the main error message
            let rendered_str = err.render().to_string();
            let _lines: Vec<&str> = rendered_str.lines().collect();

            if let Some(invalid_arg) = err.get(ContextKind::InvalidArg) {
                let arg_str = invalid_arg.to_string();

                // Get localized error word with fallback
                let error_word = translate!("common-error");

                let colored_arg = maybe_colorize(&arg_str, Color::Yellow);
                let colored_error_word = maybe_colorize(&error_word, Color::Red);

                // Print main error message with fallback
                let error_msg = translate!(
                    "clap-error-unexpected-argument",
                    "arg" => colored_arg.clone(),
                    "error_word" => colored_error_word.clone()
                );
                eprintln!("{error_msg}");
                eprintln!();

                // Show suggestion if available
                let suggestion = err.get(ContextKind::SuggestedArg);
                if let Some(suggested_arg) = suggestion {
                    let tip_word = translate!("common-tip");
                    let colored_tip_word = maybe_colorize(&tip_word, Color::Green);
                    let colored_suggestion =
                        maybe_colorize(&suggested_arg.to_string(), Color::Green);
                    let suggestion_msg = translate!(
                        "clap-error-similar-argument",
                        "tip_word" => colored_tip_word.clone(),
                        "suggestion" => colored_suggestion.clone()
                    );
                    eprintln!("{suggestion_msg}");
                    eprintln!();
                } else {
                    // Look for other clap tips (like "-- --file-with-dash") that aren't suggestions
                    // These usually start with "  tip:" and contain useful information
                    for line in _lines.iter() {
                        if line.trim().starts_with("tip:") && !line.contains("similar argument") {
                            eprintln!("{}", line);
                            eprintln!();
                        }
                    }
                }

                // Show usage information for unknown arguments
                let usage_key = format!("{util_name}-usage");
                let usage_text = translate!(&usage_key);
                let formatted_usage = crate::format_usage(&usage_text);
                let usage_label = translate!("common-usage");
                eprintln!("{}: {}", usage_label, formatted_usage);
                eprintln!();
                eprintln!("{}", translate!("common-help-suggestion"));

                std::process::exit(exit_code);
            } else {
                // Generic fallback case
                let error_word = translate!("common-error");
                let colored_error_word = maybe_colorize(&error_word, Color::Red);
                eprintln!("{colored_error_word}: unexpected argument");
                std::process::exit(exit_code);
            }
        }
        // Check if this is a simple validation error that should show simple help
        kind if should_show_simple_help_for_clap_error(kind) => {
            // For simple validation errors, use the same simple format as other errors
            let lines: Vec<&str> = rendered_str.lines().collect();
            if let Some(main_error_line) = lines.first() {
                // Keep the "error: " prefix for test compatibility
                eprintln!("{}", main_error_line);
                eprintln!();
                // Use the execution phrase for the help suggestion to match test expectations
                eprintln!("{}", translate!("common-help-suggestion"));
            } else {
                // Fallback to original rendering if we can't parse
                eprint!("{}", err.render());
            }
            std::process::exit(exit_code);
        }
        _ => {
            // For MissingRequiredArgument, use the full clap error as it includes proper usage
            if matches!(err.kind(), ErrorKind::MissingRequiredArgument) {
                eprint!("{}", err.render());
                std::process::exit(exit_code);
            }

            // For TooFewValues and similar structural errors, use the full clap error
            if matches!(
                err.kind(),
                ErrorKind::TooFewValues | ErrorKind::TooManyValues | ErrorKind::WrongNumberOfValues
            ) {
                eprint!("{}", err.render());
                std::process::exit(exit_code);
            }

            // For other errors, show just the error and help suggestion
            let rendered_str = err.render().to_string();
            let lines: Vec<&str> = rendered_str.lines().collect();

            // Print error message (first line)
            if let Some(first_line) = lines.first() {
                eprintln!("{}", first_line);
            }

            // For other errors, just show help suggestion
            eprintln!();
            eprintln!("{}", translate!("common-help-suggestion"));

            std::process::exit(exit_code);
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

    /// Get matches from args with localized error handling
    fn get_matches_from_localized<I, T>(self, itr: I) -> ArgMatches
    where
        Self: Sized,
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone;

    /// Get matches from mutable args with localized error handling
    fn get_matches_from_mut_localized<I, T>(self, itr: I) -> ArgMatches
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

    fn get_matches_from_localized<I, T>(self, itr: I) -> ArgMatches
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        self.try_get_matches_from(itr)
            .unwrap_or_else(|err| handle_clap_error_with_exit_code(err, crate::util_name(), 1))
    }

    fn get_matches_from_mut_localized<I, T>(mut self, itr: I) -> ArgMatches
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        self.try_get_matches_from_mut(itr)
            .unwrap_or_else(|err| handle_clap_error_with_exit_code(err, crate::util_name(), 1))
    }
}
