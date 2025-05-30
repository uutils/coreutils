// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
// spell-checker:disable

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
    #[error("Locales directory not found: {0}")]
    LocalesDirNotFound(String),
    #[error("Path resolution error: {0}")]
    PathResolution(String),
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

    let locales_dir = get_locales_dir(p)?;
    init_localization(&locale, &locales_dir)
}

/// Helper function to get the locales directory based on the build configuration
fn get_locales_dir(p: &str) -> Result<PathBuf, LocalizationError> {
    #[cfg(debug_assertions)]
    {
        // During development, use the project's locales directory
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        // from uucore path, load the locales directory from the program directory
        let dev_path = PathBuf::from(manifest_dir)
            .join("../uu")
            .join(p)
            .join("locales");

        if dev_path.exists() {
            return Ok(dev_path);
        }

        // Fallback for development if the expected path doesn't exist
        let fallback_dev_path = PathBuf::from(manifest_dir).join(p);
        if fallback_dev_path.exists() {
            return Ok(fallback_dev_path);
        }

        Err(LocalizationError::LocalesDirNotFound(format!(
            "Development locales directory not found at {} or {}",
            dev_path.display(),
            fallback_dev_path.display()
        )))
    }

    #[cfg(not(debug_assertions))]
    {
        use std::env;
        // In release builds, look relative to executable
        let exe_path = env::current_exe().map_err(|e| {
            LocalizationError::PathResolution(format!("Failed to get executable path: {}", e))
        })?;

        let exe_dir = exe_path.parent().ok_or_else(|| {
            LocalizationError::PathResolution("Failed to get executable directory".to_string())
        })?;

        // Try the coreutils-style path first
        let coreutils_path = exe_dir.join("locales").join(p);
        if coreutils_path.exists() {
            return Ok(coreutils_path);
        }

        // Fallback to just the parameter as a relative path
        let fallback_path = exe_dir.join(p);
        if fallback_path.exists() {
            return Ok(fallback_path);
        }

        return Err(LocalizationError::LocalesDirNotFound(format!(
            "Release locales directory not found at {} or {}",
            coreutils_path.display(),
            fallback_path.display()
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // Helper function to create a temporary directory with test locale files
    fn create_test_locales_dir() -> TempDir {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");

        // Create en-US.ftl
        let en_content = r#"
greeting = Hello, world!
welcome = Welcome, { $name }!
count-items = You have { $count ->
    [one] { $count } item
   *[other] { $count } items
}
missing-in-other = This message only exists in English
"#;

        // Create fr-FR.ftl
        let fr_content = r#"
greeting = Bonjour, le monde!
welcome = Bienvenue, { $name }!
count-items = Vous avez { $count ->
    [one] { $count } élément
   *[other] { $count } éléments
}
"#;

        // Create ja-JP.ftl (Japanese)
        let ja_content = r#"
greeting = こんにちは、世界！
welcome = ようこそ、{ $name }さん！
count-items = { $count }個のアイテムがあります
"#;

        // Create ar-SA.ftl (Arabic - Right-to-Left)
        let ar_content = r#"
greeting = أهلاً بالعالم！
welcome = أهلاً وسهلاً، { $name }！
count-items = لديك { $count ->
    [zero] لا عناصر
    [one] عنصر واحد
    [two] عنصران
    [few] { $count } عناصر
   *[other] { $count } عنصر
}
"#;

        // Create es-ES.ftl with invalid syntax
        let es_invalid_content = r#"
greeting = Hola, mundo!
invalid-syntax = This is { $missing
"#;

        fs::write(temp_dir.path().join("en-US.ftl"), en_content)
            .expect("Failed to write en-US.ftl");
        fs::write(temp_dir.path().join("fr-FR.ftl"), fr_content)
            .expect("Failed to write fr-FR.ftl");
        fs::write(temp_dir.path().join("ja-JP.ftl"), ja_content)
            .expect("Failed to write ja-JP.ftl");
        fs::write(temp_dir.path().join("ar-SA.ftl"), ar_content)
            .expect("Failed to write ar-SA.ftl");
        fs::write(temp_dir.path().join("es-ES.ftl"), es_invalid_content)
            .expect("Failed to write es-ES.ftl");

        temp_dir
    }

    #[test]
    fn test_localization_error_from_io_error() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let loc_error = LocalizationError::from(io_error);

        match loc_error {
            LocalizationError::Io { source: _, path } => {
                assert_eq!(path, PathBuf::from("<unknown>"));
            }
            _ => panic!("Expected IO error variant"),
        }
    }

    #[test]
    fn test_localization_error_uerror_impl() {
        let error = LocalizationError::Parse("test error".to_string());
        assert_eq!(error.code(), 1);
    }

    #[test]
    fn test_create_bundle_success() {
        let temp_dir = create_test_locales_dir();
        let locale = LanguageIdentifier::from_str("en-US").unwrap();

        let result = create_bundle(&locale, temp_dir.path());
        assert!(result.is_ok());

        let bundle = result.unwrap();
        assert!(bundle.get_message("greeting").is_some());
    }

    #[test]
    fn test_create_bundle_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let locale = LanguageIdentifier::from_str("de-DE").unwrap();

        let result = create_bundle(&locale, temp_dir.path());
        assert!(result.is_err());

        if let Err(LocalizationError::Io { source: _, path }) = result {
            assert!(path.to_string_lossy().contains("de-DE.ftl"));
        } else {
            panic!("Expected IO error");
        }
    }

    #[test]
    fn test_create_bundle_invalid_syntax() {
        let temp_dir = create_test_locales_dir();
        let locale = LanguageIdentifier::from_str("es-ES").unwrap();

        let result = create_bundle(&locale, temp_dir.path());
        assert!(result.is_err());

        if let Err(LocalizationError::Parse(_)) = result {
            // Expected parse error
        } else {
            panic!("Expected parse error");
        }
    }

    #[test]
    fn test_localizer_format_primary_bundle() {
        let temp_dir = create_test_locales_dir();
        let en_bundle = create_bundle(
            &LanguageIdentifier::from_str("en-US").unwrap(),
            temp_dir.path(),
        )
        .unwrap();

        let localizer = Localizer::new(en_bundle);
        let result = localizer.format("greeting", None);
        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_localizer_format_with_args() {
        let temp_dir = create_test_locales_dir();
        let en_bundle = create_bundle(
            &LanguageIdentifier::from_str("en-US").unwrap(),
            temp_dir.path(),
        )
        .unwrap();

        let localizer = Localizer::new(en_bundle);
        let mut args = FluentArgs::new();
        args.set("name", "Alice");

        let result = localizer.format("welcome", Some(&args));
        assert_eq!(result, "Welcome, \u{2068}Alice\u{2069}!");
    }

    #[test]
    fn test_localizer_fallback_to_english() {
        let temp_dir = create_test_locales_dir();
        let fr_bundle = create_bundle(
            &LanguageIdentifier::from_str("fr-FR").unwrap(),
            temp_dir.path(),
        )
        .unwrap();
        let en_bundle = create_bundle(
            &LanguageIdentifier::from_str("en-US").unwrap(),
            temp_dir.path(),
        )
        .unwrap();

        let localizer = Localizer::new(fr_bundle).with_fallback(en_bundle);

        // This message exists in French
        let result1 = localizer.format("greeting", None);
        assert_eq!(result1, "Bonjour, le monde!");

        // This message only exists in English, should fallback
        let result2 = localizer.format("missing-in-other", None);
        assert_eq!(result2, "This message only exists in English");
    }

    #[test]
    fn test_localizer_format_message_not_found() {
        let temp_dir = create_test_locales_dir();
        let en_bundle = create_bundle(
            &LanguageIdentifier::from_str("en-US").unwrap(),
            temp_dir.path(),
        )
        .unwrap();

        let localizer = Localizer::new(en_bundle);
        let result = localizer.format("nonexistent-message", None);
        assert_eq!(result, "nonexistent-message");
    }

    #[test]
    fn test_init_localization_english_only() {
        // Run in a separate thread to avoid conflicts with other tests
        std::thread::spawn(|| {
            let temp_dir = create_test_locales_dir();
            let locale = LanguageIdentifier::from_str("en-US").unwrap();

            let result = init_localization(&locale, temp_dir.path());
            assert!(result.is_ok());

            // Test that we can get messages
            let message = get_message("greeting");
            assert_eq!(message, "Hello, world!");
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_init_localization_with_fallback() {
        std::thread::spawn(|| {
            let temp_dir = create_test_locales_dir();
            let locale = LanguageIdentifier::from_str("fr-FR").unwrap();

            let result = init_localization(&locale, temp_dir.path());
            assert!(result.is_ok());

            // Test French message
            let message1 = get_message("greeting");
            assert_eq!(message1, "Bonjour, le monde!");

            // Test fallback to English
            let message2 = get_message("missing-in-other");
            assert_eq!(message2, "This message only exists in English");
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_init_localization_invalid_locale_falls_back_to_english() {
        std::thread::spawn(|| {
            let temp_dir = create_test_locales_dir();
            let locale = LanguageIdentifier::from_str("de-DE").unwrap(); // No German file

            let result = init_localization(&locale, temp_dir.path());
            assert!(result.is_ok());

            // Should use English as primary since German failed to load
            let message = get_message("greeting");
            assert_eq!(message, "Hello, world!");
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_init_localization_already_initialized() {
        std::thread::spawn(|| {
            let temp_dir = create_test_locales_dir();
            let locale = LanguageIdentifier::from_str("en-US").unwrap();

            // Initialize once
            let result1 = init_localization(&locale, temp_dir.path());
            assert!(result1.is_ok());

            // Try to initialize again - should fail
            let result2 = init_localization(&locale, temp_dir.path());
            assert!(result2.is_err());

            match result2 {
                Err(LocalizationError::Bundle(msg)) => {
                    assert!(msg.contains("already initialized"));
                }
                _ => panic!("Expected Bundle error"),
            }
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_get_message() {
        std::thread::spawn(|| {
            let temp_dir = create_test_locales_dir();
            let locale = LanguageIdentifier::from_str("fr-FR").unwrap();

            init_localization(&locale, temp_dir.path()).unwrap();

            let message = get_message("greeting");
            assert_eq!(message, "Bonjour, le monde!");
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_get_message_not_initialized() {
        std::thread::spawn(|| {
            let message = get_message("greeting");
            assert_eq!(message, "greeting"); // Should return the ID itself
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_get_message_with_args() {
        std::thread::spawn(|| {
            let temp_dir = create_test_locales_dir();
            let locale = LanguageIdentifier::from_str("en-US").unwrap();

            init_localization(&locale, temp_dir.path()).unwrap();

            let mut args = HashMap::new();
            args.insert("name".to_string(), "Bob".to_string());

            let message = get_message_with_args("welcome", args);
            assert_eq!(message, "Welcome, \u{2068}Bob\u{2069}!");
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_get_message_with_args_pluralization() {
        std::thread::spawn(|| {
            let temp_dir = create_test_locales_dir();
            let locale = LanguageIdentifier::from_str("en-US").unwrap();

            init_localization(&locale, temp_dir.path()).unwrap();

            // Test singular
            let mut args1 = HashMap::new();
            args1.insert("count".to_string(), "1".to_string());
            let message1 = get_message_with_args("count-items", args1);
            assert_eq!(message1, "You have \u{2068}\u{2068}1\u{2069} item\u{2069}");

            // Test plural
            let mut args2 = HashMap::new();
            args2.insert("count".to_string(), "5".to_string());
            let message2 = get_message_with_args("count-items", args2);
            assert_eq!(message2, "You have \u{2068}\u{2068}5\u{2069} items\u{2069}");
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_detect_system_locale_from_lang_env() {
        // Save current LANG value
        let original_lang = env::var("LANG").ok();

        // Test with a valid locale
        unsafe {
            env::set_var("LANG", "fr-FR.UTF-8");
        }
        let result = detect_system_locale();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "fr-FR");

        // Test with locale without encoding
        unsafe {
            env::set_var("LANG", "es-ES");
        }
        let result = detect_system_locale();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "es-ES");

        // Restore original LANG value
        match original_lang {
            Some(val) => unsafe {
                env::set_var("LANG", val);
            },
            None => unsafe {
                env::remove_var("LANG");
            },
        }
    }

    #[test]
    fn test_detect_system_locale_no_lang_env() {
        // Save current LANG value
        let original_lang = env::var("LANG").ok();

        // Remove LANG environment variable
        unsafe {
            env::remove_var("LANG");
        }

        let result = detect_system_locale();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_string(), "en-US");

        // Restore original LANG value
        match original_lang {
            Some(val) => unsafe {
                env::set_var("LANG", val);
            },
            None => {} // Was already unset
        }
    }

    #[test]
    fn test_setup_localization_success() {
        std::thread::spawn(|| {
            let temp_dir = create_test_locales_dir();

            // Save current LANG value
            let original_lang = env::var("LANG").ok();
            unsafe {
                env::set_var("LANG", "fr-FR.UTF-8");
            }

            let result = setup_localization(temp_dir.path().to_str().unwrap());
            assert!(result.is_ok());

            // Test that French is loaded
            let message = get_message("greeting");
            assert_eq!(message, "Bonjour, le monde!");

            // Restore original LANG value
            match original_lang {
                Some(val) => unsafe {
                    env::set_var("LANG", val);
                },
                None => unsafe {
                    env::remove_var("LANG");
                },
            }
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_setup_localization_falls_back_to_english() {
        std::thread::spawn(|| {
            let temp_dir = create_test_locales_dir();

            // Save current LANG value
            let original_lang = env::var("LANG").ok();
            unsafe {
                env::set_var("LANG", "de-DE.UTF-8");
            } // German file doesn't exist

            let result = setup_localization(temp_dir.path().to_str().unwrap());
            assert!(result.is_ok());

            // Should fall back to English
            let message = get_message("greeting");
            assert_eq!(message, "Hello, world!");

            // Restore original LANG value
            match original_lang {
                Some(val) => unsafe {
                    env::set_var("LANG", val);
                },
                None => unsafe {
                    env::remove_var("LANG");
                },
            }
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_setup_localization_missing_english_file() {
        std::thread::spawn(|| {
            let temp_dir = TempDir::new().unwrap(); // Empty directory

            let result = setup_localization(temp_dir.path().to_str().unwrap());
            assert!(result.is_err());

            match result {
                Err(LocalizationError::Io { source: _, path }) => {
                    assert!(path.to_string_lossy().contains("en-US.ftl"));
                }
                _ => panic!("Expected IO error for missing English file"),
            }
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_thread_local_isolation() {
        use std::thread;

        let temp_dir = create_test_locales_dir();

        // Initialize in main thread with French
        let temp_path_main = temp_dir.path().to_path_buf();
        let main_handle = thread::spawn(move || {
            let locale = LanguageIdentifier::from_str("fr-FR").unwrap();
            init_localization(&locale, &temp_path_main).unwrap();
            let main_message = get_message("greeting");
            assert_eq!(main_message, "Bonjour, le monde!");
        });
        main_handle.join().unwrap();

        // Test in a different thread - should not be initialized
        let temp_path = temp_dir.path().to_path_buf();
        let handle = thread::spawn(move || {
            // This thread should have its own uninitialized LOCALIZER
            let thread_message = get_message("greeting");
            assert_eq!(thread_message, "greeting"); // Returns ID since not initialized

            // Initialize in this thread with English
            let en_locale = LanguageIdentifier::from_str("en-US").unwrap();
            init_localization(&en_locale, &temp_path).unwrap();
            let thread_message_after_init = get_message("greeting");
            assert_eq!(thread_message_after_init, "Hello, world!");
        });

        handle.join().unwrap();

        // Test another thread to verify French doesn't persist across threads
        let final_handle = thread::spawn(move || {
            // Should be uninitialized again
            let final_message = get_message("greeting");
            assert_eq!(final_message, "greeting");
        });
        final_handle.join().unwrap();
    }

    #[test]
    fn test_japanese_localization() {
        std::thread::spawn(|| {
            let temp_dir = create_test_locales_dir();
            let locale = LanguageIdentifier::from_str("ja-JP").unwrap();

            let result = init_localization(&locale, temp_dir.path());
            assert!(result.is_ok());

            // Test Japanese greeting
            let message = get_message("greeting");
            assert_eq!(message, "こんにちは、世界！");

            // Test Japanese with arguments
            let mut args = HashMap::new();
            args.insert("name".to_string(), "田中".to_string());
            let welcome = get_message_with_args("welcome", args);
            assert_eq!(welcome, "ようこそ、\u{2068}田中\u{2069}さん！");

            // Test Japanese count (no pluralization)
            let mut count_args = HashMap::new();
            count_args.insert("count".to_string(), "5".to_string());
            let count_message = get_message_with_args("count-items", count_args);
            assert_eq!(count_message, "\u{2068}5\u{2069}個のアイテムがあります");
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_arabic_localization() {
        std::thread::spawn(|| {
            let temp_dir = create_test_locales_dir();
            let locale = LanguageIdentifier::from_str("ar-SA").unwrap();

            let result = init_localization(&locale, temp_dir.path());
            assert!(result.is_ok());

            // Test Arabic greeting (RTL text)
            let message = get_message("greeting");
            assert_eq!(message, "أهلاً بالعالم！");

            // Test Arabic with arguments
            let mut args = HashMap::new();
            args.insert("name".to_string(), "أحمد".to_string());
            let welcome = get_message_with_args("welcome", args);
            assert_eq!(welcome, "أهلاً وسهلاً، \u{2068}أحمد\u{2069}！");

            // Test Arabic pluralization (zero case)
            let mut args_zero = HashMap::new();
            args_zero.insert("count".to_string(), "0".to_string());
            let message_zero = get_message_with_args("count-items", args_zero);
            assert_eq!(message_zero, "لديك \u{2068}لا عناصر\u{2069}");

            // Test Arabic pluralization (one case)
            let mut args_one = HashMap::new();
            args_one.insert("count".to_string(), "1".to_string());
            let message_one = get_message_with_args("count-items", args_one);
            assert_eq!(message_one, "لديك \u{2068}عنصر واحد\u{2069}");

            // Test Arabic pluralization (two case)
            let mut args_two = HashMap::new();
            args_two.insert("count".to_string(), "2".to_string());
            let message_two = get_message_with_args("count-items", args_two);
            assert_eq!(message_two, "لديك \u{2068}عنصران\u{2069}");

            // Test Arabic pluralization (few case - 3-10)
            let mut args_few = HashMap::new();
            args_few.insert("count".to_string(), "5".to_string());
            let message_few = get_message_with_args("count-items", args_few);
            assert_eq!(message_few, "لديك \u{2068}\u{2068}5\u{2069} عناصر\u{2069}");

            // Test Arabic pluralization (other case - 11+)
            let mut args_many = HashMap::new();
            args_many.insert("count".to_string(), "15".to_string());
            let message_many = get_message_with_args("count-items", args_many);
            assert_eq!(message_many, "لديك \u{2068}\u{2068}15\u{2069} عنصر\u{2069}");
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_mixed_script_fallback() {
        std::thread::spawn(|| {
            let temp_dir = create_test_locales_dir();
            let locale = LanguageIdentifier::from_str("ar-SA").unwrap();

            let result = init_localization(&locale, temp_dir.path());
            assert!(result.is_ok());

            // Test Arabic message exists
            let arabic_message = get_message("greeting");
            assert_eq!(arabic_message, "أهلاً بالعالم！");

            // Test fallback to English for missing message
            let fallback_message = get_message("missing-in-other");
            assert_eq!(fallback_message, "This message only exists in English");
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_unicode_directional_isolation() {
        std::thread::spawn(|| {
            let temp_dir = create_test_locales_dir();
            let locale = LanguageIdentifier::from_str("ar-SA").unwrap();

            init_localization(&locale, temp_dir.path()).unwrap();

            // Test that Latin script names are properly isolated in RTL context
            let mut args = HashMap::new();
            args.insert("name".to_string(), "John Smith".to_string());
            let message = get_message_with_args("welcome", args);

            // The Latin name should be wrapped in directional isolate characters
            assert!(message.contains("\u{2068}John Smith\u{2069}"));
            assert_eq!(message, "أهلاً وسهلاً، \u{2068}John Smith\u{2069}！");
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_error_display() {
        let io_error = LocalizationError::Io {
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"),
            path: PathBuf::from("/test/path.ftl"),
        };
        let error_string = format!("{}", io_error);
        assert!(error_string.contains("I/O error loading"));
        assert!(error_string.contains("/test/path.ftl"));

        let parse_error = LocalizationError::Parse("Syntax error".to_string());
        let parse_string = format!("{}", parse_error);
        assert!(parse_string.contains("Parse error: Syntax error"));

        let bundle_error = LocalizationError::Bundle("Bundle creation failed".to_string());
        let bundle_string = format!("{}", bundle_error);
        assert!(bundle_string.contains("Bundle error: Bundle creation failed"));
    }
}
