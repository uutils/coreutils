// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (vars) krate mangen tldr

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

    // Check for tldr.zip when building uudoc to warn users once at build time
    // instead of repeatedly at runtime for each utility
    if env::var("CARGO_FEATURE_UUDOC").is_ok() && !Path::new("docs/tldr.zip").exists() {
        println!(
            "cargo:warning=No tldr archive found, so the documentation will not include examples."
        );
        println!("cargo:warning=To include examples, download the tldr archive:");
        println!(
            "cargo:warning=  curl -L https://github.com/tldr-pages/tldr/releases/latest/download/tldr.zip -o docs/tldr.zip"
        );
    }

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
         fn util_map<T: Args>() -> UtilityMap<T> {\n"
            .as_bytes(),
    )
    .unwrap();

    let mut phf_map = phf_codegen::OrderedMap::<&str>::new();
    let mut entries = Vec::new();

    for krate in &crates {
        let map_value = format!("({krate}::uumain, {krate}::uu_app)");
        match krate.as_ref() {
            // 'test' is named uu_test to avoid collision with rust core crate 'test'.
            // It can also be invoked by name '[' for the '[ expr ] syntax'.
            "uu_test" => {
                entries.push(("test", map_value.clone()));
                entries.push(("[", map_value.clone()));
            }
            k if k.starts_with(OVERRIDE_PREFIX) => {
                entries.push((&k[OVERRIDE_PREFIX.len()..], map_value.clone()));
            }
            "false" | "true" => {
                entries.push((
                    krate.as_str(),
                    format!("(r#{krate}::uumain, r#{krate}::uu_app)"),
                ));
            }
            _ => {
                entries.push((krate.as_str(), map_value.clone()));
            }
        }
    }
    entries.sort_by_key(|(name, _)| *name);

    for (name, value) in entries {
        phf_map.entry(name, value);
    }

    write!(mf, "{}", phf_map.build()).unwrap();
    mf.write_all(b"\n}\n").unwrap();

    mf.flush().unwrap();
}
