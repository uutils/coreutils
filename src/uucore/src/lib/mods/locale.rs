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

// A struct to handle localization
struct Localizer {
    bundle: FluentBundle<FluentResource>,
}

impl Localizer {
    fn new(bundle: FluentBundle<FluentResource>) -> Self {
        Self { bundle }
    }

    fn format(&self, id: &str, args: Option<&FluentArgs>, default: &str) -> String {
        match self.bundle.get_message(id).and_then(|m| m.value()) {
            Some(value) => {
                let mut errs = Vec::new();
                self.bundle
                    .format_pattern(value, args, &mut errs)
                    .to_string()
            }
            None => default.to_string(),
        }
    }
}

// Global localizer stored in thread-local OnceLock
thread_local! {
    static LOCALIZER: OnceLock<Localizer> = const { OnceLock::new() };
}

// Initialize localization with a specific locale and config
fn init_localization(
    locale: &LanguageIdentifier,
    config: &LocalizationConfig,
) -> Result<(), LocalizationError> {
    let bundle = create_bundle(locale, config)?;
    LOCALIZER.with(|lock| {
        let loc = Localizer::new(bundle);
        lock.set(loc)
            .map_err(|_| LocalizationError::Bundle("Localizer already initialized".into()))
    })?;
    Ok(())
}

// Create a bundle for a locale with fallback chain
fn create_bundle(
    locale: &LanguageIdentifier,
    config: &LocalizationConfig,
) -> Result<FluentBundle<FluentResource>, LocalizationError> {
    // Create a new bundle with requested locale
    let mut bundle = FluentBundle::new(vec![locale.clone()]);

    // Try to load the requested locale
    let mut locales_to_try = vec![locale.clone()];
    locales_to_try.extend_from_slice(&config.fallback_locales);

    // Try each locale in the chain
    let mut tried_paths = Vec::new();

    for try_locale in locales_to_try {
        let locale_path = config.get_locale_path(&try_locale);
        tried_paths.push(locale_path.clone());

        if let Ok(ftl_file) = fs::read_to_string(&locale_path) {
            let resource = FluentResource::try_new(ftl_file).map_err(|_| {
                LocalizationError::Parse(format!(
                    "Failed to parse localization resource for {}",
                    try_locale
                ))
            })?;

            bundle.add_resource(resource).map_err(|_| {
                LocalizationError::Bundle(format!(
                    "Failed to add resource to bundle for {}",
                    try_locale
                ))
            })?;

            return Ok(bundle);
        }
    }

    let paths_str = tried_paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    Err(LocalizationError::Io {
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "No localization files found"),
        path: PathBuf::from(paths_str),
    })
}

fn get_message_internal(id: &str, args: Option<FluentArgs>, default: &str) -> String {
    LOCALIZER.with(|lock| {
        lock.get()
            .map(|loc| loc.format(id, args.as_ref(), default))
            .unwrap_or_else(|| default.to_string())
    })
}

/// Retrieves a localized message by its identifier.
///
/// Looks up a message with the given ID in the current locale bundle and returns
/// the localized text. If the message ID is not found, returns the provided default text.
///
/// # Arguments
///
/// * `id` - The message identifier in the Fluent resources
/// * `default` - Default text to use if the message ID isn't found
///
/// # Returns
///
/// A `String` containing either the localized message or the default text
///
/// # Examples
///
/// ```
/// use uucore::locale::get_message;
///
/// // Get a localized greeting or fall back to English
/// let greeting = get_message("greeting", "Hello, World!");
/// println!("{}", greeting);
/// ```
pub fn get_message(id: &str, default: &str) -> String {
    get_message_internal(id, None, default)
}

/// Retrieves a localized message with variable substitution.
///
/// Looks up a message with the given ID in the current locale bundle,
/// substitutes variables from the provided arguments map, and returns the
/// localized text. If the message ID is not found, returns the provided default text.
///
/// # Arguments
///
/// * `id` - The message identifier in the Fluent resources
/// * `ftl_args` - Key-value pairs for variable substitution in the message
/// * `default` - Default text to use if the message ID isn't found
///
/// # Returns
///
/// A `String` containing either the localized message with variable substitution or the default text
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
/// let message = get_message_with_args(
///     "notification",
///     args,
///     "Hello! You have notifications."
/// );
/// println!("{}", message);
/// ```
pub fn get_message_with_args(id: &str, ftl_args: HashMap<String, String>, default: &str) -> String {
    let args = ftl_args.into_iter().collect();
    get_message_internal(id, Some(args), default)
}

// Configuration for localization
#[derive(Clone)]
struct LocalizationConfig {
    locales_dir: PathBuf,
    fallback_locales: Vec<LanguageIdentifier>,
}

impl LocalizationConfig {
    // Create a new config with a specific locales directory
    fn new<P: AsRef<Path>>(locales_dir: P) -> Self {
        Self {
            locales_dir: locales_dir.as_ref().to_path_buf(),
            fallback_locales: vec![],
        }
    }

    // Set fallback locales
    fn with_fallbacks(mut self, fallbacks: Vec<LanguageIdentifier>) -> Self {
        self.fallback_locales = fallbacks;
        self
    }

    // Get path for a specific locale
    fn get_locale_path(&self, locale: &LanguageIdentifier) -> PathBuf {
        self.locales_dir.join(format!("{}.ftl", locale))
    }
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

/// Sets up localization using the system locale (or default) and project paths.
///
/// This function initializes the localization system based on the system's locale
/// preferences (via the LANG environment variable) or falls back to the default locale
/// if the system locale cannot be determined or is invalid.
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
/// * The localization files cannot be read
/// * The files contain invalid syntax
/// * The bundle cannot be initialized properly
///
/// # Examples
///
/// ```
/// use uucore::locale::setup_localization;
///
/// // Initialize localization using files in the "locales" directory
/// match setup_localization("./locales") {
///     Ok(_) => println!("Localization initialized successfully"),
///     Err(e) => eprintln!("Failed to initialize localization: {}", e),
/// }
/// ```
pub fn setup_localization(p: &str) -> Result<(), LocalizationError> {
    let locale = detect_system_locale().unwrap_or_else(|_| {
        LanguageIdentifier::from_str(DEFAULT_LOCALE).expect("Default locale should always be valid")
    });

    let locales_dir = PathBuf::from(p);
    let fallback_locales = vec![
        LanguageIdentifier::from_str(DEFAULT_LOCALE)
            .expect("Default locale should always be valid"),
    ];

    let config = LocalizationConfig::new(locales_dir).with_fallbacks(fallback_locales);

    init_localization(&locale, &config)?;
    Ok(())
}
