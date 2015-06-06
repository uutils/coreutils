use std::env;
use std::fs::File;
use std::io::{Read, Write};

fn main() {
    let args : Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("usage: mkuutils <outfile> <crates>");
        std::process::exit(1);
    }

    let mut crates = String::new();
    let mut util_map = String::new();
    let mut hashsum = false;
    for prog in args[2..].iter() {
        match &prog[..] {
            "hashsum" | "md5sum" | "sha1sum" | "sha224sum" | "sha256sum" | "sha384sum" | "sha512sum" => {
                if !hashsum {
                    crates.push_str("extern crate hashsum;\n");
                    util_map.push_str("map.insert(\"hashsum\", hashsum::uumain);\n");
                    util_map.push_str("map.insert(\"md5sum\", hashsum::uumain);\n");
                    util_map.push_str("map.insert(\"sha1sum\", hashsum::uumain);\n");
                    util_map.push_str("map.insert(\"sha224sum\", hashsum::uumain);\n");
                    util_map.push_str("map.insert(\"sha256sum\", hashsum::uumain);\n");
                    util_map.push_str("map.insert(\"sha384sum\", hashsum::uumain);\n");
                    util_map.push_str("map.insert(\"sha512sum\", hashsum::uumain);\n");
                    hashsum = true;
                }
            },
            "true" => {
                util_map.push_str("fn uutrue(_: Vec<String>) -> i32 { 0 }\n");
                util_map.push_str("map.insert(\"true\", uutrue as fn(Vec<String>) -> i32);\n");
            },
            "false" => {
                util_map.push_str("fn uufalse(_: Vec<String>) -> i32 { 1 }\n");
                util_map.push_str("map.insert(\"false\", uufalse as fn(Vec<String>) -> i32);\n");
            },
            _ => {
                if prog == "test" {
                    crates.push_str(&(format!("extern crate uu{0} as uu{0};\n", prog))[..]);
                } else {
                    crates.push_str(&(format!("extern crate {0} as uu{0};\n", prog))[..]);
                }
                util_map.push_str(&(format!("map.insert(\"{prog}\", uu{prog}::uumain as fn(Vec<String>) -> i32);\n", prog = prog))[..]);
            }
        }
    }
    let outfile = &(args[1])[..];

    // XXX: this all just assumes that the IO works correctly
    let mut out = File::create(outfile).unwrap();
    let mut input = File::open("src/uutils/uutils.rs").unwrap();

    let mut template = String::new();
    input.read_to_string(&mut template).unwrap();
    let template = template;

    let main = template.replace("@CRATES@", &crates[..]).replace("@UTIL_MAP@", &util_map[..]);
    match out.write_all(main.as_bytes()) {
        Err(e) => panic!("{}", e),
        _ => (),
    }
}
