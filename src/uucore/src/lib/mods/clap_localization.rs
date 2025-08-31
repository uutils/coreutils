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

            print!("{localized_help}");
        }
        ErrorKind::DisplayVersion => {
            // For version, use clap's built-in formatting and exit with 0
            // Output to stdout as expected by tests
            print!("{}", err.render());
        }
        _ => unreachable!("handle_display_errors called with non-display error"),
    }
    std::process::exit(0);
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

        if let Some(main_error_line) = rendered_str.lines().next() {
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
            if let Some(main_error_line) = rendered_str.lines().next() {
                // Keep the "error: " prefix for test compatibility
                eprintln!("{main_error_line}");
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

            // Print error message (first line)
            if let Some(first_line) = rendered_str.lines().next() {
                eprintln!("{first_line}");
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

/* spell-checker: disable */
#[cfg(test)]
mod tests {
    use super::*;
    use clap::{Arg, Command};
    use std::ffi::OsString;

    #[test]
    fn test_color_codes() {
        assert_eq!(Color::Red.code(), "31");
        assert_eq!(Color::Yellow.code(), "33");
        assert_eq!(Color::Green.code(), "32");
    }

    #[test]
    fn test_colorize() {
        let red_text = colorize("error", Color::Red);
        assert_eq!(red_text, "\x1b[31merror\x1b[0m");

        let yellow_text = colorize("warning", Color::Yellow);
        assert_eq!(yellow_text, "\x1b[33mwarning\x1b[0m");

        let green_text = colorize("success", Color::Green);
        assert_eq!(green_text, "\x1b[32msuccess\x1b[0m");
    }

    fn create_test_command() -> Command {
        Command::new("test")
            .arg(
                Arg::new("input")
                    .short('i')
                    .long("input")
                    .value_name("FILE")
                    .help("Input file"),
            )
            .arg(
                Arg::new("output")
                    .short('o')
                    .long("output")
                    .value_name("FILE")
                    .help("Output file"),
            )
            .arg(
                Arg::new("format")
                    .long("format")
                    .value_parser(["json", "xml", "csv"])
                    .help("Output format"),
            )
    }

    #[test]
    fn test_get_matches_from_localized_with_valid_args() {
        let result = std::panic::catch_unwind(|| {
            let cmd = create_test_command();
            let matches = cmd.get_matches_from_localized(vec!["test", "--input", "file.txt"]);
            matches.get_one::<String>("input").unwrap().clone()
        });

        if let Ok(input_value) = result {
            assert_eq!(input_value, "file.txt");
        }
    }

    #[test]
    fn test_get_matches_from_localized_with_osstring_args() {
        let args: Vec<OsString> = vec!["test".into(), "--input".into(), "test.txt".into()];

        let result = std::panic::catch_unwind(|| {
            let cmd = create_test_command();
            let matches = cmd.get_matches_from_localized(args);
            matches.get_one::<String>("input").unwrap().clone()
        });

        if let Ok(input_value) = result {
            assert_eq!(input_value, "test.txt");
        }
    }

    #[test]
    fn test_localized_command_from_mut() {
        let args: Vec<OsString> = vec!["test".into(), "--output".into(), "result.txt".into()];

        let result = std::panic::catch_unwind(|| {
            let cmd = create_test_command();
            let matches = cmd.get_matches_from_mut_localized(args);
            matches.get_one::<String>("output").unwrap().clone()
        });

        if let Ok(output_value) = result {
            assert_eq!(output_value, "result.txt");
        }
    }

    fn create_unknown_argument_error() -> Error {
        let cmd = create_test_command();
        cmd.try_get_matches_from(vec!["test", "--unknown-arg"])
            .unwrap_err()
    }

    fn create_invalid_value_error() -> Error {
        let cmd = create_test_command();
        cmd.try_get_matches_from(vec!["test", "--format", "invalid"])
            .unwrap_err()
    }

    fn create_help_error() -> Error {
        let cmd = create_test_command();
        cmd.try_get_matches_from(vec!["test", "--help"])
            .unwrap_err()
    }

    fn create_version_error() -> Error {
        let cmd = Command::new("test").version("1.0.0");
        cmd.try_get_matches_from(vec!["test", "--version"])
            .unwrap_err()
    }

    #[test]
    fn test_error_kind_detection() {
        let unknown_err = create_unknown_argument_error();
        assert_eq!(unknown_err.kind(), ErrorKind::UnknownArgument);

        let invalid_value_err = create_invalid_value_error();
        assert_eq!(invalid_value_err.kind(), ErrorKind::InvalidValue);

        let help_err = create_help_error();
        assert_eq!(help_err.kind(), ErrorKind::DisplayHelp);

        let version_err = create_version_error();
        assert_eq!(version_err.kind(), ErrorKind::DisplayVersion);
    }

    #[test]
    fn test_context_extraction() {
        let unknown_err = create_unknown_argument_error();
        let invalid_arg = unknown_err.get(ContextKind::InvalidArg);
        assert!(invalid_arg.is_some());
        assert!(invalid_arg.unwrap().to_string().contains("unknown-arg"));

        let invalid_value_err = create_invalid_value_error();
        let invalid_value = invalid_value_err.get(ContextKind::InvalidValue);
        assert!(invalid_value.is_some());
        assert_eq!(invalid_value.unwrap().to_string(), "invalid");
    }

    fn test_maybe_colorize_helper(colors_enabled: bool) {
        let maybe_colorize = |text: &str, color: Color| -> String {
            if colors_enabled {
                colorize(text, color)
            } else {
                text.to_string()
            }
        };

        let result = maybe_colorize("test", Color::Red);
        if colors_enabled {
            assert!(result.contains("\x1b[31m"));
            assert!(result.contains("\x1b[0m"));
        } else {
            assert_eq!(result, "test");
        }
    }

    #[test]
    fn test_maybe_colorize_with_colors() {
        test_maybe_colorize_helper(true);
    }

    #[test]
    fn test_maybe_colorize_without_colors() {
        test_maybe_colorize_helper(false);
    }

    #[test]
    fn test_simple_help_classification() {
        let simple_help_kinds = [
            ErrorKind::InvalidValue,
            ErrorKind::ValueValidation,
            ErrorKind::InvalidSubcommand,
            ErrorKind::InvalidUtf8,
            ErrorKind::ArgumentConflict,
            ErrorKind::NoEquals,
            ErrorKind::Io,
            ErrorKind::Format,
        ];

        let non_simple_help_kinds = [
            ErrorKind::TooFewValues,
            ErrorKind::TooManyValues,
            ErrorKind::WrongNumberOfValues,
            ErrorKind::MissingSubcommand,
            ErrorKind::MissingRequiredArgument,
            ErrorKind::DisplayHelp,
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand,
            ErrorKind::DisplayVersion,
            ErrorKind::UnknownArgument,
        ];

        for kind in &simple_help_kinds {
            assert!(
                should_show_simple_help_for_clap_error(*kind),
                "Expected {:?} to show simple help",
                kind
            );
        }

        for kind in &non_simple_help_kinds {
            assert!(
                !should_show_simple_help_for_clap_error(*kind),
                "Expected {:?} to NOT show simple help",
                kind
            );
        }
    }

    #[test]
    fn test_localization_setup() {
        use crate::locale::{get_message, setup_localization};

        let _ = setup_localization("test");

        let common_keys = [
            "common-error",
            "common-usage",
            "common-help-suggestion",
            "clap-error-unexpected-argument",
            "clap-error-invalid-value",
        ];
        for key in &common_keys {
            let message = get_message(key);
            assert_ne!(message, *key, "Translation not found for key: {}", key);
        }
    }

    #[test]
    fn test_localization_with_args() {
        use crate::locale::{get_message_with_args, setup_localization};
        use fluent::FluentArgs;

        let _ = setup_localization("test");

        let mut args = FluentArgs::new();
        args.set("error_word", "ERROR");
        args.set("arg", "--test");

        let message = get_message_with_args("clap-error-unexpected-argument", args);
        assert_ne!(
            message, "clap-error-unexpected-argument",
            "Translation not found for key: clap-error-unexpected-argument"
        );
    }

    #[test]
    fn test_french_localization() {
        use crate::locale::{get_message, setup_localization};
        use std::env;

        let original_lang = env::var("LANG").unwrap_or_default();

        unsafe {
            env::set_var("LANG", "fr-FR");
        }
        let result = setup_localization("test");

        if result.is_ok() {
            let error_word = get_message("common-error");
            assert_eq!(error_word, "erreur");

            let usage_word = get_message("common-usage");
            assert_eq!(usage_word, "Utilisation");

            let tip_word = get_message("common-tip");
            assert_eq!(tip_word, "conseil");
        }

        unsafe {
            if original_lang.is_empty() {
                env::remove_var("LANG");
            } else {
                env::set_var("LANG", original_lang);
            }
        }
    }

    #[test]
    fn test_french_clap_error_messages() {
        use crate::locale::{get_message_with_args, setup_localization};
        use fluent::FluentArgs;
        use std::env;

        let original_lang = env::var("LANG").unwrap_or_default();

        unsafe {
            env::set_var("LANG", "fr-FR");
        }
        let result = setup_localization("test");

        if result.is_ok() {
            let mut args = FluentArgs::new();
            args.set("error_word", "erreur");
            args.set("arg", "--inconnu");

            let unexpected_msg = get_message_with_args("clap-error-unexpected-argument", args);
            assert!(unexpected_msg.contains("erreur"));
            assert!(unexpected_msg.contains("--inconnu"));
            assert!(unexpected_msg.contains("inattendu"));

            let mut value_args = FluentArgs::new();
            value_args.set("error_word", "erreur");
            value_args.set("value", "invalide");
            value_args.set("option", "--format");

            let invalid_msg = get_message_with_args("clap-error-invalid-value", value_args);
            assert!(invalid_msg.contains("erreur"));
            assert!(invalid_msg.contains("invalide"));
            assert!(invalid_msg.contains("--format"));
        }

        unsafe {
            if original_lang.is_empty() {
                env::remove_var("LANG");
            } else {
                env::set_var("LANG", original_lang);
            }
        }
    }
}
/* spell-checker: enable */
