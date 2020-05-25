// spell-checker:ignore (vars) krate

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn main() {
    // println!("cargo:warning=Running build.rs");

    if let Ok(profile) = env::var("PROFILE") {
        println!("cargo:rustc-cfg=build={:?}", profile);
    }

    let env_feature_prefix: &str = "CARGO_FEATURE_";
    let feature_prefix: &str = "feat_";
    let override_prefix: &str = "uu_";

    let out_dir = env::var("OUT_DIR").unwrap();
    // println!("cargo:warning=out_dir={}", out_dir);
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap().replace("\\", "/");
    // println!("cargo:warning=manifest_dir={}", manifest_dir);

    let mut crates = Vec::new();
    for (key, val) in env::vars() {
        if val == "1" && key.starts_with(env_feature_prefix) {
            let krate = key[env_feature_prefix.len()..].to_lowercase();
            match krate.as_ref() {
                "default" | "macos" | "unix" | "windows" => continue, // common/standard feature names
                "nightly" | "test_unimplemented" => continue,         // crate-local custom features
                "test" => continue, // over-ridden with 'uu_test' to avoid collision with rust core crate 'test'
                s if s.starts_with(feature_prefix) => continue, // crate feature sets
                _ => {}             // util feature name
            }
            crates.push(krate.to_string());
        }
    }
    crates.sort();

    let mut mf = File::create(Path::new(&out_dir).join("uutils_map.rs")).unwrap();
    let mut tf = File::create(Path::new(&out_dir).join("test_modules.rs")).unwrap();

    mf.write_all(
        "type UtilityMap = HashMap<&'static str, fn(Vec<String>) -> i32>;\n\
        \n\
        fn util_map() -> UtilityMap {\n\
        \tlet mut map: UtilityMap = HashMap::new();\n\
        "
        .as_bytes(),
    )
    .unwrap();

    for krate in crates {
        match krate.as_ref() {
            k if k.starts_with(override_prefix) => {
                mf.write_all(
                    format!(
                        "\tmap.insert(\"{k}\", {krate}::uumain);\n",
                        k = krate[override_prefix.len()..].to_string(),
                        krate = krate
                    )
                    .as_bytes(),
                )
                .unwrap();
                tf.write_all(
                    format!(
                        "#[path=\"{dir}/tests/by-crate/test_{k}.rs\"]\nmod test_{k};\n",
                        k = krate[override_prefix.len()..].to_string(),
                        dir = manifest_dir,
                    )
                    .as_bytes(),
                )
                .unwrap()
            }
            "false" | "true" => {
                mf.write_all(
                    format!(
                        "\tmap.insert(\"{krate}\", r#{krate}::uumain);\n",
                        krate = krate
                    )
                    .as_bytes(),
                )
                .unwrap();
                tf.write_all(
                    format!(
                        "#[path=\"{dir}/tests/by-crate/test_{krate}.rs\"]\nmod test_{krate};\n",
                        krate = krate,
                        dir = manifest_dir,
                    )
                    .as_bytes(),
                )
                .unwrap()
            }
            "hashsum" => {
                mf.write_all(
                    format!(
                        "\
                        \tmap.insert(\"{krate}\", {krate}::uumain);\n\
                        \t\tmap.insert(\"md5sum\", {krate}::uumain);\n\
                        \t\tmap.insert(\"sha1sum\", {krate}::uumain);\n\
                        \t\tmap.insert(\"sha224sum\", {krate}::uumain);\n\
                        \t\tmap.insert(\"sha256sum\", {krate}::uumain);\n\
                        \t\tmap.insert(\"sha384sum\", {krate}::uumain);\n\
                        \t\tmap.insert(\"sha512sum\", {krate}::uumain);\n\
                        \t\tmap.insert(\"sha3sum\", {krate}::uumain);\n\
                        \t\tmap.insert(\"sha3-224sum\", {krate}::uumain);\n\
                        \t\tmap.insert(\"sha3-256sum\", {krate}::uumain);\n\
                        \t\tmap.insert(\"sha3-384sum\", {krate}::uumain);\n\
                        \t\tmap.insert(\"sha3-512sum\", {krate}::uumain);\n\
                        \t\tmap.insert(\"shake128sum\", {krate}::uumain);\n\
                        \t\tmap.insert(\"shake256sum\", {krate}::uumain);\n\
                        ",
                        krate = krate
                    )
                    .as_bytes(),
                )
                .unwrap();
                tf.write_all(
                    format!(
                        "#[path=\"{dir}/tests/by-crate/test_{krate}.rs\"]\nmod test_{krate};\n",
                        krate = krate,
                        dir = manifest_dir,
                    )
                    .as_bytes(),
                )
                .unwrap()
            }
            _ => {
                mf.write_all(
                    format!(
                        "\tmap.insert(\"{krate}\", {krate}::uumain);\n",
                        krate = krate
                    )
                    .as_bytes(),
                )
                .unwrap();
                tf.write_all(
                    format!(
                        "#[path=\"{dir}/tests/by-crate/test_{krate}.rs\"]\nmod test_{krate};\n",
                        krate = krate,
                        dir = manifest_dir,
                    )
                    .as_bytes(),
                )
                .unwrap()
            }
        }
    }

    mf.write_all(b"map\n}\n").unwrap();

    mf.flush().unwrap();
    tf.flush().unwrap();
}
