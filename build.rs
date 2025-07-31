// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) krate

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn main() {
    const ENV_FEATURE_PREFIX: &str = "CARGO_FEATURE_";
    const FEATURE_PREFIX: &str = "feat_";
    const OVERRIDE_PREFIX: &str = "uu_";

    // Do not rebuild build script unless the script itself or the enabled features are modified
    // See <https://doc.rust-lang.org/cargo/reference/build-scripts.html#change-detection>
    println!("cargo:rerun-if-changed=build.rs");

    if let Ok(profile) = env::var("PROFILE") {
        println!("cargo:rustc-cfg=build={profile:?}");
    }

    let out_dir = env::var("OUT_DIR").unwrap();

    let mut crates = Vec::new();
    for (key, val) in env::vars() {
        if val == "1" && key.starts_with(ENV_FEATURE_PREFIX) {
            let krate = key[ENV_FEATURE_PREFIX.len()..].to_lowercase();
            // Allow this as we have a bunch of info in the comments
            #[allow(clippy::match_same_arms)]
            match krate.as_ref() {
                "default" | "macos" | "unix" | "windows" | "selinux" | "zip" => continue, // common/standard feature names
                "nightly" | "test_unimplemented" | "expensive_tests" | "test_risky_names" => {
                    continue;
                } // crate-local custom features
                "uudoc" => continue, // is not a utility
                "test" => continue, // over-ridden with 'uu_test' to avoid collision with rust core crate 'test'
                s if s.starts_with(FEATURE_PREFIX) => continue, // crate feature sets
                _ => {}             // util feature name
            }
            crates.push(krate);
        }
    }
    crates.sort();

    let mut mf = File::create(Path::new(&out_dir).join("uutils_map.rs")).unwrap();

    mf.write_all(
        "type UtilityMap<T> = phf::OrderedMap<&'static str, (fn(T) -> i32, fn() -> Command)>;\n\
         \n\
         #[allow(clippy::too_many_lines)]
         #[allow(clippy::unreadable_literal)]
         fn util_map<T: uucore::Args>() -> UtilityMap<T> {\n"
            .as_bytes(),
    )
    .unwrap();

    let mut phf_map = phf_codegen::OrderedMap::<&str>::new();
    for krate in &crates {
        let map_value = format!("({krate}::uumain, {krate}::uu_app)");
        match krate.as_ref() {
            // 'test' is named uu_test to avoid collision with rust core crate 'test'.
            // It can also be invoked by name '[' for the '[ expr ] syntax'.
            "uu_test" => {
                phf_map.entry("test", map_value.clone());
                phf_map.entry("[", map_value.clone());
            }
            k if k.starts_with(OVERRIDE_PREFIX) => {
                phf_map.entry(&k[OVERRIDE_PREFIX.len()..], map_value.clone());
            }
            "false" | "true" => {
                phf_map.entry(krate, format!("(r#{krate}::uumain, r#{krate}::uu_app)"));
            }
            "hashsum" => {
                phf_map.entry(krate, format!("({krate}::uumain, {krate}::uu_app_custom)"));

                let map_value = format!("({krate}::uumain, {krate}::uu_app_common)");
                let map_value_bits = format!("({krate}::uumain, {krate}::uu_app_bits)");
                let map_value_b3sum = format!("({krate}::uumain, {krate}::uu_app_b3sum)");
                phf_map.entry("md5sum", map_value.clone());
                phf_map.entry("sha1sum", map_value.clone());
                phf_map.entry("sha224sum", map_value.clone());
                phf_map.entry("sha256sum", map_value.clone());
                phf_map.entry("sha384sum", map_value.clone());
                phf_map.entry("sha512sum", map_value.clone());
                phf_map.entry("sha3sum", map_value_bits.clone());
                phf_map.entry("sha3-224sum", map_value.clone());
                phf_map.entry("sha3-256sum", map_value.clone());
                phf_map.entry("sha3-384sum", map_value.clone());
                phf_map.entry("sha3-512sum", map_value.clone());
                phf_map.entry("shake128sum", map_value_bits.clone());
                phf_map.entry("shake256sum", map_value_bits.clone());
                phf_map.entry("b2sum", map_value.clone());
                phf_map.entry("b3sum", map_value_b3sum);
            }
            _ => {
                phf_map.entry(krate, map_value.clone());
            }
        }
    }
    write!(mf, "{}", phf_map.build()).unwrap();
    mf.write_all(b"\n}\n").unwrap();

    mf.flush().unwrap();

    // Always generate embedded English locale files
    // This ensures English works even when locale files aren't available (e.g., cargo install)
    generate_embedded_english_locales(&out_dir, &crates).unwrap();
}

/// Generate embedded English locale files
///
/// # Errors
///
/// Returns an error if file operations fail or if there are I/O issues
fn generate_embedded_english_locales(
    out_dir: &str,
    crates: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    let mut embedded_file = File::create(Path::new(&out_dir).join("embedded_locales.rs"))?;

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

    // Embed locale files for each utility
    for krate in crates {
        let util_name = if let Some(stripped) = krate.strip_prefix("uu_") {
            stripped
        } else {
            krate
        };

        let locale_path = Path::new("src/uu")
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
    }

    // Also embed uucore locale file if it exists
    let uucore_locale_path = Path::new("src/uucore/locales/en-US.ftl");
    if uucore_locale_path.exists() {
        let content = fs::read_to_string(uucore_locale_path)?;
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
