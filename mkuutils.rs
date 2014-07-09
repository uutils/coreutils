use std::io::{File, Truncate, Write};
use std::os;
use std::path::Path;

fn main() {
    let args = os::args();
    if args.len() < 3 {
        println!("usage: mkuutils <outfile> <crates>");
        os::set_exit_status(1);
        return;
    }

    let mut crates = String::new();
    let mut util_map = String::new();
    let mut hashsum = false;
    for prog in args.slice_from(2).iter() {
        match prog.as_slice() {
            "md5sum" | "sha1sum" | "sha224sum" | "sha256sum" | "sha384sum" | "sha512sum" => {
                if !hashsum {
                    crates.push_str("extern crate hashsum;\n");
                    hashsum = true;
                }
                util_map.push_str(format!("map.insert(\"{}\", hashsum::uumain);\n", prog).as_slice());
            }
            "true" => util_map.push_str("fn uutrue(_: Vec<String>) -> int { 0 }\nmap.insert(\"true\", uutrue);\n"),
            "false" => util_map.push_str("fn uufalse(_: Vec<String>) -> int { 1 }\nmap.insert(\"false\", uufalse);\n"),
            "sync" => {
                crates.push_str("extern crate uusync;\n");
                util_map.push_str("map.insert(\"sync\", uusync::uumain);\n");
            }
            _ => {
                crates.push_str(format!("extern crate {};\n", prog).as_slice());
                util_map.push_str(format!("map.insert(\"{prog}\", {prog}::uumain);\n", prog = prog).as_slice());
            }
        }
    }
    let outfile = args.get(1).as_slice();

    // XXX: this all just assumes that the IO works correctly
    let mut out = File::open_mode(&Path::new(outfile), Truncate, Write).unwrap();
    let mut input = File::open(&Path::new("uutils/uutils.rs")).unwrap();
    let main = input.read_to_string().unwrap().replace("@CRATES@", crates.as_slice()).replace("@UTIL_MAP@", util_map.as_slice());

    match out.write(main.as_bytes()) {
        Err(e) => fail!("{}", e),
        _ => (),
    }
}
