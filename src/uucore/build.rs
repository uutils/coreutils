// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var("OUT_DIR")?;

    let mut embedded_file = File::create(Path::new(&out_dir).join("embedded_locales.rs"))?;
    writeln!(embedded_file, "// Generated at compile time - do not edit")?;
    writeln!(
        embedded_file,
        "// This file contains embedded English locale files"
    )?;
    writeln!(embedded_file)?;
    // No imports needed for match-based lookup
    writeln!(embedded_file)?;

    // Generate optimized lookup function instead of HashMap
    writeln!(
        embedded_file,
        "pub fn get_embedded_locale(key: &str) -> Option<&'static str> {{"
    )?;
    writeln!(embedded_file, "    match key {{")?;

    // Try to detect if we're building for a specific utility by checking build configuration
    // This attempts to identify individual utility builds vs multicall binary builds
    let target_utility = detect_target_utility();
    let locales_to_embed = get_locales_to_embed();

    match target_utility {
        Some(util_name) => {
            // Embed only the specific utility's locale (cat.ftl for cat for example)
            embed_single_utility_locale(
                &mut embedded_file,
                &project_root()?,
                &util_name,
                &locales_to_embed,
            )?;
        }
        None => {
            // Embed all utility locales (multicall binary or fallback)
            embed_all_utility_locales(&mut embedded_file, &project_root()?, &locales_to_embed)?;
        }
    }

    writeln!(embedded_file, "        _ => None,")?;
    writeln!(embedded_file, "    }}")?;
    writeln!(embedded_file, "}}")?;

    embedded_file.flush()?;
    Ok(())
}

/// Get the project root directory
///
/// # Errors
///
/// Returns an error if the `CARGO_MANIFEST_DIR` environment variable is not set
/// or if the current directory structure does not allow determining the project root.
fn project_root() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
    let uucore_path = std::path::Path::new(&manifest_dir);

    // Navigate from src/uucore to project root
    let project_root = uucore_path
        .parent() // src/
        .and_then(|p| p.parent()) // project root
        .ok_or("Could not determine project root")?;

    Ok(project_root.to_path_buf())
}

/// Attempt to detect which specific utility is being built
fn detect_target_utility() -> Option<String> {
    use std::fs;

    // Tell Cargo to rerun if this environment variable changes
    println!("cargo:rerun-if-env-changed=UUCORE_TARGET_UTIL");

    // First check if an explicit environment variable was set
    if let Ok(target_util) = env::var("UUCORE_TARGET_UTIL") {
        if !target_util.is_empty() {
            return Some(target_util);
        }
    }

    // Auto-detect utility name from CARGO_PKG_NAME if it's a uu_* package
    if let Ok(pkg_name) = env::var("CARGO_PKG_NAME") {
        if let Some(util_name) = pkg_name.strip_prefix("uu_") {
            println!("cargo:warning=Auto-detected utility name: {util_name}");
            return Some(util_name.to_string());
        }
    }

    // Check for a build configuration file in the target directory
    if let Ok(target_dir) = env::var("CARGO_TARGET_DIR") {
        let config_path = std::path::Path::new(&target_dir).join("uucore_target_util.txt");
        if let Ok(content) = fs::read_to_string(&config_path) {
            let util_name = content.trim();
            if !util_name.is_empty() && util_name != "multicall" {
                return Some(util_name.to_string());
            }
        }
    }

    // Fallback: Check the default target directory
    if let Ok(project_root) = project_root() {
        let config_path = project_root.join("target/uucore_target_util.txt");
        if let Ok(content) = fs::read_to_string(&config_path) {
            let util_name = content.trim();
            if !util_name.is_empty() && util_name != "multicall" {
                return Some(util_name.to_string());
            }
        }
    }

    // If no configuration found, assume multicall build
    None
}

/// Embed locale for a single specific utility
///
/// # Errors
///
/// Returns an error if the locales for `util_name` or `uucore` cannot be found
/// or if writing to the `embedded_file` fails.
fn embed_single_utility_locale(
    embedded_file: &mut std::fs::File,
    project_root: &Path,
    util_name: &str,
    locales_to_embed: &(String, Option<String>),
) -> Result<(), Box<dyn std::error::Error>> {
    // Embed utility-specific locales
    embed_component_locales(embedded_file, locales_to_embed, util_name, |locale| {
        project_root
            .join("src/uu")
            .join(util_name)
            .join(format!("locales/{locale}.ftl"))
    })?;

    // Always embed uucore locale file if it exists
    embed_component_locales(embedded_file, locales_to_embed, "uucore", |locale| {
        project_root.join(format!("src/uucore/locales/{locale}.ftl"))
    })?;

    Ok(())
}

/// Embed locale files for all utilities (multicall binary).
///
/// # Errors
///
/// Returns an error if the `src/uu` directory cannot be read, if any utility
/// locales cannot be embedded, or if flushing the `embedded_file` fails.
fn embed_all_utility_locales(
    embedded_file: &mut std::fs::File,
    project_root: &Path,
    locales_to_embed: &(String, Option<String>),
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    // Discover all uu_* directories
    let src_uu_dir = project_root.join("src/uu");
    if !src_uu_dir.exists() {
        // When src/uu doesn't exist (e.g., standalone uucore from crates.io),
        // embed a static list of utility locales that are commonly used
        embed_static_utility_locales(embedded_file, locales_to_embed)?;
        return Ok(());
    }

    let mut util_dirs = Vec::new();
    for entry in fs::read_dir(&src_uu_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(dir_name) = entry.file_name().to_str() {
                util_dirs.push(dir_name.to_string());
            }
        }
    }
    util_dirs.sort();

    // Embed locale files for each utility
    for util_name in &util_dirs {
        embed_component_locales(embedded_file, locales_to_embed, util_name, |locale| {
            src_uu_dir
                .join(util_name)
                .join(format!("locales/{locale}.ftl"))
        })?;
    }

    // Also embed uucore locale file if it exists
    embed_component_locales(embedded_file, locales_to_embed, "uucore", |locale| {
        project_root.join(format!("src/uucore/locales/{locale}.ftl"))
    })?;

    embedded_file.flush()?;
    Ok(())
}

/// Embed static utility locales for crates.io builds.
///
/// # Errors
///
/// Returns an error if the directory containing the crate cannot be read or
/// if writing to the `embedded_file` fails.
fn embed_static_utility_locales(
    embedded_file: &mut std::fs::File,
    locales_to_embed: &(String, Option<String>),
) -> Result<(), Box<dyn std::error::Error>> {
    use std::env;

    writeln!(
        embedded_file,
        "        // Static utility locales for crates.io builds"
    )?;

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let Some(registry_dir) = Path::new(&manifest_dir).parent() else {
        return Ok(()); // nothing to scan
    };

    // First, try to embed uucore locales - critical for common translations like "Usage:"
    embed_component_locales(embedded_file, locales_to_embed, "uucore", |locale| {
        Path::new(&manifest_dir).join(format!("locales/{locale}.ftl"))
    })?;

    // Collect and sort for deterministic builds
    let mut entries: Vec<_> = std::fs::read_dir(registry_dir)?
        .filter_map(Result::ok)
        .collect();
    entries.sort_by_key(std::fs::DirEntry::file_name);

    for entry in entries {
        let file_name = entry.file_name();
        if let Some(dir_name) = file_name.to_str() {
            // Match uu_<util>-<version>
            if let Some((util_part, _)) = dir_name.split_once('-') {
                if let Some(util_name) = util_part.strip_prefix("uu_") {
                    embed_component_locales(
                        embedded_file,
                        locales_to_embed,
                        util_name,
                        |locale| entry.path().join(format!("locales/{locale}.ftl")),
                    )?;
                }
            }
        }
    }

    Ok(())
}

/// Determines which locales to embed into the binary.
///
/// To support localized messages in installed binaries (e.g., via `cargo install`),
/// this function identifies the user's current locale from the `LANG` environment
/// variable.
///
/// It always includes "en-US" to ensure that a fallback is available if the
/// system locale's translation file is missing or if `LANG` is not set.
fn get_locales_to_embed() -> (String, Option<String>) {
    let system_locale = env::var("LANG").ok().and_then(|lang| {
        let locale = lang.split('.').next()?.replace('_', "-");
        if locale != "en-US" && !locale.is_empty() {
            Some(locale)
        } else {
            None
        }
    });
    ("en-US".to_string(), system_locale)
}

/// Helper function to iterate over the locales to embed.
///
/// # Errors
///
/// Returns an error if the provided closure `f` returns an error when called
/// on either the primary or system locale.
fn for_each_locale<F>(
    locales: &(String, Option<String>),
    mut f: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnMut(&str) -> Result<(), Box<dyn std::error::Error>>,
{
    f(&locales.0)?;
    if let Some(ref system_locale) = locales.1 {
        f(system_locale)?;
    }
    Ok(())
}

/// Helper function to embed a single locale file.
///
/// # Errors
///
/// Returns an error if the file at `locale_path` cannot be read or if
/// writing to `embedded_file` fails.
fn embed_locale_file(
    embedded_file: &mut std::fs::File,
    locale_path: &Path,
    locale_key: &str,
    locale: &str,
    component: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    if locale_path.exists() || locale_path.is_file() {
        let content = fs::read_to_string(locale_path)?;
        writeln!(
            embedded_file,
            "        // Locale for {component} ({locale})"
        )?;
        // Determine if we need a hash. If content contains ", we need r#""#
        let delimiter = if content.contains('"') { "#" } else { "" };
        writeln!(
            embedded_file,
            "        \"{locale_key}\" => Some(r{delimiter}\"{content}\"{delimiter}),"
        )?;

        // Tell Cargo to rerun if this file changes
        println!("cargo:rerun-if-changed={}", locale_path.display());
    }
    Ok(())
}

/// Higher-level helper to embed locale files for a component with a path pattern.
///
/// This eliminates the repetitive `for_each_locale` + `embed_locale_file` pattern.
///
/// # Errors
///
/// Returns an error if `for_each_locale` fails, which typically happens if
/// reading a locale file or writing to the `embedded_file` fails.
fn embed_component_locales<F>(
    embedded_file: &mut std::fs::File,
    locales: &(String, Option<String>),
    component_name: &str,
    path_builder: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: Fn(&str) -> std::path::PathBuf,
{
    for_each_locale(locales, |locale| {
        let locale_path = path_builder(locale);
        embed_locale_file(
            embedded_file,
            &locale_path,
            &format!("{component_name}/{locale}.ftl"),
            locale,
            component_name,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_locales_to_embed_no_lang() {
        unsafe {
            env::remove_var("LANG");
        }
        let (en_locale, system_locale) = get_locales_to_embed();
        assert_eq!(en_locale, "en-US");
        assert_eq!(system_locale, None);

        unsafe {
            env::set_var("LANG", "");
        }
        let (en_locale, system_locale) = get_locales_to_embed();
        assert_eq!(en_locale, "en-US");
        assert_eq!(system_locale, None);
        unsafe {
            env::remove_var("LANG");
        }

        unsafe {
            env::set_var("LANG", "en_US.UTF-8");
        }
        let (en_locale, system_locale) = get_locales_to_embed();
        assert_eq!(en_locale, "en-US");
        assert_eq!(system_locale, None);
        unsafe {
            env::remove_var("LANG");
        }
    }

    #[test]
    fn get_locales_to_embed_with_lang() {
        unsafe {
            env::set_var("LANG", "fr_FR.UTF-8");
        }
        let (en_locale, system_locale) = get_locales_to_embed();
        assert_eq!(en_locale, "en-US");
        assert_eq!(system_locale, Some("fr-FR".to_string()));
        unsafe {
            env::remove_var("LANG");
        }

        unsafe {
            env::set_var("LANG", "zh_CN.UTF-8");
        }
        let (en_locale, system_locale) = get_locales_to_embed();
        assert_eq!(en_locale, "en-US");
        assert_eq!(system_locale, Some("zh-CN".to_string()));
        unsafe {
            env::remove_var("LANG");
        }

        unsafe {
            env::set_var("LANG", "de");
        }
        let (en_locale, system_locale) = get_locales_to_embed();
        assert_eq!(en_locale, "en-US");
        assert_eq!(system_locale, Some("de".to_string()));
        unsafe {
            env::remove_var("LANG");
        }
    }

    #[test]
    fn get_locales_to_embed_invalid_lang() {
        // invalid locale format
        unsafe {
            env::set_var("LANG", "invalid");
        }
        let (en_locale, system_locale) = get_locales_to_embed();
        assert_eq!(en_locale, "en-US");
        assert_eq!(system_locale, Some("invalid".to_string()));
        unsafe {
            env::remove_var("LANG");
        }

        // numeric values
        unsafe {
            env::set_var("LANG", "123");
        }
        let (en_locale, system_locale) = get_locales_to_embed();
        assert_eq!(en_locale, "en-US");
        assert_eq!(system_locale, Some("123".to_string()));
        unsafe {
            env::remove_var("LANG");
        }

        // special characters
        unsafe {
            env::set_var("LANG", "@@@@");
        }
        let (en_locale, system_locale) = get_locales_to_embed();
        assert_eq!(en_locale, "en-US");
        assert_eq!(system_locale, Some("@@@@".to_string()));
        unsafe {
            env::remove_var("LANG");
        }

        // malformed locale (no country code but with encoding)
        unsafe {
            env::set_var("LANG", "en.UTF-8");
        }
        let (en_locale, system_locale) = get_locales_to_embed();
        assert_eq!(en_locale, "en-US");
        assert_eq!(system_locale, Some("en".to_string()));
        unsafe {
            env::remove_var("LANG");
        }

        // valid format but unusual locale
        unsafe {
            env::set_var("LANG", "XX_YY.UTF-8");
        }
        let (en_locale, system_locale) = get_locales_to_embed();
        assert_eq!(en_locale, "en-US");
        assert_eq!(system_locale, Some("XX-YY".to_string()));
        unsafe {
            env::remove_var("LANG");
        }
    }

    #[test]
    fn for_each_locale_basic() {
        let locales = ("en-US".to_string(), Some("fr-FR".to_string()));
        let mut collected = Vec::new();

        for_each_locale(&locales, |locale| {
            collected.push(locale.to_string());
            Ok(())
        })
        .unwrap();

        assert_eq!(collected, vec!["en-US", "fr-FR"]);
    }

    #[test]
    fn for_each_locale_no_system_locale() {
        let locales = ("en-US".to_string(), None);
        let mut collected = Vec::new();

        for_each_locale(&locales, |locale| {
            collected.push(locale.to_string());
            Ok(())
        })
        .unwrap();

        assert_eq!(collected, vec!["en-US"]);
    }

    #[test]
    fn for_each_locale_error_handling() {
        let locales = ("en-US".to_string(), Some("fr-FR".to_string()));

        let result = for_each_locale(&locales, |_locale| Err("test error".into()));

        assert!(result.is_err());
    }
}
