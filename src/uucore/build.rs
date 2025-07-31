// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::env;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = env::var("OUT_DIR").unwrap();

    // Always generate embedded English locale files for fallback
    generate_embedded_english_locales(&out_dir).unwrap();
}

/// Generate embedded English locale files
///
/// # Errors
///
/// Returns an error if file operations fail or if there are I/O issues
fn generate_embedded_english_locales(out_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::{self, File};
    use std::io::Write;

    // Since we're in uucore, we need to go up to the project root
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // src/
        .and_then(|p| p.parent()) // project root
        .ok_or("Failed to find project root")?;

    let mut embedded_file = File::create(Path::new(out_dir).join("embedded_locales.rs"))?;

    writeln!(embedded_file, "// Generated at compile time - do not edit")?;
    writeln!(
        embedded_file,
        "// This file contains embedded English locale files for all utilities"
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

    // Scan for all utilities in src/uu/
    let uu_dir = project_root.join("src/uu");
    for entry in fs::read_dir(&uu_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let util_name = entry.file_name().to_string_lossy().to_string();
            let locale_path = path.join("locales/en-US.ftl");
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

    writeln!(embedded_file)?;
    writeln!(embedded_file, "    locales")?;
    writeln!(embedded_file, "}}")?;

    embedded_file.flush()?;
    Ok(())
}
