// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:ignore unic_langid

use crate::error::UError;
use fluent::{FluentArgs, FluentBundle, FluentResource};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::OnceLock;
use thiserror::Error;
use unic_langid::LanguageIdentifier;

#[derive(Error, Debug)]
pub enum LocalizationError {
    #[error("I/O error loading '{path}': {source}")]
    Io {
        source: std::io::Error,
        path: PathBuf,
    },
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Bundle error: {0}")]
    Bundle(String),
}

impl From<std::io::Error> for LocalizationError {
    fn from(error: std::io::Error) -> Self {
        LocalizationError::Io {
            source: error,
            path: PathBuf::from("<unknown>"),
        }
    }
}

// Add a generic way to convert LocalizationError to UError
impl UError for LocalizationError {
    fn code(&self) -> i32 {
        1
    }
}

pub const DEFAULT_LOCALE: &str = "en-US";

// A struct to handle localization with optional English fallback
struct Localizer {
    primary_bundle: FluentBundle<FluentResource>,
    fallback_bundle: Option<FluentBundle<FluentResource>>,
}

impl Localizer {
    fn new(primary_bundle: FluentBundle<FluentResource>) -> Self {
        Self {
            primary_bundle,
            fallback_bundle: None,
        }
    }

    fn with_fallback(mut self, fallback_bundle: FluentBundle<FluentResource>) -> Self {
        self.fallback_bundle = Some(fallback_bundle);
        self
    }

    fn format(&self, id: &str, args: Option<&FluentArgs>) -> String {
        // Try primary bundle first
        if let Some(message) = self.primary_bundle.get_message(id).and_then(|m| m.value()) {
            let mut errs = Vec::new();
            return self
                .primary_bundle
                .format_pattern(message, args, &mut errs)
                .to_string();
        }

        // Fall back to English bundle if available
        if let Some(ref fallback) = self.fallback_bundle {
            if let Some(message) = fallback.get_message(id).and_then(|m| m.value()) {
                let mut errs = Vec::new();
                return fallback
                    .format_pattern(message, args, &mut errs)
                    .to_string();
            }
        }

        // Return the key ID if not found anywhere
        id.to_string()
    }
}

// Global localizer stored in thread-local OnceLock
thread_local! {
    static LOCALIZER: OnceLock<Localizer> = const { OnceLock::new() };
}

// Initialize localization with a specific locale and config
fn init_localization(
    locale: &LanguageIdentifier,
    locales_dir: &Path,
) -> Result<(), LocalizationError> {
    let en_locale = LanguageIdentifier::from_str(DEFAULT_LOCALE)
        .expect("Default locale should always be valid");

    let english_bundle = create_bundle(&en_locale, locales_dir)?;
    let loc = if locale == &en_locale {
        // If requesting English, just use English as primary (no fallback needed)
        Localizer::new(english_bundle)
    } else {
        // Try to load the requested locale
        if let Ok(primary_bundle) = create_bundle(locale, locales_dir) {
            // Successfully loaded requested locale, load English as fallback
            Localizer::new(primary_bundle).with_fallback(english_bundle)
        } else {
            // Failed to load requested locale, just use English as primary
            Localizer::new(english_bundle)
        }
    };

    LOCALIZER.with(|lock| {
        lock.set(loc)
            .map_err(|_| LocalizationError::Bundle("Localizer already initialized".into()))
    })?;
    Ok(())
}

// Create a bundle for a specific locale
fn create_bundle(
    locale: &LanguageIdentifier,
    locales_dir: &Path,
) -> Result<FluentBundle<FluentResource>, LocalizationError> {
    let locale_path = locales_dir.join(format!("{locale}.ftl"));

    let ftl_file = fs::read_to_string(&locale_path).map_err(|e| LocalizationError::Io {
        source: e,
        path: locale_path.clone(),
    })?;

    let resource = FluentResource::try_new(ftl_file).map_err(|_| {
        LocalizationError::Parse(format!(
            "Failed to parse localization resource for {}: {}",
            locale,
            locale_path.display()
        ))
    })?;

    let mut bundle = FluentBundle::new(vec![locale.clone()]);

    bundle.add_resource(resource).map_err(|errs| {
        LocalizationError::Bundle(format!(
            "Failed to add resource to bundle for {}: {:?}",
            locale, errs
        ))
    })?;

    Ok(bundle)
}

fn get_message_internal(id: &str, args: Option<FluentArgs>) -> String {
    LOCALIZER.with(|lock| {
        lock.get()
            .map(|loc| loc.format(id, args.as_ref()))
            .unwrap_or_else(|| id.to_string()) // Return the key ID if localizer not initialized
    })
}

/// Retrieves a localized message by its identifier.
///
/// Looks up a message with the given ID in the current locale bundle and returns
/// the localized text. If the message ID is not found in the current locale,
/// it will fall back to English. If the message is not found in English either,
/// returns the message ID itself.
///
/// # Arguments
///
/// * `id` - The message identifier in the Fluent resources
///
/// # Returns
///
/// A `String` containing the localized message, or the message ID if not found
///
/// # Examples
///
/// ```
/// use uucore::locale::get_message;
///
/// // Get a localized greeting (from .ftl files)
/// let greeting = get_message("greeting");
/// println!("{}", greeting);
/// ```
pub fn get_message(id: &str) -> String {
    get_message_internal(id, None)
}

/// Retrieves a localized message with variable substitution.
///
/// Looks up a message with the given ID in the current locale bundle,
/// substitutes variables from the provided arguments map, and returns the
/// localized text. If the message ID is not found in the current locale,
/// it will fall back to English. If the message is not found in English either,
/// returns the message ID itself.
///
/// # Arguments
///
/// * `id` - The message identifier in the Fluent resources
/// * `ftl_args` - Key-value pairs for variable substitution in the message
///
/// # Returns
///
/// A `String` containing the localized message with variable substitution, or the message ID if not found
///
/// # Examples
///
/// ```
/// use uucore::locale::get_message_with_args;
/// use std::collections::HashMap;
///
/// // For a Fluent message like: "Hello, { $name }! You have { $count } notifications."
/// let mut args = HashMap::new();
/// args.insert("name".to_string(), "Alice".to_string());
/// args.insert("count".to_string(), "3".to_string());
///
/// let message = get_message_with_args("notification", args);
/// println!("{}", message);
/// ```
pub fn get_message_with_args(id: &str, ftl_args: HashMap<String, String>) -> String {
    let mut args = FluentArgs::new();

    for (key, value) in ftl_args {
        // Try to parse as number first for proper pluralization support
        if let Ok(num_val) = value.parse::<i64>() {
            args.set(key, num_val);
        } else if let Ok(float_val) = value.parse::<f64>() {
            args.set(key, float_val);
        } else {
            // Keep as string if not a number
            args.set(key, value);
        }
    }

    get_message_internal(id, Some(args))
}

// Function to detect system locale from environment variables
fn detect_system_locale() -> Result<LanguageIdentifier, LocalizationError> {
    let locale_str = std::env::var("LANG")
        .unwrap_or_else(|_| DEFAULT_LOCALE.to_string())
        .split('.')
        .next()
        .unwrap_or(DEFAULT_LOCALE)
        .to_string();

    LanguageIdentifier::from_str(&locale_str)
        .map_err(|_| LocalizationError::Parse(format!("Failed to parse locale: {}", locale_str)))
}

/// Sets up localization using the system locale with English fallback.
///
/// This function initializes the localization system based on the system's locale
/// preferences (via the LANG environment variable) or falls back to English
/// if the system locale cannot be determined or the locale file doesn't exist.
/// English is always loaded as a fallback.
///
/// # Arguments
///
/// * `p` - Path to the directory containing localization (.ftl) files
///
/// # Returns
///
/// * `Ok(())` if initialization succeeds
/// * `Err(LocalizationError)` if initialization fails
///
/// # Errors
///
/// Returns a `LocalizationError` if:
/// * The en-US.ftl file cannot be read (English is required)
/// * The files contain invalid Fluent syntax
/// * The bundle cannot be initialized properly
///
/// # Examples
///
/// ```
/// use uucore::locale::setup_localization;
///
/// // Initialize localization using files in the "locales" directory
/// // Make sure you have at least an "en-US.ftl" file in this directory
/// // Other locale files like "fr-FR.ftl" are optional
/// match setup_localization("./locales") {
///     Ok(_) => println!("Localization initialized successfully"),
///     Err(e) => eprintln!("Failed to initialize localization: {}", e),
/// }
/// ```
pub fn setup_localization(p: &str) -> Result<(), LocalizationError> {
    let locale = detect_system_locale().unwrap_or_else(|_| {
        LanguageIdentifier::from_str(DEFAULT_LOCALE).expect("Default locale should always be valid")
    });

    let coreutils_path = PathBuf::from(format!("src/uu/{p}/locales/"));
    let locales_dir = if coreutils_path.exists() {
        coreutils_path
    } else {
        PathBuf::from(p)
    };

    init_localization(&locale, &locales_dir)
}
