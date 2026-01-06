// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) krate mangen

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
                "default" | "macos" | "unix" | "windows" | "selinux" | "zip" | "clap_complete"
                | "clap_mangen" | "fluent_syntax" => continue, // common/standard feature names
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

    #[cfg(not(debug_assertions))]
    {
        copy_locales_release();
    }

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
                phf_map.entry("md5sum", map_value.clone());
                phf_map.entry("sha1sum", map_value.clone());
                phf_map.entry("sha224sum", map_value.clone());
                phf_map.entry("sha256sum", map_value.clone());
                phf_map.entry("sha384sum", map_value.clone());
                phf_map.entry("sha512sum", map_value.clone());
                phf_map.entry("b2sum", map_value.clone());
            }
            _ => {
                phf_map.entry(krate, map_value.clone());
            }
        }
    }
    write!(mf, "{}", phf_map.build()).unwrap();
    mf.write_all(b"\n}\n").unwrap();

    mf.flush().unwrap();
}

#[cfg(not(debug_assertions))]
fn copy_locales_release() {
    use std::path::PathBuf;
    let enabled_crates = env::var("CARGO_CFG_FEATURE").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let uu_dir = Path::new(&manifest_dir).join("src/uu");

    let target_dir = if let Ok(custom_target) = env::var("CARGO_TARGET_DIR") {
        PathBuf::from(custom_target)
    } else {
        Path::new(&manifest_dir).join("target")
    };

    let locales_dir = target_dir.join("release").join("locales");

    if !locales_dir.exists() {
        std::fs::create_dir_all(&locales_dir).expect("Failed to create locales directory");
    }

    for krate in enabled_crates.split(',') {
        let uu_crate_locales_path = uu_dir.join(krate).join("locales");

        if uu_crate_locales_path.exists() && uu_crate_locales_path.is_dir() {
            let crate_locales_dir = locales_dir.join(krate);
            if !crate_locales_dir.exists() {
                std::fs::create_dir_all(&crate_locales_dir)
                    .unwrap_or_else(|_| panic!("Failed to create directory for crate {krate}"));
            }

            match uu_crate_locales_path.read_dir() {
                Ok(read_dir) => {
                    for entry_result in read_dir {
                        match entry_result {
                            Ok(entry) => {
                                let file_name = entry.file_name();
                                let source_path = entry.path();
                                let dest_path = crate_locales_dir.join(&file_name);

                                if let Err(err) = std::fs::copy(&source_path, &dest_path) {
                                    eprintln!(
                                        "Failed to copy {:?} to {:?}: {}",
                                        source_path, dest_path, err
                                    );
                                } else {
                                    println!(
                                        "Copied locale file: {} -> {}",
                                        source_path.display(),
                                        dest_path.display()
                                    );
                                }
                            }
                            Err(err) => eprintln!("Error reading directory entry: {:?}", err),
                        }
                    }
                }
                Err(err) => eprintln!("Error reading locales directory for {}: {:?}", krate, err),
            }
        }
    }
}
