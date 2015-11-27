use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn main() {
    let feature_prefix = "CARGO_FEATURE_";
    let out_dir = env::var("OUT_DIR").unwrap();

    let mut crates = Vec::new();
    for (key, val) in env::vars() {
        if val == "1" && key.starts_with(feature_prefix) {
            let krate = key[feature_prefix.len()..].to_lowercase();
            match krate.as_ref() {
            "default" => continue,
            "all" => continue,
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
        match krate.as_ref() {
            "false" | "true" | "test" => {},
            _ => cf.write_all(format!("extern crate {krate} as uu{krate};\n", krate=krate).as_bytes()).unwrap(),
        }

        match krate.as_ref() {
            "hashsum" => {
                mf.write_all("map.insert(\"hashsum\", uuhashsum::uumain);
                              map.insert(\"md5sum\", uuhashsum::uumain);
                              map.insert(\"sha1sum\", uuhashsum::uumain);
                              map.insert(\"sha224sum\", uuhashsum::uumain);
                              map.insert(\"sha256sum\", uuhashsum::uumain);
                              map.insert(\"sha384sum\", uuhashsum::uumain);
                              map.insert(\"sha512sum\", uuhashsum::uumain);\n".as_bytes()).unwrap();
            },
            "true" => {
                mf.write_all(format!("fn uu{}", krate).as_bytes()).unwrap();
                mf.write_all("(_: Vec<String>) -> i32 { 0 }\n".as_bytes()).unwrap();
                mf.write_all(format!("map.insert(\"{krate}\", uu{krate} as fn(Vec<String>) -> i32);\n", krate=krate).as_bytes()).unwrap();
            }
            "false" | "test" => {
                mf.write_all(format!("fn uu{}", krate).as_bytes()).unwrap();
                mf.write_all("(_: Vec<String>) -> i32 { 1 }\n".as_bytes()).unwrap();
                mf.write_all(format!("map.insert(\"{krate}\", uu{krate} as fn(Vec<String>) -> i32);\n", krate=krate).as_bytes()).unwrap();
            },
            _ => 
                mf.write_all(format!("map.insert(\"{krate}\", uu{krate}::uumain as fn(Vec<String>) -> i32);\n", krate= krate).as_bytes()).unwrap(),
        }
    }
    mf.write_all("map
    }\n".as_bytes()).unwrap();
    cf.flush().unwrap();
    mf.flush().unwrap();
}
