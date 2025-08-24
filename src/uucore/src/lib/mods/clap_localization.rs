// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (path) osrelease

//! Helper clap functions to localize error handling and options
//!
//! This module provides utilities for handling clap errors with localization support.
//! It uses clap's error context API to extract structured information from errors
//! instead of parsing error strings, providing a more robust solution.
//!

use crate::locale::translate;

use clap::error::{ContextKind, ErrorKind};
use clap::{ArgMatches, Command, Error};

use std::error::Error as StdError;
use std::ffi::OsString;

/// Determines if a clap error should show simple help instead of full usage
/// Based on clap's own design patterns and error categorization
fn should_show_simple_help_for_clap_error(kind: ErrorKind) -> bool {
    match kind {
        // Show simple help
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

/// Handle DisplayHelp and DisplayVersion errors
fn handle_display_errors(err: Error) -> ! {
    match err.kind() {
        ErrorKind::DisplayHelp => {
            // For help messages, we use the localized help template
            // The template should already have the localized usage label,
            // but we also replace any remaining "Usage:" instances for fallback

            let help_text = err.render().to_string();

            // Replace any remaining "Usage:" with localized version as fallback
            let usage_label = translate!("common-usage");
            let localized_help = help_text.replace("Usage:", &format!("{usage_label}:"));

            print!("{}", localized_help);
            std::process::exit(0);
        }
        ErrorKind::DisplayVersion => {
            // For version, use clap's built-in formatting and exit with 0
            // Output to stdout as expected by tests
            print!("{}", err.render());
            std::process::exit(0);
        }
        _ => unreachable!("handle_display_errors called with non-display error"),
    }
}

/// Handle UnknownArgument errors with localization and suggestions
fn handle_unknown_argument_error(
    err: Error,
    util_name: &str,
    maybe_colorize: impl Fn(&str, Color) -> String,
) -> ! {
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
        if let Some(suggested_arg) = err.get(ContextKind::SuggestedArg) {
            let tip_word = translate!("common-tip");
            let colored_tip_word = maybe_colorize(&tip_word, Color::Green);
            let colored_suggestion = maybe_colorize(&suggested_arg.to_string(), Color::Green);
            let suggestion_msg = translate!(
                "clap-error-similar-argument",
                "tip_word" => colored_tip_word.clone(),
                "suggestion" => colored_suggestion.clone()
            );
            eprintln!("{suggestion_msg}");
            eprintln!();
        } else {
            // For UnknownArgument, we need to preserve clap's built-in tips (like using -- for values)
            // while still allowing localization of the main error message
            let rendered_str = err.render().to_string();

            // Look for other clap tips (like "-- --file-with-dash") that aren't suggestions
            // These usually start with "  tip:" and contain useful information
            for line in rendered_str.lines() {
                if line.trim_start().starts_with("tip:") && !line.contains("similar argument") {
                    eprintln!("{line}");
                    eprintln!();
                }
            }
        }

        // Show usage information for unknown arguments
        let usage_key = format!("{util_name}-usage");
        let usage_text = translate!(&usage_key);
        let formatted_usage = crate::format_usage(&usage_text);
        let usage_label = translate!("common-usage");
        eprintln!("{usage_label}: {formatted_usage}");
        eprintln!();
        eprintln!("{}", translate!("common-help-suggestion"));
    } else {
        // Generic fallback case
        let error_word = translate!("common-error");
        let colored_error_word = maybe_colorize(&error_word, Color::Red);
        eprintln!("{colored_error_word}: unexpected argument");
    }
    // Choose exit code based on utility name
    let exit_code = match util_name {
        // These utilities expect exit code 2 for invalid options
        "ls" | "dir" | "vdir" | "sort" | "tty" | "printenv" => 2,
        // Most utilities expect exit code 1
        _ => 1,
    };

    std::process::exit(exit_code);
}

/// Handle InvalidValue and ValueValidation errors with localization
fn handle_invalid_value_error(err: Error, maybe_colorize: impl Fn(&str, Color) -> String) -> ! {
    // Extract value and option from error context using clap's context API
    // This is much more robust than parsing the error string
    let invalid_arg = err.get(ContextKind::InvalidArg);
    let invalid_value = err.get(ContextKind::InvalidValue);

    if let (Some(arg), Some(value)) = (invalid_arg, invalid_value) {
        let option = arg.to_string();
        let value = value.to_string();

        // Check if this is actually a missing value (empty string)
        if value.is_empty() {
            // This is the case where no value was provided for an option that requires one
            let error_word = translate!("common-error");
            eprintln!(
                "{}",
                translate!("clap-error-value-required", "error_word" => error_word, "option" => option)
            );
        } else {
            // Get localized error word and prepare message components outside conditionals
            let error_word = translate!("common-error");
            let colored_error_word = maybe_colorize(&error_word, Color::Red);
            let colored_value = maybe_colorize(&value, Color::Yellow);
            let colored_option = maybe_colorize(&option, Color::Green);

            let error_msg = translate!(
                "clap-error-invalid-value",
                "error_word" => colored_error_word,
                "value" => colored_value,
                "option" => colored_option
            );

            // For ValueValidation errors, include the validation error in the message
            match err.source() {
                Some(source) if matches!(err.kind(), ErrorKind::ValueValidation) => {
                    eprintln!("{error_msg}: {source}");
                }
                _ => eprintln!("{error_msg}"),
            }
        }

        // For ValueValidation errors, include the validation error details
        // Note: We don't print these separately anymore as they're part of the main message

        // Show possible values if available (for InvalidValue errors)
        if matches!(err.kind(), ErrorKind::InvalidValue) {
            if let Some(valid_values) = err.get(ContextKind::ValidValue) {
                if !valid_values.to_string().is_empty() {
                    // Don't show possible values if they are empty
                    eprintln!();
                    let possible_values_label = translate!("clap-error-possible-values");
                    eprintln!("  [{possible_values_label}: {valid_values}]");
                }
            }
        }

        eprintln!();
        eprintln!("{}", translate!("common-help-suggestion"));
    } else {
        // Fallback if we can't extract context - use clap's default formatting
        let rendered_str = err.render().to_string();
        let lines: Vec<&str> = rendered_str.lines().collect();
        if let Some(main_error_line) = lines.first() {
            eprintln!("{main_error_line}");
            eprintln!();
            eprintln!("{}", translate!("common-help-suggestion"));
        } else {
            eprint!("{}", err.render());
        }
    }
    std::process::exit(1);
}

pub fn handle_clap_error_with_exit_code(err: Error, util_name: &str, exit_code: i32) -> ! {
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
            handle_display_errors(err);
        }
        ErrorKind::UnknownArgument => {
            handle_unknown_argument_error(err, util_name, maybe_colorize);
        }
        // Check if this is a simple validation error that should show simple help
        kind if should_show_simple_help_for_clap_error(kind) => {
            // Special handling for InvalidValue and ValueValidation to provide localized error
            if matches!(kind, ErrorKind::InvalidValue | ErrorKind::ValueValidation) {
                handle_invalid_value_error(err, maybe_colorize);
            }

            // For other simple validation errors, use the same simple format as other errors
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

            // InvalidValue errors should exit with code 1 for all utilities
            let actual_exit_code = if matches!(kind, ErrorKind::InvalidValue) {
                1
            } else {
                exit_code
            };

            std::process::exit(actual_exit_code);
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
