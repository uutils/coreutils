// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore (path) osrelease myutil

//! Helper clap functions to localize error handling and options
//!
//! This module provides utilities for handling clap errors with localization support.
//! It uses clap's error context API to extract structured information from errors
//! instead of parsing error strings, providing a more robust solution.
//!

use crate::error::{UResult, USimpleError};
use crate::locale::translate;

use clap::error::{ContextKind, ErrorKind};
use clap::{ArgMatches, Command, Error};

use std::error::Error as StdError;
use std::ffi::OsString;

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
            Self::Red => "31",
            Self::Yellow => "33",
            Self::Green => "32",
        }
    }
}

/// Determine color choice based on environment variables
fn get_color_choice() -> clap::ColorChoice {
    if std::env::var("NO_COLOR").is_ok() {
        clap::ColorChoice::Never
    } else if std::env::var("CLICOLOR_FORCE").is_ok() || std::env::var("FORCE_COLOR").is_ok() {
        clap::ColorChoice::Always
    } else {
        clap::ColorChoice::Auto
    }
}

/// Generic helper to check if colors should be enabled for a given stream
fn should_use_color_for_stream<S: std::io::IsTerminal>(stream: &S) -> bool {
    match get_color_choice() {
        clap::ColorChoice::Always => true,
        clap::ColorChoice::Never => false,
        clap::ColorChoice::Auto => {
            stream.is_terminal() && std::env::var("TERM").unwrap_or_default() != "dumb"
        }
    }
}

/// Manages color output based on environment settings
struct ColorManager(bool);

impl ColorManager {
    /// Create a new ColorManager based on environment variables
    fn from_env() -> Self {
        Self(should_use_color_for_stream(&std::io::stderr()))
    }

    /// Apply color to text if colors are enabled
    fn colorize(&self, text: &str, color: Color) -> String {
        if self.0 {
            format!("\x1b[{}m{text}\x1b[0m", color.code())
        } else {
            text.to_string()
        }
    }
}

/// Unified error formatter that handles all error types consistently
pub struct ErrorFormatter<'a> {
    color_mgr: ColorManager,
    util_name: &'a str,
}

impl<'a> ErrorFormatter<'a> {
    pub fn new(util_name: &'a str) -> Self {
        Self {
            color_mgr: ColorManager::from_env(),
            util_name,
        }
    }

    /// Print error and exit with the specified code
    fn print_error_and_exit(&self, err: &Error, exit_code: i32) -> ! {
        self.print_error_and_exit_with_callback(err, exit_code, || {})
    }

    /// Print error with optional callback before exit
    pub fn print_error_and_exit_with_callback<F>(
        &self,
        err: &Error,
        exit_code: i32,
        callback: F,
    ) -> !
    where
        F: FnOnce(),
    {
        let code = self.print_error(err, exit_code);
        callback();
        std::process::exit(code);
    }

    /// Print error and return exit code (no exit call)
    pub fn print_error(&self, err: &Error, exit_code: i32) -> i32 {
        match err.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => self.handle_display_errors(err),
            ErrorKind::UnknownArgument => self.handle_unknown_argument(err, exit_code),
            ErrorKind::InvalidValue | ErrorKind::ValueValidation => {
                self.handle_invalid_value(err, exit_code)
            }
            ErrorKind::MissingRequiredArgument => self.handle_missing_required(err, exit_code),
            ErrorKind::TooFewValues | ErrorKind::TooManyValues | ErrorKind::WrongNumberOfValues => {
                // These need full clap formatting
                eprint!("{}", err.render());
                exit_code
            }
            _ => self.handle_generic_error(err, exit_code),
        }
    }

    /// Handle help and version display
    fn handle_display_errors(&self, err: &Error) -> i32 {
        print!("{}", err.render());
        0
    }

    /// Handle unknown argument errors
    fn handle_unknown_argument(&self, err: &Error, exit_code: i32) -> i32 {
        if let Some(invalid_arg) = err.get(ContextKind::InvalidArg) {
            let arg_str = invalid_arg.to_string();
            let error_word = translate!("common-error");

            // Print main error
            eprintln!(
                "{}",
                translate!(
                    "clap-error-unexpected-argument",
                    "arg" => self.color_mgr.colorize(&arg_str, Color::Yellow),
                    "error_word" => self.color_mgr.colorize(&error_word, Color::Red)
                )
            );
            eprintln!();

            // Show suggestion if available
            if let Some(suggested_arg) = err.get(ContextKind::SuggestedArg) {
                let tip_word = translate!("common-tip");
                eprintln!(
                    "{}",
                    translate!(
                        "clap-error-similar-argument",
                        "tip_word" => self.color_mgr.colorize(&tip_word, Color::Green),
                        "suggestion" => self.color_mgr.colorize(&suggested_arg.to_string(), Color::Green)
                    )
                );
                eprintln!();
            } else {
                // Look for other tips from clap
                self.print_clap_tips(err);
            }

            self.print_usage_and_help();
        } else {
            self.print_simple_error_msg(&translate!("clap-error-unexpected-argument-simple"));
        }
        exit_code
    }

    /// Handle invalid value errors
    fn handle_invalid_value(&self, err: &Error, exit_code: i32) -> i32 {
        let invalid_arg = err.get(ContextKind::InvalidArg);
        let invalid_value = err.get(ContextKind::InvalidValue);

        if let (Some(arg), Some(value)) = (invalid_arg, invalid_value) {
            let option = arg.to_string();
            let value = value.to_string();

            if value.is_empty() {
                // Value required but not provided
                let error_word = translate!("common-error");
                eprintln!(
                    "{}",
                    translate!("clap-error-value-required",
                        "error_word" => self.color_mgr.colorize(&error_word, Color::Red),
                        "option" => self.color_mgr.colorize(&option, Color::Green))
                );
            } else {
                // Invalid value provided
                let error_word = translate!("common-error");
                let error_msg = translate!(
                    "clap-error-invalid-value",
                    "error_word" => self.color_mgr.colorize(&error_word, Color::Red),
                    "value" => self.color_mgr.colorize(&value, Color::Yellow),
                    "option" => self.color_mgr.colorize(&option, Color::Green)
                );
                // Include validation error if present
                match err.source() {
                    Some(source) if matches!(err.kind(), ErrorKind::ValueValidation) => {
                        eprintln!("{error_msg}: {source}");
                    }
                    _ => eprintln!("{error_msg}"),
                }
            }

            // Show possible values for InvalidValue errors
            if matches!(err.kind(), ErrorKind::InvalidValue) {
                if let Some(valid_values) = err.get(ContextKind::ValidValue) {
                    if !valid_values.to_string().is_empty() {
                        eprintln!();
                        eprintln!(
                            "  [{}: {valid_values}]",
                            translate!("clap-error-possible-values")
                        );
                    }
                }
            }

            eprintln!();
            eprintln!("{}", translate!("common-help-suggestion"));
        } else {
            self.print_simple_error_msg(&err.render().to_string());
        }

        // InvalidValue errors traditionally use exit code 1 for backward compatibility
        // But if a utility explicitly requests a high exit code (>= 125), respect it
        // This allows utilities like runcon (125) to override the default while preserving
        // the standard behavior for utilities using normal error codes (1, 2, etc.)
        if matches!(err.kind(), ErrorKind::InvalidValue) && exit_code < 125 {
            1 // Force exit code 1 for InvalidValue unless using special exit codes
        } else {
            exit_code // Respect the requested exit code for special cases
        }
    }

    /// Handle missing required argument errors
    fn handle_missing_required(&self, err: &Error, exit_code: i32) -> i32 {
        let rendered_str = err.render().to_string();
        let lines: Vec<&str> = rendered_str.lines().collect();

        match lines.first() {
            Some(first_line)
                if first_line
                    .starts_with("error: the following required arguments were not provided:") =>
            {
                let error_word = translate!("common-error");
                eprintln!(
                    "{}",
                    translate!(
                        "clap-error-missing-required-arguments",
                        "error_word" => self.color_mgr.colorize(&error_word, Color::Red)
                    )
                );

                // Print the missing arguments
                for line in lines.iter().skip(1) {
                    if line.starts_with("  ") {
                        eprintln!("{line}");
                    } else if line.starts_with("Usage:") || line.starts_with("For more information")
                    {
                        break;
                    }
                }
                eprintln!();

                // Print usage
                lines
                    .iter()
                    .skip_while(|line| !line.starts_with("Usage:"))
                    .for_each(|line| {
                        if line.starts_with("For more information, try '--help'.") {
                            eprintln!("{}", translate!("common-help-suggestion"));
                        } else {
                            eprintln!("{line}");
                        }
                    });
            }
            _ => eprint!("{}", err.render()),
        }
        exit_code
    }

    /// Handle generic errors
    fn handle_generic_error(&self, err: &Error, exit_code: i32) -> i32 {
        let rendered_str = err.render().to_string();
        if let Some(main_error_line) = rendered_str.lines().next() {
            self.print_localized_error_line(main_error_line);
            eprintln!();
            eprintln!("{}", translate!("common-help-suggestion"));
        } else {
            eprint!("{}", err.render());
        }
        exit_code
    }

    /// Print a simple error message (no exit)
    fn print_simple_error_msg(&self, message: &str) {
        let error_word = translate!("common-error");
        eprintln!(
            "{}: {message}",
            self.color_mgr.colorize(&error_word, Color::Red)
        );
    }

    /// Print error line with localized "error:" prefix
    fn print_localized_error_line(&self, line: &str) {
        let error_word = translate!("common-error");
        let colored_error = self.color_mgr.colorize(&error_word, Color::Red);

        if let Some(colon_pos) = line.find(':') {
            let after_colon = &line[colon_pos..];
            eprintln!("{colored_error}{after_colon}");
        } else {
            eprintln!("{line}");
        }
    }

    /// Extract and print clap's built-in tips
    fn print_clap_tips(&self, err: &Error) {
        let rendered_str = err.render().to_string();
        for line in rendered_str.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("tip:") && !line.contains("similar argument") {
                let tip_word = translate!("common-tip");
                if let Some(colon_pos) = trimmed.find(':') {
                    let after_colon = &trimmed[colon_pos..];
                    eprintln!(
                        "  {}{after_colon}",
                        self.color_mgr.colorize(&tip_word, Color::Green)
                    );
                } else {
                    eprintln!("{line}");
                }
                eprintln!();
            }
        }
    }

    /// Print usage information and help suggestion
    fn print_usage_and_help(&self) {
        let usage_key = format!("{}-usage", self.util_name);
        let usage_text = translate!(&usage_key);
        let formatted_usage = crate::format_usage(&usage_text);
        let usage_label = translate!("common-usage");
        eprintln!("{usage_label}: {formatted_usage}");
        eprintln!();
        eprintln!("{}", translate!("common-help-suggestion"));
    }
}

/// Handles clap command parsing results with proper localization support.
///
/// This is the main entry point for processing command-line arguments with localized error messages.
/// It parses the provided arguments and returns either the parsed matches or handles errors with
/// localized messages.
///
/// # Arguments
///
/// * `cmd` - The clap `Command` to parse arguments against
/// * `itr` - An iterator of command-line arguments to parse
///
/// # Returns
///
/// * `Ok(ArgMatches)` - Successfully parsed command-line arguments
/// * `Err` - For help/version display (preserves original styling)
///
/// # Examples
///
/// ```no_run
/// use clap::Command;
/// use uucore::clap_localization::handle_clap_result;
///
/// let cmd = Command::new("myutil");
/// let args = vec!["myutil", "--help"];
/// let result = handle_clap_result(cmd, args);
/// ```
pub fn handle_clap_result<I, T>(cmd: Command, itr: I) -> UResult<ArgMatches>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    handle_clap_result_with_exit_code(cmd, itr, 1)
}

/// Handles clap command parsing with a custom exit code for errors.
///
/// Similar to `handle_clap_result` but allows specifying a custom exit code
/// for error conditions. This is useful for utilities that need specific
/// exit codes for different error types.
///
/// # Arguments
///
/// * `cmd` - The clap `Command` to parse arguments against
/// * `itr` - An iterator of command-line arguments to parse
/// * `exit_code` - The exit code to use when exiting due to an error
///
/// # Returns
///
/// * `Ok(ArgMatches)` - Successfully parsed command-line arguments
/// * `Err` - For help/version display (preserves original styling)
///
/// # Exit Behavior
///
/// This function will call `std::process::exit()` with the specified exit code
/// when encountering parsing errors (except help/version which use exit code 0).
///
/// # Examples
///
/// ```no_run
/// use clap::Command;
/// use uucore::clap_localization::handle_clap_result_with_exit_code;
///
/// let cmd = Command::new("myutil");
/// let args = vec!["myutil", "--invalid"];
/// let result = handle_clap_result_with_exit_code(cmd, args, 125);
/// ```
pub fn handle_clap_result_with_exit_code<I, T>(
    cmd: Command,
    itr: I,
    exit_code: i32,
) -> UResult<ArgMatches>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    cmd.try_get_matches_from(itr).map_err(|e| {
        if e.exit_code() == 0 {
            e.into() // Preserve help/version
        } else {
            let formatter = ErrorFormatter::new(crate::util_name());
            let code = formatter.print_error(&e, exit_code);
            USimpleError::new(code, "")
        }
    })
}

/// Handles a clap error directly with a custom exit code.
///
/// This function processes a clap error and exits the program with the specified
/// exit code. It formats error messages with proper localization and color support
/// based on environment variables.
///
/// # Arguments
///
/// * `err` - The clap `Error` to handle
/// * `exit_code` - The exit code to use when exiting
///
/// # Panics
///
/// This function never returns - it always calls `std::process::exit()`.
///
/// # Examples
///
/// ```no_run
/// use clap::Command;
/// use uucore::clap_localization::handle_clap_error_with_exit_code;
///
/// let cmd = Command::new("myutil");
/// match cmd.try_get_matches() {
///     Ok(matches) => { /* handle matches */ },
///     Err(e) => handle_clap_error_with_exit_code(e, 1),
/// }
/// ```
pub fn handle_clap_error_with_exit_code(err: Error, exit_code: i32) -> ! {
    let formatter = ErrorFormatter::new(crate::util_name());
    formatter.print_error_and_exit(&err, exit_code);
}

/// Configures a clap `Command` with proper localization and color settings.
///
/// This function sets up a `Command` with:
/// - Appropriate color settings based on environment variables (`NO_COLOR`, `CLICOLOR_FORCE`, etc.)
/// - Localized help template with proper formatting
/// - TTY detection for automatic color enabling/disabling
///
/// # Arguments
///
/// * `cmd` - The clap `Command` to configure
///
/// # Returns
///
/// The configured `Command` with localization and color settings applied.
///
/// # Environment Variables
///
/// The following environment variables affect color output:
/// - `NO_COLOR` - Disables all color output
/// - `CLICOLOR_FORCE` or `FORCE_COLOR` - Forces color output even when not in a TTY
/// - `TERM` - If set to "dumb", colors are disabled in auto mode
///
/// # Examples
///
/// ```no_run
/// use clap::Command;
/// use uucore::clap_localization::configure_localized_command;
///
/// let cmd = Command::new("myutil")
///     .arg(clap::Arg::new("input").short('i'));
/// let configured_cmd = configure_localized_command(cmd);
/// ```
pub fn configure_localized_command(mut cmd: Command) -> Command {
    let color_choice = get_color_choice();
    cmd = cmd.color(color_choice);

    // For help output (stdout), we check stdout TTY status
    let colors_enabled = should_use_color_for_stream(&std::io::stdout());

    cmd = cmd.help_template(crate::localized_help_template_with_colors(
        crate::util_name(),
        colors_enabled,
    ));
    cmd
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
    fn test_color_manager() {
        let mgr = ColorManager(true);
        let red_text = mgr.colorize("error", Color::Red);
        assert_eq!(red_text, "\x1b[31merror\x1b[0m");

        let mgr_disabled = ColorManager(false);
        let plain_text = mgr_disabled.colorize("error", Color::Red);
        assert_eq!(plain_text, "error");
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
    fn test_handle_clap_result_with_valid_args() {
        let cmd = create_test_command();
        let result = handle_clap_result(cmd, vec!["test", "--input", "file.txt"]);
        assert!(result.is_ok());
        let matches = result.unwrap();
        assert_eq!(matches.get_one::<String>("input").unwrap(), "file.txt");
    }

    #[test]
    fn test_handle_clap_result_with_osstring() {
        let args: Vec<OsString> = vec!["test".into(), "--output".into(), "out.txt".into()];
        let cmd = create_test_command();
        let result = handle_clap_result(cmd, args);
        assert!(result.is_ok());
        let matches = result.unwrap();
        assert_eq!(matches.get_one::<String>("output").unwrap(), "out.txt");
    }

    #[test]
    fn test_configure_localized_command() {
        let cmd = Command::new("test");
        let configured = configure_localized_command(cmd);
        // The command should have color and help template configured
        // We can't easily test the internal state, but we can verify it doesn't panic
        assert_eq!(configured.get_name(), "test");
    }

    #[test]
    fn test_color_environment_vars() {
        use std::env;

        // Test NO_COLOR disables colors
        unsafe {
            env::set_var("NO_COLOR", "1");
        }
        assert_eq!(get_color_choice(), clap::ColorChoice::Never);
        assert!(!should_use_color_for_stream(&std::io::stderr()));
        let mgr = ColorManager::from_env();
        assert!(!mgr.0);
        unsafe {
            env::remove_var("NO_COLOR");
        }

        // Test CLICOLOR_FORCE enables colors
        unsafe {
            env::set_var("CLICOLOR_FORCE", "1");
        }
        assert_eq!(get_color_choice(), clap::ColorChoice::Always);
        assert!(should_use_color_for_stream(&std::io::stderr()));
        let mgr = ColorManager::from_env();
        assert!(mgr.0);
        unsafe {
            env::remove_var("CLICOLOR_FORCE");
        }

        // Test FORCE_COLOR also enables colors
        unsafe {
            env::set_var("FORCE_COLOR", "1");
        }
        assert_eq!(get_color_choice(), clap::ColorChoice::Always);
        assert!(should_use_color_for_stream(&std::io::stderr()));
        unsafe {
            env::remove_var("FORCE_COLOR");
        }
    }

    #[test]
    fn test_error_formatter_creation() {
        let formatter = ErrorFormatter::new("test");
        assert_eq!(formatter.util_name, "test");
        // Color manager should be created based on environment
    }

    #[test]
    fn test_localization_keys_exist() {
        use crate::locale::{get_message, setup_localization};

        let _ = setup_localization("test");

        let required_keys = [
            "common-error",
            "common-usage",
            "common-tip",
            "common-help-suggestion",
            "clap-error-unexpected-argument",
            "clap-error-invalid-value",
            "clap-error-missing-required-arguments",
            "clap-error-similar-argument",
            "clap-error-possible-values",
            "clap-error-value-required",
        ];

        for key in &required_keys {
            let message = get_message(key);
            assert_ne!(message, *key, "Translation missing for key: {key}");
        }
    }

    #[test]
    fn test_french_localization() {
        use crate::locale::{get_message, setup_localization};
        use std::env;

        let original_lang = env::var("LANG").unwrap_or_default();

        unsafe {
            env::set_var("LANG", "fr_FR.UTF-8");
        }

        if setup_localization("test").is_ok() {
            assert_eq!(get_message("common-error"), "erreur");
            assert_eq!(get_message("common-usage"), "Utilisation");
            assert_eq!(get_message("common-tip"), "conseil");
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
