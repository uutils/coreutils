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
    if let Ok(profile) = env::var("PROFILE") {
        println!("cargo:rustc-cfg=build={profile:?}");
    }

    const ENV_FEATURE_PREFIX: &str = "CARGO_FEATURE_";
    const FEATURE_PREFIX: &str = "feat_";
    const OVERRIDE_PREFIX: &str = "uu_";

    let out_dir = env::var("OUT_DIR").unwrap();

    let mut crates = Vec::new();
    for (key, val) in env::vars() {
        if val == "1" && key.starts_with(ENV_FEATURE_PREFIX) {
            let krate = key[ENV_FEATURE_PREFIX.len()..].to_lowercase();
            // Allow this as we have a bunch of info in the comments
            #[allow(clippy::match_same_arms)]
            match krate.as_ref() {
                "default" | "macos" | "unix" | "windows" | "selinux" | "zip" => continue, // common/standard feature names
                "nightly" | "test_unimplemented" => continue, // crate-local custom features
                "uudoc" => continue,                          // is not a utility
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
                phf_map.entry("test", &map_value);
                phf_map.entry("[", &map_value);
            }
            k if k.starts_with(OVERRIDE_PREFIX) => {
                phf_map.entry(&k[OVERRIDE_PREFIX.len()..], &map_value);
            }
            "false" | "true" => {
                phf_map.entry(krate, &format!("(r#{krate}::uumain, r#{krate}::uu_app)"));
            }
            "hashsum" => {
                phf_map.entry(krate, &format!("({krate}::uumain, {krate}::uu_app_custom)"));

                let map_value = format!("({krate}::uumain, {krate}::uu_app_common)");
                let map_value_bits = format!("({krate}::uumain, {krate}::uu_app_bits)");
                let map_value_b3sum = format!("({krate}::uumain, {krate}::uu_app_b3sum)");
                phf_map.entry("md5sum", &map_value);
                phf_map.entry("sha1sum", &map_value);
                phf_map.entry("sha224sum", &map_value);
                phf_map.entry("sha256sum", &map_value);
                phf_map.entry("sha384sum", &map_value);
                phf_map.entry("sha512sum", &map_value);
                phf_map.entry("sha3sum", &map_value_bits);
                phf_map.entry("sha3-224sum", &map_value);
                phf_map.entry("sha3-256sum", &map_value);
                phf_map.entry("sha3-384sum", &map_value);
                phf_map.entry("sha3-512sum", &map_value);
                phf_map.entry("shake128sum", &map_value_bits);
                phf_map.entry("shake256sum", &map_value_bits);
                phf_map.entry("b2sum", &map_value);
                phf_map.entry("b3sum", &map_value_b3sum);
            }
            _ => {
                phf_map.entry(krate, &map_value);
            }
        }
    }
    write!(mf, "{}", phf_map.build()).unwrap();
    mf.write_all(b"\n}\n").unwrap();

    mf.flush().unwrap();
}
