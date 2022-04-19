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

    const ENV_FEATURE_PREFIX: &str = "CARGO_FEATURE_";
    const FEATURE_PREFIX: &str = "feat_";
    const OVERRIDE_PREFIX: &str = "uu_";

    let out_dir = env::var("OUT_DIR").unwrap();
    // println!("cargo:warning=out_dir={}", out_dir);
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap().replace('\\', "/");
    // println!("cargo:warning=manifest_dir={}", manifest_dir);
    let util_tests_dir = format!("{}/tests/by-util", manifest_dir);
    // println!("cargo:warning=util_tests_dir={}", util_tests_dir);

    let mut crates = Vec::new();
    for (key, val) in env::vars() {
        if val == "1" && key.starts_with(ENV_FEATURE_PREFIX) {
            let krate = key[ENV_FEATURE_PREFIX.len()..].to_lowercase();
            match krate.as_ref() {
                "default" | "macos" | "unix" | "windows" | "selinux" => continue, // common/standard feature names
                "nightly" | "test_unimplemented" => continue, // crate-local custom features
                "test" => continue, // over-ridden with 'uu_test' to avoid collision with rust core crate 'test'
                s if s.starts_with(FEATURE_PREFIX) => continue, // crate feature sets
                _ => {}             // util feature name
            }
            crates.push(krate);
        }
    }
    crates.sort();

    let mut mf = File::create(Path::new(&out_dir).join("uutils_map.rs")).unwrap();
    let mut tf = File::create(Path::new(&out_dir).join("test_modules.rs")).unwrap();

    mf.write_all(
        "type UtilityMap<T> = phf::Map<&'static str, (fn(T) -> i32, fn() -> Command<'static>)>;\n\
         \n\
         fn util_map<T: uucore::Args>() -> UtilityMap<T> {\n"
            .as_bytes(),
    )
    .unwrap();

    let mut phf_map = phf_codegen::Map::<&str>::new();
    for krate in &crates {
        let map_value = format!("({krate}::uumain, {krate}::uu_app)", krate = krate);
        match krate.as_ref() {
            // 'test' is named uu_test to avoid collision with rust core crate 'test'.
            // It can also be invoked by name '[' for the '[ expr ] syntax'.
            "uu_test" => {
                phf_map.entry("test", &map_value);
                phf_map.entry("[", &map_value);

                tf.write_all(
                    format!(
                        "#[path=\"{dir}/test_test.rs\"]\nmod test_test;\n",
                        dir = util_tests_dir,
                    )
                    .as_bytes(),
                )
                .unwrap();
            }
            k if k.starts_with(OVERRIDE_PREFIX) => {
                phf_map.entry(&k[OVERRIDE_PREFIX.len()..], &map_value);
                tf.write_all(
                    format!(
                        "#[path=\"{dir}/test_{k}.rs\"]\nmod test_{k};\n",
                        k = &krate[OVERRIDE_PREFIX.len()..],
                        dir = util_tests_dir,
                    )
                    .as_bytes(),
                )
                .unwrap();
            }
            "false" | "true" => {
                phf_map.entry(
                    krate,
                    &format!("(r#{krate}::uumain, r#{krate}::uu_app)", krate = krate),
                );
                tf.write_all(
                    format!(
                        "#[path=\"{dir}/test_{krate}.rs\"]\nmod test_{krate};\n",
                        krate = krate,
                        dir = util_tests_dir,
                    )
                    .as_bytes(),
                )
                .unwrap();
            }
            "hashsum" => {
                phf_map.entry(
                    krate,
                    &format!("({krate}::uumain, {krate}::uu_app_custom)", krate = krate),
                );

                let map_value = format!("({krate}::uumain, {krate}::uu_app_common)", krate = krate);
                phf_map.entry("md5sum", &map_value);
                phf_map.entry("sha1sum", &map_value);
                phf_map.entry("sha224sum", &map_value);
                phf_map.entry("sha256sum", &map_value);
                phf_map.entry("sha384sum", &map_value);
                phf_map.entry("sha512sum", &map_value);
                phf_map.entry("sha3sum", &map_value);
                phf_map.entry("sha3-224sum", &map_value);
                phf_map.entry("sha3-256sum", &map_value);
                phf_map.entry("sha3-384sum", &map_value);
                phf_map.entry("sha3-512sum", &map_value);
                phf_map.entry("shake128sum", &map_value);
                phf_map.entry("shake256sum", &map_value);
                phf_map.entry("b2sum", &map_value);
                phf_map.entry("b3sum", &map_value);
                tf.write_all(
                    format!(
                        "#[path=\"{dir}/test_{krate}.rs\"]\nmod test_{krate};\n",
                        krate = krate,
                        dir = util_tests_dir,
                    )
                    .as_bytes(),
                )
                .unwrap();
            }
            _ => {
                phf_map.entry(krate, &map_value);
                tf.write_all(
                    format!(
                        "#[path=\"{dir}/test_{krate}.rs\"]\nmod test_{krate};\n",
                        krate = krate,
                        dir = util_tests_dir,
                    )
                    .as_bytes(),
                )
                .unwrap();
            }
        }
    }
    write!(mf, "{}", phf_map.build()).unwrap();
    mf.write_all(b"\n}\n").unwrap();

    mf.flush().unwrap();
    tf.flush().unwrap();
}
