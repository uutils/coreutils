// spell-checker:ignore (utils) chgrp chmod chown chroot cksum dircolors hashsum hostid logname mkdir mkfifo mknod mktemp nohup nproc numfmt pathchk printenv printf readlink realpath relpath rmdir shuf stdbuf tsort uname unexpand whoami
// spell-checker:ignore () uutils uumain rustfmt rustc macos krate

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
                "default" | "macos" | "unix" | "windows" => continue,
                "nightly" | "test_unimplemented" => continue,
                s if s.starts_with("feat_") => continue,
                _ => {}
            }
            crates.push(krate.to_string());
        }
    }
    crates.sort();

    let mut cf = File::create(Path::new(&out_dir).join("uutils_crates.rs")).unwrap();
    let mut mf = File::create(Path::new(&out_dir).join("uutils_map.rs")).unwrap();

    mf.write_all(
        "
    type UtilityMap = HashMap<&'static str, fn(Vec<String>) -> i32>;

    fn util_map() -> UtilityMap {
    let mut map: UtilityMap = HashMap::new();\n"
            .as_bytes(),
    )
    .unwrap();

    for krate in crates {
        match krate.as_ref() {
            "arch"
            | "base32" | "base64" | "basename"
            | "cat" | "chgrp" | "chmod" | "chown" | "chroot" | "cksum" | "comm" | "cp" | "cut"
            | "date" | "df" | "dircolors" | "dirname" | "du"
            | "echo" | "env" | "expand" | "expr"
            | "factor" | "fmt" | "fold"
            | "groups"
            | "head" | "hostid" | "hostname"
            | "id" | "install"
            | "join"
            | "kill"
            | "link" | "ln" | "logname" | "ls"
            | "mkdir" | "mkfifo" | "mknod" | "mktemp" | "more" | "mv"
            | "nice" | "nl" | "nohup" | "nproc" | "numfmt"
            | "od"
            | "paste" | "pathchk" | "pinky" | "printenv" | "printf" | "ptx" | "pwd"
            | "readlink" | "realpath" | "relpath" | "rm" | "rmdir"
            | "seq" | "shred" | "shuf" | "sleep" | "sort" | "split" | "stat" | "stdbuf" | "sum" | "sync"
            | "tac" | "tail" | "tee" | "test" | "timeout" | "touch" | "tr" | "truncate" | "tsort" | "tty"
            | "uname" | "unexpand" | "uniq" | "unlink" | "uptime" | "users"
            | "wc" | "who" | "whoami"
            | "yes"
            | "false" | "true"
            | "hashsum"
            | "uu_test"
            => {
                // cf.write_all(format!("extern crate {krate};\n", krate = krate).as_bytes())
                //     .unwrap();
            }
            _ => {
                cf.write_all(format!("extern crate uu_{krate};\n", krate = krate).as_bytes())
                    .unwrap();
            }
        }

        match krate.as_ref() {
            // * use "uu_" prefix as bypass method to avoid name collisions with imported crates, when necessary (eg, 'test')
            k if k.starts_with("uu_")
                => mf
                    .write_all(
                        format!("map.insert(\"{k}\", {krate}::uumain);\n", k = krate.clone().remove("uu_".len()), krate = krate)
                            .as_bytes(),
                    )
                    .unwrap(),
            "arch"
            | "base32" | "base64" | "basename"
            | "cat" | "chgrp" | "chmod" | "chown" | "chroot" | "cksum" | "comm" | "cp" | "cut"
            | "date" | "df" | "dircolors" | "dirname" | "du"
            | "echo" | "env" | "expand" | "expr"
            | "factor" | "fmt" | "fold"
            | "groups"
            | "head" | "hostid" | "hostname"
            | "id" | "install"
            | "join"
            | "kill"
            | "link" | "ln" | "logname" | "ls"
            | "mkdir" | "mkfifo" | "mknod" | "mktemp" | "more" | "mv"
            | "nice" | "nl" | "nohup" | "nproc" | "numfmt"
            | "od"
            | "paste" | "pathchk" | "pinky" | "printenv" | "printf" | "ptx" | "pwd"
            | "readlink" | "realpath" | "relpath" | "rm" | "rmdir"
            | "seq" | "shred" | "shuf" | "sleep" | "sort" | "split" | "stat" | "stdbuf" | "sum" | "sync"
            | "tac" | "tail" | "tee" | "timeout" | "touch" | "tr" | "truncate" | "tsort" | "tty"
            | "uname" | "unexpand" | "uniq" | "unlink" | "uptime" | "users"
            | "wc" | "who" | "whoami"
            | "yes"
            => mf
                .write_all(
                    format!("map.insert(\"{krate}\", {krate}::uumain);\n", krate = krate)
                        .as_bytes(),
                )
                .unwrap(),
            "false" | "true" => mf
                .write_all(
                    format!("map.insert(\"{krate}\", r#{krate}::uumain);\n", krate = krate)
                        .as_bytes(),
                )
                .unwrap(),
            "hashsum" => mf
                .write_all(
                    format!(
                        "
                        map.insert(\"{krate}\", {krate}::uumain);
                            map.insert(\"md5sum\", {krate}::uumain);
                            map.insert(\"sha1sum\", {krate}::uumain);
                            map.insert(\"sha224sum\", {krate}::uumain);
                            map.insert(\"sha256sum\", {krate}::uumain);
                            map.insert(\"sha384sum\", {krate}::uumain);
                            map.insert(\"sha512sum\", {krate}::uumain);
                            map.insert(\"sha3sum\", {krate}::uumain);
                            map.insert(\"sha3-224sum\", {krate}::uumain);
                            map.insert(\"sha3-256sum\", {krate}::uumain);
                            map.insert(\"sha3-384sum\", {krate}::uumain);
                            map.insert(\"sha3-512sum\", {krate}::uumain);
                            map.insert(\"shake128sum\", {krate}::uumain);
                            map.insert(\"shake256sum\", {krate}::uumain);\n",
                        krate = krate
                    )
                    .as_bytes(),
                )
                .unwrap(),
            _ => mf
                .write_all(
                    format!(
                        "map.insert(\"{krate}\", uu_{krate}::uumain);\n",
                        krate = krate
                    )
                    .as_bytes(),
                )
                .unwrap(),
        }
    }

    mf.write_all(b"map\n}\n").unwrap();

    cf.flush().unwrap();
    mf.flush().unwrap();
}
