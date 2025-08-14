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
    writeln!(embedded_file, "use std::collections::HashMap;")?;
    writeln!(embedded_file)?;

    // Start the function that returns embedded locales
    writeln!(
        embedded_file,
        "pub fn get_embedded_locales() -> HashMap<&'static str, &'static str> {{"
    )?;
    writeln!(embedded_file, "    let mut locales = HashMap::new();")?;
    writeln!(embedded_file)?;

    // Try to detect if we're building for a specific utility by checking build configuration
    // This attempts to identify individual utility builds vs multicall binary builds
    let target_utility = detect_target_utility();

    match target_utility {
        Some(util_name) => {
            // Embed only the specific utility's locale (cat.ftl for cat for example)
            embed_single_utility_locale(&mut embedded_file, &project_root()?, &util_name)?;
        }
        None => {
            // Embed all utilities locales (multicall binary or fallback)
            embed_all_utilities_locales(&mut embedded_file, &project_root()?)?;
        }
    }

    writeln!(embedded_file)?;
    writeln!(embedded_file, "    locales")?;
    writeln!(embedded_file, "}}")?;

    embedded_file.flush()?;
    Ok(())
}

/// Get the project root directory
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

    // First check if an explicit environment variable was set
    if let Ok(target_util) = env::var("UUCORE_TARGET_UTIL")
        && !target_util.is_empty()
    {
        return Some(target_util);
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
fn embed_single_utility_locale(
    embedded_file: &mut std::fs::File,
    project_root: &Path,
    util_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    // Embed the specific utility's locale
    let locale_path = project_root
        .join("src/uu")
        .join(util_name)
        .join("locales/en-US.ftl");

    if locale_path.exists() {
        let content = fs::read_to_string(&locale_path)?;
        writeln!(embedded_file, "    // Locale for {util_name}")?;
        writeln!(
            embedded_file,
            "    locales.insert(\"{util_name}/en-US.ftl\", r###\"{content}\"###);"
        )?;
        writeln!(embedded_file)?;

        // Tell Cargo to rerun if this file changes
        println!("cargo:rerun-if-changed={}", locale_path.display());
    }

    // Always embed uucore locale file if it exists
    let uucore_locale_path = project_root.join("src/uucore/locales/en-US.ftl");
    if uucore_locale_path.exists() {
        let content = fs::read_to_string(&uucore_locale_path)?;
        writeln!(embedded_file, "    // Common uucore locale")?;
        writeln!(
            embedded_file,
            "    locales.insert(\"uucore/en-US.ftl\", r###\"{content}\"###);"
        )?;
        println!("cargo:rerun-if-changed={}", uucore_locale_path.display());
    }

    Ok(())
}

/// Embed locale files for all utilities (multicall binary)
fn embed_all_utilities_locales(
    embedded_file: &mut std::fs::File,
    project_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    // Discover all uu_* directories
    let src_uu_dir = project_root.join("src/uu");
    if !src_uu_dir.exists() {
        return Ok(());
    }

    let mut util_dirs = Vec::new();
    for entry in fs::read_dir(&src_uu_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && let Some(dir_name) = entry.file_name().to_str()
        {
            util_dirs.push(dir_name.to_string());
        }
    }
    util_dirs.sort();

    // Embed locale files for each utility
    for util_name in &util_dirs {
        let locale_path = src_uu_dir.join(util_name).join("locales/en-US.ftl");
        if locale_path.exists() {
            let content = fs::read_to_string(&locale_path)?;
            writeln!(embedded_file, "    // Locale for {util_name}")?;
            writeln!(
                embedded_file,
                "    locales.insert(\"{util_name}/en-US.ftl\", r###\"{content}\"###);"
            )?;
            writeln!(embedded_file)?;

            // Tell Cargo to rerun if this file changes
            println!("cargo:rerun-if-changed={}", locale_path.display());
        }
    }

    // Also embed uucore locale file if it exists
    let uucore_locale_path = project_root.join("src/uucore/locales/en-US.ftl");
    if uucore_locale_path.exists() {
        let content = fs::read_to_string(&uucore_locale_path)?;
        writeln!(embedded_file, "    // Common uucore locale")?;
        writeln!(
            embedded_file,
            "    locales.insert(\"uucore/en-US.ftl\", r###\"{content}\"###);"
        )?;
        println!("cargo:rerun-if-changed={}", uucore_locale_path.display());
    }

    embedded_file.flush()?;
    Ok(())
}
