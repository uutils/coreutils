use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn main() {
    if let Ok(profile) = env::var("PROFILE") {
        println!("cargo:rustc-cfg=build={:?}", profile);
    }

    let feature_prefix = "CARGO_FEATURE_";
    let out_dir = env::var("OUT_DIR").unwrap();

    let mut crates = Vec::new();
    for (key, val) in env::vars() {
        if val == "1" && key.starts_with(feature_prefix) {
            let krate = key[feature_prefix.len()..].to_lowercase();
            match krate.as_ref() {
                "default" | "unix" | "redox" | "fuchsia" | "generic" | "nightly" | "test_unimplemented" => continue,
                _ => {},
            }
            crates.push(krate.to_string());
        }
    }
    crates.sort();

    let mut cf = File::create(Path::new(&out_dir).join("uutils_crates.rs")).unwrap();
    let mut mf = File::create(Path::new(&out_dir).join("uutils_map.rs")).unwrap();

    mf.write_all("
    type UtilityMap = HashMap<&'static str, fn(Vec<String>) -> i32>;

    fn util_map() -> UtilityMap {
    let mut map: UtilityMap = HashMap::new();\n".as_bytes()).unwrap();

    for krate in crates {
        cf.write_all(format!("extern crate uu_{krate};\n", krate=krate).as_bytes()).unwrap();

        match krate.as_ref() {
            "hashsum" => {
                mf.write_all("map.insert(\"hashsum\", uu_hashsum::uumain);
                              map.insert(\"md5sum\", uu_hashsum::uumain);
                              map.insert(\"sha1sum\", uu_hashsum::uumain);
                              map.insert(\"sha224sum\", uu_hashsum::uumain);
                              map.insert(\"sha256sum\", uu_hashsum::uumain);
                              map.insert(\"sha384sum\", uu_hashsum::uumain);
                              map.insert(\"sha512sum\", uu_hashsum::uumain);
                              map.insert(\"sha3sum\", uu_hashsum::uumain);
                              map.insert(\"sha3-224sum\", uu_hashsum::uumain);
                              map.insert(\"sha3-256sum\", uu_hashsum::uumain);
                              map.insert(\"sha3-384sum\", uu_hashsum::uumain);
                              map.insert(\"sha3-512sum\", uu_hashsum::uumain);
                              map.insert(\"shake128sum\", uu_hashsum::uumain);
                              map.insert(\"shake256sum\", uu_hashsum::uumain);\n".as_bytes()).unwrap();
            },
            _ =>
                mf.write_all(format!("map.insert(\"{krate}\", uu_{krate}::uumain);\n", krate=krate).as_bytes()).unwrap(),
        }
    }

    mf.write_all("map\n}\n".as_bytes()).unwrap();

    cf.flush().unwrap();
    mf.flush().unwrap();
}
